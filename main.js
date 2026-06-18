// Hand-written entry wrapper over the napi-generated `index.js`.
//
// Its ONLY job is to attach `Symbol.asyncDispose` so `await using` works. NAPI-RS
// emits `Symbol.asyncIterator` but has no codegen for the dispose symbols, and the
// instances are built in Rust and returned from factory methods (`open`,
// `declarePublisher`, `declareSubscriber`) — never `new`'d in JS — so the only way
// to reach them is by patching the shared prototype.
//
// This file is deliberately minimal and must stay that way: it is NOT a home for
// convenience helpers. Anything that can be done in Rust must be done in Rust, so
// the native binding stays the single source of truth and the JS layer never
// becomes a crutch. Add to this file only what genuinely cannot exist in Rust.
import { Publisher, Session, Subscriber } from './index.js'

Session.prototype[Symbol.asyncDispose] = function () {
  return this.close()
}
Publisher.prototype[Symbol.asyncDispose] = function () {
  return this.undeclare()
}
Subscriber.prototype[Symbol.asyncDispose] = function () {
  return this.undeclare()
}

export * from './index.js'
