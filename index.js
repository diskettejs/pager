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

import { Publisher, Session, Subscriber } from './binding.js'

Session.prototype[Symbol.asyncDispose] = function () {
  return this.close()
}

Subscriber.prototype[Symbol.asyncDispose] = function () {
  return this.undeclare()
}

Publisher.prototype[Symbol.asyncDispose] = function () {
  return this.undeclare()
}

// Future entities (Queryable / Querier / MatchingListener / SampleMissListener /
// LivelinessToken / Scout) follow the same pattern as they land — async cleanup
// → `Symbol.asyncDispose`.

export * from './binding.js'
