import { describe, expectTypeOf, test } from 'vitest'
import { Querier, Session } from '../index.js'
import type {
  FifoChannelHandlerReply,
  FifoChannelHandlerSample,
  Liveliness,
  ReplyHandler,
  RingChannelHandlerReply,
  RingChannelHandlerSample,
  SampleHandler,
  Subscriber,
} from '../index.js'

// `.test-d.ts` bodies are type-checked, never executed, so these are only ever
// inspected for their types.
const querier = new Querier()
const session = new Session()

describe('Querier.get narrows the reply handler by channel kind', () => {
  test('no options → FIFO handler (the default)', async () => {
    expectTypeOf(await querier.get()).toEqualTypeOf<FifoChannelHandlerReply>()
  })

  test("kind: 'Fifo' → FIFO handler with the full surface", async () => {
    const replies = await querier.get({ handler: { kind: 'Fifo', capacity: 10 } })
    expectTypeOf(replies).toEqualTypeOf<FifoChannelHandlerReply>()
    // Full surface: async-iterator `stream()` plus introspection.
    expectTypeOf(replies.stream).toBeFunction()
    expectTypeOf(replies).toHaveProperty('isEmpty')
    // TODO: `capacity` could narrow to `number` when a literal capacity is passed.
    expectTypeOf(replies.capacity).toEqualTypeOf<number | null>()
  })

  test("kind: 'Ring' → Ring handler, receive-only", async () => {
    const replies = await querier.get({ handler: { kind: 'Ring', capacity: 10 } })
    expectTypeOf(replies).toEqualTypeOf<RingChannelHandlerReply>()
    // Ring is sparse: no `stream()` / introspection getters.
    expectTypeOf(replies).not.toHaveProperty('stream')
    expectTypeOf(replies).not.toHaveProperty('capacity')
    expectTypeOf(replies.recvAsync).toBeFunction()
  })

  test('non-literal kind → union fallback', async () => {
    const kind = 'Ring' as 'Fifo' | 'Ring'
    expectTypeOf(await querier.get({ handler: { kind } })).toEqualTypeOf<ReplyHandler>()
  })
})

describe('Session.declareSubscriber narrows the subscriber handler', () => {
  test('no options → FIFO subscriber', async () => {
    const sub = await session.declareSubscriber('key/**')
    expectTypeOf(sub).toEqualTypeOf<Subscriber<FifoChannelHandlerSample>>()
    expectTypeOf(sub.handler).toEqualTypeOf<FifoChannelHandlerSample>()
    expectTypeOf(sub.handler.stream).toBeFunction()
  })

  test("kind: 'Ring' → Ring subscriber, receive-only handler", async () => {
    const sub = await session.declareSubscriber('key/**', { handler: { kind: 'Ring' } })
    expectTypeOf(sub).toEqualTypeOf<Subscriber<RingChannelHandlerSample>>()
    expectTypeOf(sub.handler).toEqualTypeOf<RingChannelHandlerSample>()
    expectTypeOf(sub.handler).not.toHaveProperty('stream')
  })

  test('SampleHandler is the union of both channel handlers', () => {
    expectTypeOf<SampleHandler>().toEqualTypeOf<
      FifoChannelHandlerSample | RingChannelHandlerSample
    >()
  })
})

describe('Session facade exposes the querier + liveliness sub-APIs', () => {
  test('declareQuerier → Querier', async () => {
    expectTypeOf(await session.declareQuerier('key/**')).toEqualTypeOf<Querier>()
  })

  test('liveliness() → Liveliness', () => {
    expectTypeOf(session.liveliness()).toEqualTypeOf<Liveliness>()
  })
})
