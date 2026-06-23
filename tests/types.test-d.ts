import { describe, expectTypeOf, test } from 'vitest'
import { KeyExpr, Querier, Selector, Session } from '../index.js'
import type {
  ChannelKind,
  FifoChannelHandlerReply,
  FifoChannelHandlerSample,
  Liveliness,
  ReplyHandler,
  RingChannelHandlerReply,
  RingChannelHandlerSample,
  SampleHandler,
  Subscriber,
} from '../index.js'

// These assert only what the facade overloads narrow to — the channel-handler
// *type* a call resolves to. The handler surface itself (does `stream()` exist,
// does ring omit it) follows from that type and is fixed by the generated
// declarations, so re-checking it here would test nothing the `toEqualTypeOf`
// already guarantees. Method *behavior* belongs in executed runtime tests, since
// `.test-d.ts` bodies are only type-checked, never run.
const querier = new Querier()
const session = new Session()

describe('Querier.get narrows the reply handler by channel kind', () => {
  test('no options → FIFO handler (the default)', async () => {
    expectTypeOf(await querier.get()).toEqualTypeOf<FifoChannelHandlerReply>()
  })

  test("kind: 'Fifo' → FIFO handler", async () => {
    expectTypeOf(await querier.get({ handler: { kind: 'Fifo', capacity: 10 } }))
      .toEqualTypeOf<FifoChannelHandlerReply>()
  })

  test("kind: 'Ring' → Ring handler", async () => {
    expectTypeOf(await querier.get({ handler: { kind: 'Ring', capacity: 10 } }))
      .toEqualTypeOf<RingChannelHandlerReply>()
  })

  test('non-literal kind → union fallback', async () => {
    const kind = 'Ring' as ChannelKind
    expectTypeOf(await querier.get({ handler: { kind } })).toEqualTypeOf<ReplyHandler>()
  })
})

describe('Liveliness.get narrows the reply handler by channel kind', () => {
  const liveliness = session.liveliness()

  test('no options → FIFO handler (the default)', async () => {
    expectTypeOf(await liveliness.get('key/**')).toEqualTypeOf<FifoChannelHandlerReply>()
  })

  test("kind: 'Fifo' → FIFO handler", async () => {
    expectTypeOf(await liveliness.get('key/**', { handler: { kind: 'Fifo', capacity: 10 } }))
      .toEqualTypeOf<FifoChannelHandlerReply>()
  })

  test("kind: 'Ring' → Ring handler", async () => {
    expectTypeOf(await liveliness.get('key/**', { handler: { kind: 'Ring', capacity: 10 } }))
      .toEqualTypeOf<RingChannelHandlerReply>()
  })

  test('non-literal kind → union fallback', async () => {
    const kind = 'Ring' as ChannelKind
    expectTypeOf(await liveliness.get('key/**', { handler: { kind } })).toEqualTypeOf<ReplyHandler>()
  })
})

describe('Session.get narrows the reply handler by channel kind', () => {
  test('no options → FIFO handler (the default)', async () => {
    expectTypeOf(await session.get('key/**')).toEqualTypeOf<FifoChannelHandlerReply>()
  })

  test("kind: 'Fifo' → FIFO handler", async () => {
    expectTypeOf(await session.get('key/**', { handler: { kind: 'Fifo', capacity: 10 } }))
      .toEqualTypeOf<FifoChannelHandlerReply>()
  })

  test("kind: 'Ring' → Ring handler", async () => {
    expectTypeOf(await session.get('key/**', { handler: { kind: 'Ring', capacity: 10 } }))
      .toEqualTypeOf<RingChannelHandlerReply>()
  })

  test('non-literal kind → union fallback', async () => {
    const kind = 'Ring' as ChannelKind
    expectTypeOf(await session.get('key/**', { handler: { kind } })).toEqualTypeOf<ReplyHandler>()
  })

  test('selector accepts a string, KeyExpr, or Selector', async () => {
    // Unlike Querier/Liveliness, the key expression is per-call: a `key?p=1`
    // string, a `KeyExpr` (no parameters), or a full `Selector` are all valid.
    expectTypeOf(await session.get('key/**?p=1')).toEqualTypeOf<FifoChannelHandlerReply>()
    expectTypeOf(await session.get(new KeyExpr('key/x'))).toEqualTypeOf<FifoChannelHandlerReply>()
    expectTypeOf(await session.get(new Selector('key/x', 'p=1')))
      .toEqualTypeOf<FifoChannelHandlerReply>()
  })
})

describe('Session.declareSubscriber narrows the subscriber by channel kind', () => {
  test('no options → FIFO subscriber', async () => {
    const sub = await session.declareSubscriber('key/**')
    expectTypeOf(sub).toEqualTypeOf<Subscriber<FifoChannelHandlerSample>>()
    expectTypeOf(sub.handler).toEqualTypeOf<FifoChannelHandlerSample>()
  })

  test("kind: 'Ring' → Ring subscriber", async () => {
    const sub = await session.declareSubscriber('key/**', { handler: { kind: 'Ring' } })
    expectTypeOf(sub).toEqualTypeOf<Subscriber<RingChannelHandlerSample>>()
    expectTypeOf(sub.handler).toEqualTypeOf<RingChannelHandlerSample>()
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
