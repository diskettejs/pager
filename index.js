// Hand-written entry wrapper over the napi-generated `binding.js`.
//
// Its ONLY job is to attach the disposal symbols so `await using` works. NAPI-RS
// emits `Symbol.asyncIterator` but has no codegen for the dispose symbols, and
// these instances are built in Rust and returned from factory methods (`open`,
// `declareSubscriber`, …) — never `new`'d in JS — so the only way to reach them
// is by patching the shared prototype. Cleanup is async (`close`/`undeclare`
// resolve over the network), so both are `AsyncDisposable`.
//
// This file is deliberately minimal and must stay that way: it is NOT a home for
// convenience helpers.

import {
  LivelinessSubscriber,
  LivelinessToken,
  MatchingListener,
  Publisher,
  Querier,
  Queryable,
  SampleMissListener,
  Scout,
  Session,
  Subscriber,
} from './binding.js'

Session.prototype[Symbol.asyncDispose] = function () {
  return this.close()
}

Subscriber.prototype[Symbol.asyncDispose] = function () {
  return this.undeclare()
}

Publisher.prototype[Symbol.asyncDispose] = function () {
  return this.undeclare()
}

MatchingListener.prototype[Symbol.asyncDispose] = function () {
  return this.undeclare()
}

SampleMissListener.prototype[Symbol.asyncDispose] = function () {
  return this.undeclare()
}

LivelinessToken.prototype[Symbol.asyncDispose] = function () {
  return this.undeclare()
}

LivelinessSubscriber.prototype[Symbol.asyncDispose] = function () {
  return this.undeclare()
}

Querier.prototype[Symbol.asyncDispose] = function () {
  return this.undeclare()
}

Queryable.prototype[Symbol.asyncDispose] = function () {
  return this.undeclare()
}

// `Scout` is the lone exception: stopping it cancels a local multicast task with
// no network round-trip, so its cleanup is synchronous — `Symbol.dispose`
// (`using`), not `Symbol.asyncDispose` (`await using`).
Scout.prototype[Symbol.dispose] = function () {
  return this.stop()
}

export * from './binding.js'
