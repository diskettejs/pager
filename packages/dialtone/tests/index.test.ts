import { expect, test } from 'vitest'
import { Config, KeyExpr, scout, Session } from '../index.js'

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

  expect(publisher.keyExpr.toString()).toBe('demo/zenoh-ts/pub')
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
  expect(sample!.keyExpr.toString()).toBe('demo/zenoh-ts/rt')
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
      expect(query.keyExpr.toString()).toBe('demo/zenoh-ts/q')
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
  expect(query!.keyExpr.toString()).toBe('demo/zenoh-ts/meta')
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
  expect(querier.keyExpr.toString()).toBe('demo/zenoh-ts/qr')
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

test('KeyExpr validates, canonizes, compares, and joins', () => {
  const ke = new KeyExpr('demo/zenoh-ts/**')
  expect(ke.toString()).toBe('demo/zenoh-ts/**')

  // The constructor rejects non-canon input...
  expect(() => new KeyExpr('demo/**/**/x')).toThrow()
  // ...while autocanonize repairs it.
  expect(KeyExpr.autocanonize('demo/**/**/x').toString()).toBe('demo/**/x')

  // Matching accepts either a string or a KeyExpr.
  expect(ke.intersects('demo/zenoh-ts/value')).toBe(true)
  expect(ke.includes('demo/zenoh-ts/value')).toBe(true)
  expect(ke.intersects(new KeyExpr('other/**'))).toBe(false)
  expect(ke.equals(new KeyExpr('demo/zenoh-ts/**'))).toBe(true)

  expect(ke.join('child').toString()).toBe('demo/zenoh-ts/**/child')
})

test('a KeyExpr is accepted anywhere a string key expression is', async () => {
  const session = await Session.open()
  const subscriber = await session.declareSubscriber(new KeyExpr('demo/zenoh-ts/ke'))

  await session.put(new KeyExpr('demo/zenoh-ts/ke'), 'hi')

  const sample = await subscriber.recv()
  expect(sample).not.toBeNull()
  expect(sample!.keyExpr).toBeInstanceOf(KeyExpr)
  expect(sample!.keyExpr.toString()).toBe('demo/zenoh-ts/ke')

  subscriber.undeclare()
  await session.close()
}, 15_000)

test('Liveliness: a token appears as Put and vanishes as Delete to a subscriber', async () => {
  const session = await Session.open()
  const liveliness = session.liveliness()

  const subscriber = await liveliness.declareSubscriber('demo/zenoh-ts/liveliness/**')
  const token = await liveliness.declareToken('demo/zenoh-ts/liveliness/token')

  // The token appearing is delivered as a `Put` for its key expression.
  const appeared = await subscriber.recv()
  expect(appeared).not.toBeNull()
  expect(appeared!.kind).toBe('Put')
  expect(appeared!.keyExpr.toString()).toBe('demo/zenoh-ts/liveliness/token')

  // Undeclaring the token is delivered as a `Delete` for the same key.
  token.undeclare()
  const vanished = await subscriber.recv()
  expect(vanished).not.toBeNull()
  expect(vanished!.kind).toBe('Delete')
  expect(vanished!.keyExpr.toString()).toBe('demo/zenoh-ts/liveliness/token')

  subscriber.undeclare()
  await session.close()
}, 15_000)

test('Liveliness.get reports currently-live tokens', async () => {
  const session = await Session.open()
  const liveliness = session.liveliness()

  const token = await liveliness.declareToken('demo/zenoh-ts/liveliness/get')

  const replies = await liveliness.get('demo/zenoh-ts/liveliness/**', { timeout: 5_000 })
  const keys: string[] = []
  for await (const reply of replies) {
    if (reply.sample) keys.push(reply.sample.keyExpr.toString())
  }
  expect(keys).toContain('demo/zenoh-ts/liveliness/get')

  token.undeclare()
  await session.close()
}, 15_000)

test('scout yields Hellos and stops cleanly', async () => {
  // Open a session so there is a discoverable node on the local network.
  const session = await Session.open()

  const handle = await scout(['Peer', 'Router', 'Client'], Config.default())

  // Discovery is best-effort (multicast may be unavailable in some sandboxes),
  // so race the first Hello against a short timeout and only assert its shape
  // when one actually arrives.
  const hello = await Promise.race([
    handle.recv(),
    new Promise<null>((resolve) => setTimeout(() => resolve(null), 2_000)),
  ])
  if (hello) {
    expect(['Router', 'Peer', 'Client']).toContain(hello.whatami)
    expect(typeof hello.zid).toBe('string')
    expect(Array.isArray(hello.locators)).toBe(true)
  }

  // Stopping closes the channel: tryRecv must then throw, distinct from "empty".
  handle.stop()
  expect(() => handle.tryRecv()).toThrow()

  await session.close()
}, 15_000)

test('scout accepts an empty matcher (scout all) and a Ring handler', async () => {
  const handle = await scout([], Config.default(), { kind: 'Ring', capacity: 4 })

  // Before stopping, a non-blocking poll yields a buffered Hello or null, never throws.
  const first = handle.tryRecv()
  expect(first === null || typeof first === 'object').toBe(true)

  handle.stop()
}, 15_000)
