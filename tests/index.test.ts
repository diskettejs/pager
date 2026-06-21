import { expect, test } from 'vitest'
import { Config, Session } from '../index.js'

test('Config factory methods construct instances', () => {
  expect(Config.default()).toBeInstanceOf(Config)
  expect(Config.fromJson5('{}')).toBeInstanceOf(Config)
})

test('Session opens, exposes a zid, and closes', async () => {
  const session = await Session.open()

  expect(session.isClosed).toBe(false)
  expect(typeof session.zid).toBe('string')
  expect(session.zid.length).toBeGreaterThan(0)

  await session.close()
  expect(session.isClosed).toBe(true)
}, 15_000)

test('Session.open accepts an explicit Config', async () => {
  const session = await Session.open(Config.default())
  await session.close()
}, 15_000)

test('Session.newTimestamp returns an NTP64 time and id', async () => {
  const session = await Session.open()
  const timestamp = session.newTimestamp()

  expect(typeof timestamp.time).toBe('bigint')
  expect(typeof timestamp.id).toBe('string')

  await session.close()
}, 15_000)

test('Session.put and delete resolve, with the full option set', async () => {
  const session = await Session.open()
  const timestamp = session.newTimestamp()

  await session.put('demo/zenoh-ts/value', 'hello')
  await session.put('demo/zenoh-ts/value', Buffer.from([1, 2, 3]), {
    encoding: 'application/octet-stream',
    attachment: 'metadata',
    congestionControl: 'Block',
    priority: 'DataHigh',
    express: true,
    reliability: 'Reliable',
    allowedDestination: 'Any',
    timestamp,
    sourceInfo: { sourceId: { zid: session.zid, eid: 0 }, sourceSn: 0 },
  })
  await session.delete('demo/zenoh-ts/value', { timestamp })

  await session.close()
}, 15_000)

test('Session.info reports the session and peer ids', async () => {
  const session = await Session.open()
  const info = session.info()

  expect(await info.zid()).toBe(session.zid)
  expect(Array.isArray(await info.routersZid())).toBe(true)
  expect(Array.isArray(await info.peersZid())).toBe(true)

  await session.close()
}, 15_000)

test('Session.declarePublisher exposes its config and publishes', async () => {
  const session = await Session.open()
  const publisher = await session.declarePublisher('demo/zenoh-ts/pub', {
    encoding: 'text/plain',
    congestionControl: 'Block',
    priority: 'DataHigh',
    express: true,
    reliability: 'Reliable',
    allowedDestination: 'Any',
  })

  expect(publisher.keyExpr).toBe('demo/zenoh-ts/pub')
  expect(publisher.encoding).toBe('text/plain')
  expect(publisher.congestionControl).toBe('Block')
  expect(publisher.priority).toBe('DataHigh')
  expect(publisher.reliability).toBe('Reliable')
  expect(typeof publisher.id.zid).toBe('string')
  expect(typeof publisher.id.eid).toBe('number')

  await publisher.put('hello')
  await publisher.put(Buffer.from([1, 2, 3]), {
    encoding: 'application/octet-stream',
    attachment: 'meta',
  })
  await publisher.delete()

  const status = await publisher.matchingStatus()
  expect(typeof status.matching).toBe('boolean')

  publisher.undeclare()
  await session.close()
}, 15_000)

test('pub/sub round-trip within a session, via recv', async () => {
  const session = await Session.open()
  const subscriber = await session.declareSubscriber('demo/zenoh-ts/rt')

  await session.put('demo/zenoh-ts/rt', 'hello')

  const sample = await subscriber.recv()
  expect(sample).not.toBeNull()
  expect(sample!.keyExpr).toBe('demo/zenoh-ts/rt')
  expect(sample!.kind).toBe('Put')
  expect(sample!.payload.toString()).toBe('hello')

  subscriber.undeclare()
  await session.close()
}, 15_000)

test('Subscriber is an async iterator', async () => {
  const session = await Session.open()
  const subscriber = await session.declareSubscriber('demo/zenoh-ts/iter')

  await session.put('demo/zenoh-ts/iter', 'a')
  await session.put('demo/zenoh-ts/iter', 'b')

  const received: string[] = []
  for await (const sample of subscriber) {
    received.push(sample.payload.toString())
    if (received.length === 2) break
  }
  expect(received).toEqual(['a', 'b'])

  subscriber.undeclare()
  await session.close()
}, 15_000)

test('Publisher.matchingListener can be declared, polled, and undeclared', async () => {
  const session = await Session.open()
  const publisher = await session.declarePublisher('demo/zenoh-ts/ml')

  const listener = await publisher.matchingListener()
  // Non-blocking poll; may be null before any matching change is observed.
  listener.tryRecv()
  listener.undeclare()

  publisher.undeclare()
  await session.close()
}, 15_000)

test('Subscriber.tryRecv returns null when empty but throws once closed', async () => {
  const session = await Session.open()
  const subscriber = await session.declareSubscriber('demo/zenoh-ts/tryrecv')

  // Connected with nothing published yet: empty, not closed -> null.
  expect(subscriber.tryRecv()).toBeNull()

  // Undeclaring closes the channel; tryRecv must surface that as distinct from
  // "empty" so a polling loop can terminate instead of spinning forever.
  subscriber.undeclare()
  expect(() => subscriber.tryRecv()).toThrow()

  await session.close()
}, 15_000)

test('Subscriber.undeclare drops buffered samples instead of draining them', async () => {
  const session = await Session.open()
  const subscriber = await session.declareSubscriber('demo/zenoh-ts/drop')

  await session.put('demo/zenoh-ts/drop', 'a')
  await session.put('demo/zenoh-ts/drop', 'b')

  // Give both samples time to land in the FIFO buffer (timing-dependent).
  await new Promise((resolve) => setTimeout(resolve, 200))

  const first = await subscriber.recv()
  expect(first!.payload.toString()).toBe('a')

  // 'b' is buffered, but undeclaring drops the handler (and its buffer) just as
  // zenoh does -- so recv resolves to null rather than draining the leftover.
  subscriber.undeclare()
  expect(await subscriber.recv()).toBeNull()

  await session.close()
}, 15_000)

test('declareSubscriber with a Ring channel keeps the latest sample', async () => {
  const session = await Session.open()
  const subscriber = await session.declareSubscriber('demo/zenoh-ts/ring', {
    handler: { kind: 'Ring', capacity: 1 },
  })

  // Two puts into a capacity-1 ring: the oldest ('a') is dropped.
  await session.put('demo/zenoh-ts/ring', 'a')
  await session.put('demo/zenoh-ts/ring', 'b')

  const sample = await subscriber.recv()
  expect(sample!.payload.toString()).toBe('b')

  subscriber.undeclare()
  await session.close()
}, 15_000)

test('get/queryable round-trip surfaces sample and error replies', async () => {
  const session = await Session.open()
  const queryable = await session.declareQueryable('demo/zenoh-ts/q')

  // Answer the one incoming query with a sample reply, then an error reply.
  const serve = (async () => {
    for await (const query of queryable) {
      expect(query.keyExpr).toBe('demo/zenoh-ts/q')
      await query.reply('demo/zenoh-ts/q', 'answer')
      await query.replyErr('boom')
      break
    }
  })()

  const replies = await session.get('demo/zenoh-ts/q')
  const samples: string[] = []
  const errors: string[] = []
  for await (const reply of replies) {
    // Flavor-B discriminant: `reply.sample` truthy => sample reply, else error.
    if (reply.sample) {
      samples.push(reply.sample.payload.toString())
    } else {
      expect(reply.sample).toBeNull()
      errors.push(reply.payload.toString())
    }
    if (samples.length + errors.length === 2) break
  }

  await serve

  expect(samples).toEqual(['answer'])
  expect(errors).toEqual(['boom'])

  queryable.undeclare()
  await session.close()
}, 15_000)

test('Queryable.recv exposes query metadata and get carries a payload', async () => {
  const session = await Session.open()
  const queryable = await session.declareQueryable('demo/zenoh-ts/meta')

  const replies = await session.get('demo/zenoh-ts/meta?arg=1', {
    payload: 'q-payload',
  })

  const query = await queryable.recv()
  expect(query).not.toBeNull()
  expect(query!.keyExpr).toBe('demo/zenoh-ts/meta')
  expect(query!.selector).toContain('arg=1')
  expect(query!.parameters).toContain('arg=1')
  expect(query!.payload?.toString()).toBe('q-payload')
  await query!.reply('demo/zenoh-ts/meta', 'ok')

  const reply = await replies.recv()
  expect(reply).not.toBeNull()
  expect(reply!.sample).not.toBeNull()
  expect(reply!.sample!.payload.toString()).toBe('ok')

  queryable.undeclare()
  await session.close()
}, 15_000)

test('declareQuerier issues gets, round-trips, and exposes its config', async () => {
  const session = await Session.open()
  const queryable = await session.declareQueryable('demo/zenoh-ts/qr')

  const querier = await session.declareQuerier('demo/zenoh-ts/qr', {
    target: 'All',
    consolidation: 'None',
    congestionControl: 'Block',
    priority: 'DataHigh',
  })
  expect(querier.keyExpr).toBe('demo/zenoh-ts/qr')
  expect(querier.congestionControl).toBe('Block')
  expect(querier.priority).toBe('DataHigh')
  expect(typeof querier.id.zid).toBe('string')

  // Answer the querier's one query.
  const serve = (async () => {
    for await (const query of queryable) {
      expect(query.parameters).toContain('k=v')
      await query.reply('demo/zenoh-ts/qr', 'pong')
      break
    }
  })()

  const replies = await querier.get({ parameters: 'k=v' })
  const reply = await replies.recv()
  expect(reply).not.toBeNull()
  expect(reply!.sample).not.toBeNull()
  expect(reply!.sample!.payload.toString()).toBe('pong')

  await serve

  const status = await querier.matchingStatus()
  expect(typeof status.matching).toBe('boolean')

  querier.undeclare()
  queryable.undeclare()
  await session.close()
}, 15_000)
