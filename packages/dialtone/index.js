// Hand-written entry wrapper over the napi-generated `binding.js`.
//
// Its ONLY job is to attach `Symbol.dispose` so `using` works. NAPI-RS
// emits `Symbol.asyncIterator` but has no codegen for the dispose symbols, and the
// instances are built in Rust and returned from factory methods (`open`,
// `declarePublisher`, `declareSubscriber`) — never `new`'d in JS — so the only way
// to reach them is by patching the shared prototype.
//
// This file is deliberately minimal and must stay that way: it is NOT a home for
// convenience helpers.

import {
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
Publisher.prototype[Symbol.dispose] = function () {
  return this.undeclare()
}
Subscriber.prototype[Symbol.dispose] = function () {
  return this.undeclare()
}
Queryable.prototype[Symbol.dispose] = function () {
  return this.undeclare()
}
Querier.prototype[Symbol.dispose] = function () {
  return this.undeclare()
}
MatchingListener.prototype[Symbol.dispose] = function () {
  return this.undeclare()
}
SampleMissListener.prototype[Symbol.dispose] = function () {
  return this.undeclare()
}
LivelinessToken.prototype[Symbol.dispose] = function () {
  return this.undeclare()
}
Scout.prototype[Symbol.dispose] = function () {
  this.stop()
}

export * from './binding.js'
