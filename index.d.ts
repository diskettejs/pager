// Types for the entry wrapper (`index.js`). Re-exports the generated surface and
// layers on the things NAPI-RS can't codegen: the `[Symbol.asyncDispose]` members
// `index.js` patches onto the prototypes, and the generic/overload narrowing of
// the channel-handler surface.
//
// Rather than recreate entities member-for-member, the facade classes below
// `extends` the generated classes (imported as `bindings.*` from `./binding.js`)
// and declare ONLY what differs — `[Symbol.asyncDispose]` plus the members that
// narrow by channel kind (`Subscriber.handler`, `Querier.get`,
// `Session.declareSubscriber`). Everything else is inherited, so a newly
// generated method needs no facade upkeep. A local `export declare class` of the
// same name takes precedence over the `export *` re-export, so these replace the
// generated `Subscriber` / `Querier` / `Session` types while the runtime values
// still come from `binding.js`. The base must be `./binding.js` (the generated
// classes), not `./index.js`, which would be self-referential.
//
// Dispose-only entities (`Publisher`, `MatchingListener`, …) need no narrowing,
// so they keep their generated type and gain just the dispose member via
// `declare module './binding.js'` augmentation.
export * from './binding.js';

import * as bindings from './binding.js';
import type {
  Config,
  FifoChannelHandlerReply,
  FifoChannelHandlerSample,
  GetOptions,
  KeyExpr,
  LivelinessGetOptions,
  QuerierGetOptions,
  QuerierOptions,
  RingChannelHandlerReply,
  RingChannelHandlerSample,
  Selector,
  SubscriberOptions,
} from './binding.js';

/** Anywhere a key expression is accepted as input. */
export type KeyExprArg = string | KeyExpr;

/** Anywhere a selector (key expression + optional parameters) is accepted as input. */
export type SelectorArg = string | KeyExpr | Selector;

/** The channel handler a `Subscriber` exposes, by channel kind. */
export type SampleHandler = FifoChannelHandlerSample | RingChannelHandlerSample;

/** The reply handler a `get` resolves to, by channel kind. */
export type ReplyHandler = FifoChannelHandlerReply | RingChannelHandlerReply;

/**
 * Narrows an options bag's `handler` to the default FIFO channel. `handler` stays
 * optional because FIFO is the default when no channel is chosen.
 */
type FifoOptions<O> = Omit<O, 'handler'> & {
  handler?: { kind: 'Fifo'; capacity?: number };
};

/**
 * Narrows an options bag's `handler` to the ring channel. `handler` is required
 * because the ring channel must be explicitly selected.
 */
type RingOptions<O> = Omit<O, 'handler'> & {
  handler: { kind: 'Ring'; capacity?: number };
};

/**
 * A live subscription whose `handler` narrows to the channel chosen at declare
 * time (defaults to the union). Inherits `keyExpr` / `id` / `undeclare` and
 * `detectPublishers` / `sampleMissListener` from the generated `Subscriber`.
 */
export declare class Subscriber<
  H extends SampleHandler = SampleHandler,
> extends bindings.Subscriber {
  /** The receive end of the subscription, narrowed to the chosen channel. */
  get handler(): H;
  /** Async-disposes by undeclaring the subscription (`await using`). */
  [Symbol.asyncDispose](): Promise<void>;
}

/**
 * A declared querier whose `get` narrows the reply handler to the channel chosen
 * via the `handler` option (mirroring zenoh, where `replies` is the handler).
 * Inherits `matchingListener` / `matchingStatus` / `undeclare` and the config
 * getters from the generated `Querier`.
 */
export declare class Querier extends bindings.Querier {
  /** FIFO (default): the reply handler has the full receive + introspection + `stream()` surface. */
  get(options?: FifoOptions<QuerierGetOptions> | null): Promise<FifoChannelHandlerReply>;
  /** Ring: the reply handler exposes only the receive variants. */
  get(options: RingOptions<QuerierGetOptions>): Promise<RingChannelHandlerReply>;
  /** Fallback when the channel `kind` isn't a literal. */
  get(options?: QuerierGetOptions | null): Promise<ReplyHandler>;
  /** Async-disposes by undeclaring the querier (`await using`). */
  [Symbol.asyncDispose](): Promise<void>;
}

/**
 * The liveliness sub-API of a `Session`, reached via `Session.liveliness()`.
 * Its `get` narrows the reply handler to the channel chosen via the `handler`
 * option (mirroring `Querier.get`, where `replies` is the handler). Inherits
 * `declareToken` / `declareSubscriber` from the generated `Liveliness`.
 *
 * Unlike the declared entities, `Liveliness` is a borrow-free handle over the
 * session, so it has no `undeclare` / `[Symbol.asyncDispose]`.
 */
export declare class Liveliness extends bindings.Liveliness {
  /** FIFO (default): the reply handler has the full receive + introspection + `stream()` surface. */
  get(
    keyExpr: KeyExprArg,
    options?: FifoOptions<LivelinessGetOptions> | null,
  ): Promise<FifoChannelHandlerReply>;
  /** Ring: the reply handler exposes only the receive variants. */
  get(
    keyExpr: KeyExprArg,
    options: RingOptions<LivelinessGetOptions>,
  ): Promise<RingChannelHandlerReply>;
  /** Fallback when the channel `kind` isn't a literal. */
  get(
    keyExpr: KeyExprArg,
    options?: LivelinessGetOptions | null,
  ): Promise<ReplyHandler>;
}

/**
 * A session whose `declareSubscriber` / `get` narrow by channel kind and whose
 * `open` / `declareQuerier` / `liveliness` yield these narrowing facades.
 * Inherits `put` / `close` / `declarePublisher` / `zid` / `isClosed` from the
 * generated `Session`.
 *
 * `open` / `declareQuerier` / `liveliness` are overridden only to return the
 * facade types — otherwise a session/querier/liveliness obtained from them would
 * be the un-narrowed generated class.
 */
export declare class Session extends bindings.Session {
  /** Opens a session with the given configuration. */
  static open(config: Config): Promise<Session>;
  /** FIFO (default): the handler has the full receive + introspection + `stream()` surface. */
  declareSubscriber(
    keyExpr: KeyExprArg,
    options?: FifoOptions<SubscriberOptions> | null,
  ): Promise<Subscriber<FifoChannelHandlerSample>>;
  /** Ring: the handler exposes only the receive variants. */
  declareSubscriber(
    keyExpr: KeyExprArg,
    options: RingOptions<SubscriberOptions>,
  ): Promise<Subscriber<RingChannelHandlerSample>>;
  /** Fallback when the channel `kind` isn't a literal. */
  declareSubscriber(
    keyExpr: KeyExprArg,
    options?: SubscriberOptions | null,
  ): Promise<Subscriber>;
  /** Declares a querier on `keyExpr`, fixing its config for every `get`. */
  declareQuerier(
    keyExpr: KeyExprArg,
    options?: QuerierOptions | null,
  ): Promise<Querier>;
  /** The liveliness sub-API, scoped to this session. */
  liveliness(): Liveliness;
  /** FIFO (default): the reply handler has the full receive + introspection + `stream()` surface. */
  get(
    selector: SelectorArg,
    options?: FifoOptions<GetOptions> | null,
  ): Promise<FifoChannelHandlerReply>;
  /** Ring: the reply handler exposes only the receive variants. */
  get(selector: SelectorArg, options: RingOptions<GetOptions>): Promise<RingChannelHandlerReply>;
  /** Fallback when the channel `kind` isn't a literal. */
  get(selector: SelectorArg, options?: GetOptions | null): Promise<ReplyHandler>;
  /** Async-disposes by closing the session (`await using`). */
  [Symbol.asyncDispose](): Promise<void>;
}

declare module './binding.js' {
  interface Publisher {
    /** Async-disposes by undeclaring the publisher (`await using`). */
    [Symbol.asyncDispose](): Promise<void>;
  }
  interface MatchingListener {
    /** Async-disposes by undeclaring the matching listener (`await using`). */
    [Symbol.asyncDispose](): Promise<void>;
  }
  interface SampleMissListener {
    /** Async-disposes by undeclaring the sample-miss listener (`await using`). */
    [Symbol.asyncDispose](): Promise<void>;
  }
  interface LivelinessToken {
    /** Async-disposes by undeclaring the liveliness token (`await using`). */
    [Symbol.asyncDispose](): Promise<void>;
  }
  interface LivelinessSubscriber {
    /** Async-disposes by undeclaring the liveliness subscriber (`await using`). */
    [Symbol.asyncDispose](): Promise<void>;
  }
}
