import { expect, test } from 'vitest'
import { Session } from '../index.js'

// Advanced pub/sub is integrated into the regular Publisher/Subscriber surface:
// every declared publisher/subscriber is an advanced one, so these options live
// directly on `declarePublisher`/`declareSubscriber`.
//
// NOTE: pairing `cache` with `sampleMissDetection` selects sequence-number
// sequencing, which avoids the timestamping-enabled requirement that `cache`
// alone (timestamp sequencing) would impose — keeping these tests config-free.

test('advanced publisher and subscriber declare with the full option set', async () => {
  const session = await Session.open()
  const keyExpr = 'demo/zenoh-ts/adv/declare'

  const publisher = await session.declarePublisher(keyExpr, {
    cache: {
      maxSamples: 5,
      repliesConfig: { priority: 'DataHigh', congestionControl: 'Block', express: true },
    },
    sampleMissDetection: { heartbeat: { periodMs: 500 } },
    publisherDetection: true,
    publisherDetectionMetadata: 'meta/pub',
  })
  expect(publisher.keyExpr.toString()).toBe(keyExpr)

  const subscriber = await session.declareSubscriber(keyExpr, {
    history: { detectLatePublishers: true, maxSamples: 5 },
    recovery: { heartbeat: true },
    subscriberDetection: true,
    subscriberDetectionMetadata: 'meta/sub',
    queryTimeoutMs: 5_000,
  })
  expect(subscriber.keyExpr.toString()).toBe(keyExpr)

  publisher.undeclare()
  subscriber.undeclare()
  await session.close()
}, 15_000)

test('advanced pub/sub live round-trip preserves order', async () => {
  const session = await Session.open()
  const keyExpr = 'demo/zenoh-ts/adv/live'

  // Subscriber first, so the samples below arrive live (not via history).
  const subscriber = await session.declareSubscriber(keyExpr, {
    recovery: { heartbeat: true },
  })
  const publisher = await session.declarePublisher(keyExpr, {
    cache: { maxSamples: 10 },
    sampleMissDetection: { heartbeat: { periodMs: 500 } },
  })

  await publisher.put('a')
  await publisher.put('b')
  await publisher.put('c')

  // The advanced subscriber orders by sequence number; in-order samples are
  // delivered as they arrive.
  const received: string[] = []
  for await (const sample of subscriber) {
    received.push(sample.payload.toString())
    if (received.length === 3) break
  }
  expect(received).toEqual(['a', 'b', 'c'])

  publisher.undeclare()
  subscriber.undeclare()
  await session.close()
}, 15_000)

test('history: a late-joining subscriber replays cached samples', async () => {
  const session = await Session.open()
  const keyExpr = 'demo/zenoh-ts/adv/history'

  // Publish into the cache *before* any subscriber exists.
  const publisher = await session.declarePublisher(keyExpr, {
    cache: { maxSamples: 10 },
    sampleMissDetection: {},
  })
  await publisher.put('one')
  await publisher.put('two')
  await publisher.put('three')

  // Let the cache settle before the late joiner queries it.
  await new Promise((resolve) => setTimeout(resolve, 200))

  // The late joiner queries the publisher's cache on declaration and replays it.
  const subscriber = await session.declareSubscriber(keyExpr, {
    history: { maxSamples: 10 },
  })

  const received: string[] = []
  for await (const sample of subscriber) {
    received.push(sample.payload.toString())
    if (received.length === 3) break
  }
  expect(received).toEqual(['one', 'two', 'three'])

  subscriber.undeclare()
  publisher.undeclare()
  await session.close()
}, 15_000)

test('recovery requires exactly one of heartbeat or periodicQueriesMs', async () => {
  const session = await Session.open()
  const keyExpr = 'demo/zenoh-ts/adv/recovery-validation'

  // Neither mode set: the mutual-exclusion rule (enforced in zenoh-ext at the
  // type level, re-checked here at runtime) rejects.
  await expect(session.declareSubscriber(keyExpr, { recovery: {} })).rejects.toThrow()

  // Both modes set: also rejected.
  await expect(
    session.declareSubscriber(keyExpr, {
      recovery: { heartbeat: true, periodicQueriesMs: 1_000 },
    }),
  ).rejects.toThrow()

  await session.close()
}, 15_000)

test('recovery with periodicQueriesMs declares', async () => {
  const session = await Session.open()
  const keyExpr = 'demo/zenoh-ts/adv/recovery-periodic'

  const subscriber = await session.declareSubscriber(keyExpr, {
    recovery: { periodicQueriesMs: 1_000 },
  })
  expect(subscriber.keyExpr.toString()).toBe(keyExpr)

  subscriber.undeclare()
  await session.close()
}, 15_000)

test('sampleMissListener can be declared, polled, and undeclared', async () => {
  const session = await Session.open()
  const keyExpr = 'demo/zenoh-ts/adv/miss'

  const subscriber = await session.declareSubscriber(keyExpr, {
    recovery: { heartbeat: true },
  })

  const listener = await subscriber.sampleMissListener()
  // No miss has occurred; a non-blocking poll yields null (empty, not closed).
  expect(listener.tryRecv()).toBeNull()
  listener.undeclare()

  subscriber.undeclare()
  await session.close()
}, 15_000)

test('detectPublishers sees a publisher that advertises itself', async () => {
  const session = await Session.open()
  const keyExpr = 'demo/zenoh-ts/adv/detect'

  const subscriber = await session.declareSubscriber(keyExpr)
  const detected = await subscriber.detectPublishers()

  // A publisher that enables publisherDetection shows up as a `Put` (its
  // liveliness token appearing). Declared after `detectPublishers`, so it is
  // observed as a live change.
  const publisher = await session.declarePublisher(keyExpr, {
    publisherDetection: true,
  })

  const appeared = await detected.recv()
  expect(appeared).not.toBeNull()
  expect(appeared!.kind).toBe('Put')

  publisher.undeclare()
  detected.undeclare()
  subscriber.undeclare()
  await session.close()
}, 15_000)

test('sampleMissListener and detectPublishers reject on liveliness subscribers', async () => {
  const session = await Session.open()
  const liveliness = session.liveliness()

  // Liveliness subscribers are plain, not advanced — the advanced-only methods
  // are unavailable and surface a clear error rather than misbehaving.
  const subscriber = await liveliness.declareSubscriber('demo/zenoh-ts/adv/live-sub/**')

  await expect(subscriber.sampleMissListener()).rejects.toThrow()
  await expect(subscriber.detectPublishers()).rejects.toThrow()

  subscriber.undeclare()
  await session.close()
}, 15_000)
