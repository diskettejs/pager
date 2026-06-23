use napi_derive::napi;

// Imported so payload types can be passed to the handler macros as bare
// identifiers. A `:ty` macro fragment is interpolated inside an invisible
// `Group`, which defeats napi-rs's `if let Type::Path(_)` guard when it extracts
// the `AsyncGenerator::Yield` type (the `[Symbol.asyncIterator]` TS signature is
// then dropped). An `:ident` is not group-wrapped, so `type Yield = Sample;`
// stays a bare `Type::Path` and the signature is generated.
use crate::matching_status::MatchingStatus;
use crate::miss::Miss;
use crate::reply::Reply;
use crate::sample::Sample;

/// Which channel backs a subscription's handler.
#[napi(string_enum)]
pub enum ChannelKind {
  /// Bounded FIFO: back-pressures the network when full (drops nothing).
  Fifo,
  /// Bounded ring: keeps the most recent `capacity` samples, dropping oldest.
  Ring,
}

/// Channel selection for a declare call's `handler` field.
///
/// `capacity` defaults to [`DEFAULT_CHANNEL_CAPACITY`] when omitted.
#[napi(object, object_to_js = false)]
pub struct ChannelConfig {
  pub kind: ChannelKind,
  pub capacity: Option<u32>,
}

/// Default channel bound when `capacity` is not given (matches zenoh's
/// `DefaultHandler`, a FIFO of 256).
pub const DEFAULT_CHANNEL_CAPACITY: usize = 256;

/// Generates a concrete FIFO handler class (and its async-iterator companion)
/// for one payload type.
///
/// - `$name`     — the handler class name (e.g. `FifoChannelHandlerSample`)
/// - `$stream`   — the async-iterator class returned by `stream()`
/// - `$napi`     — the napi payload class yielded to JS, as a bare in-scope
///   identifier (e.g. `Sample`) — see the import note above
/// - `$zty`      — the zenoh payload carried by the channel (e.g. `zenoh::sample::Sample`)
/// - `$wrap`     — a path mapping `$zty -> $napi` (e.g. `Sample::new`)
macro_rules! fifo_channel_handler {
  ($name:ident, $stream:ident, $napi:ident, $zty:ty, $wrap:path) => {
    #[napi]
    pub struct $name {
      inner: ::zenoh::handlers::FifoChannelHandler<$zty>,
    }

    impl $name {
      pub(crate) fn from_handler(inner: ::zenoh::handlers::FifoChannelHandler<$zty>) -> Self {
        Self { inner }
      }
    }

    #[napi]
    impl $name {
      /// Receives the next value, resolving when one is available. Rejects once
      /// the channel is disconnected (the producer has been dropped).
      #[napi]
      pub async fn recv_async(&self) -> napi::Result<$napi> {
        // Clone the (cheap) flume handler so nothing borrows `&self` across the
        // await; the clone shares the same underlying channel.
        let handler = self.inner.clone();
        let value = handler
          .recv_async()
          .await
          .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok($wrap(value))
      }

      /// Receives a value without blocking, returning `null` if the channel is
      /// currently empty.
      #[napi]
      pub fn try_recv(&self) -> napi::Result<Option<$napi>> {
        self
          .inner
          .try_recv()
          .map(|opt| opt.map($wrap))
          .map_err(|e| napi::Error::from_reason(e.to_string()))
      }

      /// Returns an async-iterator object over the channel, for use with
      /// `for await`. The handler itself is not iterable; iteration lives here.
      #[napi]
      pub fn stream(&self) -> $stream {
        $stream {
          inner: self.inner.clone(),
        }
      }

      /// The number of values currently queued.
      #[napi(getter)]
      pub fn len(&self) -> u32 {
        self.inner.len() as u32
      }

      /// The channel's bound, or `null` if unbounded.
      #[napi(getter)]
      pub fn capacity(&self) -> Option<u32> {
        self.inner.capacity().map(|c| c as u32)
      }

      /// Whether the channel currently holds no values.
      #[napi(getter)]
      pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
      }

      /// Whether the channel is currently at capacity.
      #[napi(getter)]
      pub fn is_full(&self) -> bool {
        self.inner.is_full()
      }

      /// The number of senders feeding this channel.
      #[napi(getter)]
      pub fn sender_count(&self) -> u32 {
        self.inner.sender_count() as u32
      }

      /// The number of receivers sharing this channel.
      #[napi(getter)]
      pub fn receiver_count(&self) -> u32 {
        self.inner.receiver_count() as u32
      }

      /// Whether the channel has been disconnected (all senders dropped).
      #[napi(getter)]
      pub fn is_disconnected(&self) -> bool {
        self.inner.is_disconnected()
      }

      /// Whether `other` is a handle to the same underlying channel.
      #[napi]
      pub fn same_channel(&self, other: &$name) -> bool {
        self.inner.same_channel(&other.inner)
      }
    }

    #[napi(async_iterator)]
    pub struct $stream {
      inner: ::zenoh::handlers::FifoChannelHandler<$zty>,
    }

    #[napi]
    impl napi::bindgen_prelude::AsyncGenerator for $stream {
      type Yield = $napi;
      type Next = ();
      type Return = ();

      fn next(
        &mut self,
        _value: Option<Self::Next>,
      ) -> impl std::future::Future<Output = napi::Result<Option<Self::Yield>>> + Send + 'static {
        // The future must be `'static`, so clone the handler into it rather than
        // borrowing `&self`. A disconnected channel ends the iteration.
        let handler = self.inner.clone();
        async move {
          match handler.recv_async().await {
            Ok(value) => Ok(Some($wrap(value))),
            Err(_) => Ok(None),
          }
        }
      }
    }
  };
}

/// A producer that owns a ring channel handler for payload `T`.
///
/// A `RingChannelHandler` is a `Weak`, so the producer that registered the
/// channel (e.g. an `AdvancedSubscriber`, plain `Subscriber`, `MatchingListener`
/// or `SampleMissListener`) must be kept alive for the handle to stay upgradable.
/// The napi ring handler therefore holds the producer type-erased as
/// `Arc<dyn RingSource<T>>` — one concrete handler class per payload, regardless
/// of which producer minted it (a single `Sample` ring handler serves both the
/// advanced and the liveliness subscriber). Every such producer `Deref`s to its
/// handler, so each impl is a uniform deref coercion (see [`impl_ring_source`]).
pub(crate) trait RingSource<T>: Send + Sync {
  fn ring(&self) -> &::zenoh::handlers::RingChannelHandler<T>;
}

/// Implements [`RingSource`] for a producer that `Deref`s to its ring handler.
macro_rules! impl_ring_source {
  ($producer:ty, $zty:ty) => {
    impl RingSource<$zty> for $producer {
      fn ring(&self) -> &::zenoh::handlers::RingChannelHandler<$zty> {
        // Deref coercion: `&$producer -> &RingChannelHandler<$zty>`.
        self
      }
    }
  };
}

/// Generates a concrete Ring handler class for one payload type.
///
/// The ring handler is sparse (receive variants only — no `stream`, no
/// introspection). It is producer-agnostic: it holds an `Arc<dyn RingSource<$zty>>`
/// and reaches the channel via [`RingSource::ring`]. Pair it with one
/// [`impl_ring_source`] per producer that can mint this payload.
///
/// - `$name`  — the handler class name (e.g. `RingChannelHandlerSample`)
/// - `$napi`  — the napi payload class yielded to JS, as a bare in-scope identifier
/// - `$zty`   — the zenoh payload carried by the channel
/// - `$wrap`  — a path mapping the channel payload `-> $napi`
macro_rules! ring_channel_handler {
  ($name:ident, $napi:ident, $zty:ty, $wrap:path) => {
    #[napi]
    pub struct $name {
      source: std::sync::Arc<dyn RingSource<$zty>>,
    }

    impl $name {
      pub(crate) fn from_arc<P: RingSource<$zty> + 'static>(source: std::sync::Arc<P>) -> Self {
        // `Arc<P>` unsizes to `Arc<dyn RingSource<$zty>>`.
        Self { source }
      }
    }

    #[napi]
    impl $name {
      /// Receives the next value, resolving when one is available. Rejects once
      /// the producer is gone (the ring's strong owner has been dropped).
      #[napi]
      pub async fn recv_async(&self) -> napi::Result<$napi> {
        // Clone the `Arc` so the future owns a strong ref to the producer
        // (keeping the ring's channel alive) without borrowing `&self` across
        // the await.
        let source = std::sync::Arc::clone(&self.source);
        let value = source
          .ring()
          .recv_async()
          .await
          .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok($wrap(value))
      }

      /// Receives a value without blocking, returning `null` if the ring is
      /// currently empty.
      #[napi]
      pub fn try_recv(&self) -> napi::Result<Option<$napi>> {
        self
          .source
          .ring()
          .try_recv()
          .map(|opt| opt.map($wrap))
          .map_err(|e| napi::Error::from_reason(e.to_string()))
      }
    }
  };
}

fifo_channel_handler!(
  FifoChannelHandlerSample,
  SampleStream,
  Sample,
  zenoh::sample::Sample,
  Sample::new
);

// `Sample` ring handler — minted by both the advanced subscriber and the
// (plain) liveliness subscriber, so it has two `RingSource` producers.
ring_channel_handler!(
  RingChannelHandlerSample,
  Sample,
  zenoh::sample::Sample,
  Sample::new
);
impl_ring_source!(
  zenoh_ext::AdvancedSubscriber<zenoh::handlers::RingChannelHandler<zenoh::sample::Sample>>,
  zenoh::sample::Sample
);
impl_ring_source!(
  zenoh::pubsub::Subscriber<zenoh::handlers::RingChannelHandler<zenoh::sample::Sample>>,
  zenoh::sample::Sample
);

fifo_channel_handler!(
  FifoChannelHandlerMatchingStatus,
  MatchingStatusStream,
  MatchingStatus,
  zenoh::matching::MatchingStatus,
  MatchingStatus::from_inner
);

ring_channel_handler!(
  RingChannelHandlerMatchingStatus,
  MatchingStatus,
  zenoh::matching::MatchingStatus,
  MatchingStatus::from_inner
);
impl_ring_source!(
  zenoh::matching::MatchingListener<
    zenoh::handlers::RingChannelHandler<zenoh::matching::MatchingStatus>,
  >,
  zenoh::matching::MatchingStatus
);

fifo_channel_handler!(
  FifoChannelHandlerMiss,
  MissStream,
  Miss,
  zenoh_ext::Miss,
  Miss::from_inner
);

ring_channel_handler!(
  RingChannelHandlerMiss,
  Miss,
  zenoh_ext::Miss,
  Miss::from_inner
);
impl_ring_source!(
  zenoh_ext::SampleMissListener<zenoh::handlers::RingChannelHandler<zenoh_ext::Miss>>,
  zenoh_ext::Miss
);

// `Reply` handler — produced by `get` (`Querier`/`Session`/`Liveliness`). A get
// resolves directly to its handler, so there's no producer entity to hold; the
// resolved `RingChannelHandler<Reply>` is its own `RingSource` (the channel is
// kept alive by zenoh's background query task, not by us).
fifo_channel_handler!(
  FifoChannelHandlerReply,
  ReplyStream,
  Reply,
  zenoh::query::Reply,
  Reply::from_inner
);

ring_channel_handler!(
  RingChannelHandlerReply,
  Reply,
  zenoh::query::Reply,
  Reply::from_inner
);
impl_ring_source!(
  zenoh::handlers::RingChannelHandler<zenoh::query::Reply>,
  zenoh::query::Reply
);
