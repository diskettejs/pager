// Types for the entry wrapper (`index.js`). Re-exports the generated surface and
// merges in the disposal members that `index.js` attaches at runtime. This
// augmentation is the one thing that can't be expressed in Rust — NAPI-RS has no
// codegen for the dispose symbols.
//
// Cleanup is async — `Session.close()` and `Subscriber.undeclare()` resolve over
// the network — so both are `AsyncDisposable` (`await using`), with
// `[Symbol.asyncDispose]` declared on the facade classes below and patched onto
// the prototypes in `index.js`.
//
// It also narrows the channel-handler surface. NAPI-RS can't express generics,
// so the Rust `Subscriber.handler` getter returns the raw union
// `FifoChannelHandlerSample | RingChannelHandlerSample`. The facade below makes
// `Subscriber` generic over its handler and overloads `declareSubscriber` so the
// channel `kind` picks the concrete handler — `for await (… of sub.handler.stream())`
// type-checks for a FIFO subscriber, etc. A local `export declare class` of the
// same name takes precedence over the `export *` re-export (no conflict), so
// these replace the generated `Subscriber` / `Session` types while the runtime
// values still come from `binding.js`.
export * from './binding.js';

import type {
  Config,
  EntityGlobalId,
  FifoChannelHandlerSample,
  KeyExpr,
  Publisher,
  PublisherOptions,
  PutOptions,
  RingChannelHandlerSample,
  SubscriberOptions,
} from './binding.js';

/** Anywhere a key expression is accepted as input. */
export type KeyExprArg = string | KeyExpr;

type FifoSubscriberOptions = Omit<SubscriberOptions, 'handler'> & {
  handler?: { kind: 'Fifo'; capacity?: number };
};

type RingSubscriberOptions = Omit<SubscriberOptions, 'handler'> & {
  handler: { kind: 'Ring'; capacity?: number };
};

/**
 * A live subscription whose `handler` narrows to the channel chosen at declare
 * time. Defaults to the union when the kind isn't statically known.
 */
export declare class Subscriber<H extends SampleHandler = SampleHandler> {
  /** The key expression this subscription matches. */
  get keyExpr(): KeyExpr;
  /** The global id of this subscription entity. */
  get id(): EntityGlobalId;
  /**
   * The receive end of the subscription. The handler is not iterable; iterate
   * via `subscriber.handler.stream()`.
   */
  get handler(): H;
  /** Undeclare this subscription. Resolves once undeclaration completes. */
  undeclare(): Promise<void>;
  /** Async-disposes by undeclaring the subscription (`await using`). */
  [Symbol.asyncDispose](): Promise<void>;
}

export declare class Session {
  /** Opens a session with the given configuration. */
  static open(config: Config): Promise<Session>;
  /** This session's Zenoh id, as a hex string. */
  get zid(): string;
  /** Whether the session has been closed. */
  get isClosed(): boolean;
  /** Closes the session, undeclaring everything declared on it. */
  close(): Promise<void>;
  /** Publishes `payload` on `keyExpr`. */
  put(
    keyExpr: KeyExprArg,
    payload: string | Uint8Array,
    options?: PutOptions | null,
  ): Promise<void>;
  /** FIFO (default): the handler has the full receive + introspection + `stream()` surface. */
  declareSubscriber(
    keyExpr: KeyExprArg,
    options?: FifoSubscriberOptions | null,
  ): Promise<Subscriber<FifoChannelHandlerSample>>;
  /** Ring: the handler exposes only the receive variants. */
  declareSubscriber(
    keyExpr: KeyExprArg,
    options: RingSubscriberOptions,
  ): Promise<Subscriber<RingChannelHandlerSample>>;
  /** Fallback when the channel `kind` isn't a literal. */
  declareSubscriber(
    keyExpr: KeyExprArg,
    options?: SubscriberOptions | null,
  ): Promise<Subscriber>;
  /** Declares a publisher with fixed QoS on `keyExpr`. */
  declarePublisher(
    keyExpr: KeyExprArg,
    options?: PublisherOptions | null,
  ): Promise<Publisher>;
  /** Async-disposes by closing the session (`await using`). */
  [Symbol.asyncDispose](): Promise<void>;
}

// `Publisher` needs no generic narrowing (it has no handler), so rather than a
// shadowing facade class it keeps its generated type and gains only the dispose
// member, merged in by augmenting the binding module's class.
declare module './binding.js' {
  interface Publisher {
    /** Async-disposes by undeclaring the publisher (`await using`). */
    [Symbol.asyncDispose](): Promise<void>;
  }
}

// Future entities (Queryable / Querier / MatchingListener / SampleMissListener /
// LivelinessToken / Scout) get `[Symbol.asyncDispose]` the same way as they
// land: declare it on the entity (a facade class here, or via
// `declare module './binding.js'` augmentation for non-shadowed types like
// `Publisher` above) and patch the prototype in `index.js`.
