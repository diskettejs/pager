import { describe, expect, test } from 'vitest'
import { Session } from '../index.js'
import { loopbackConfig } from './loopback.js'

describe('Publisher round-trip', () => {
  test('put() reaches a matching subscriber', async () => {
    await using session = await Session.open(loopbackConfig())
    await using sub = await session.declareSubscriber('dialtone/test/pub/put')
    await using pub = await session.declarePublisher('dialtone/test/pub/put')

    await pub.put('hello')

    const sample = await sub.handler.recvAsync()
    expect(sample.keyExpr.asStr).toBe('dialtone/test/pub/put')
    expect(sample.kind).toBe('Put')
    expect(sample.payload.toString()).toBe('hello')
  })

  test('put() carries a Uint8Array payload', async () => {
    await using session = await Session.open(loopbackConfig())
    await using sub = await session.declareSubscriber('dialtone/test/pub/bytes')
    await using pub = await session.declarePublisher('dialtone/test/pub/bytes')

    await pub.put(new Uint8Array([1, 2, 3]))

    const sample = await sub.handler.recvAsync()
    expect(Array.from(sample.payload.toBytes())).toEqual([1, 2, 3])
  })

  test('delete() sends a Delete sample', async () => {
    await using session = await Session.open(loopbackConfig())
    await using sub = await session.declareSubscriber('dialtone/test/pub/del')
    await using pub = await session.declarePublisher('dialtone/test/pub/del')

    await pub.delete()

    const sample = await sub.handler.recvAsync()
    expect(sample.kind).toBe('Delete')
  })

  test('exposes keyExpr, id, and the fixed QoS', async () => {
    await using session = await Session.open(loopbackConfig())
    await using pub = await session.declarePublisher('dialtone/test/pub/meta', {
      priority: 'DataHigh',
      congestionControl: 'Block',
    })

    expect(pub.keyExpr.asStr).toBe('dialtone/test/pub/meta')
    expect(pub.priority).toBe('DataHigh')
    expect(pub.congestionControl).toBe('Block')
    expect(typeof pub.id.zid).toBe('string')
    expect(pub.id.zid.length).toBeGreaterThan(0)
  })
})

// The round-trip tests above publish and subscribe on the *same* key, which only
// shows a sample comes back. These prove Zenoh's key-expression routing is
// actually wired through the binding: a publication is matched to a subscriber
// by ke (not identity), and a non-matching publication is filtered out.
describe('Publisher routing', () => {
  test('routes a put to a wildcard subscriber matched by key expression', async () => {
    await using session = await Session.open(loopbackConfig())
    await using sub = await session.declareSubscriber('dialtone/test/route/**')
    await using pub = await session.declarePublisher('dialtone/test/route/leaf')

    await pub.put('matched')

    const sample = await sub.handler.recvAsync()
    // The subscriber declared a wildcard, yet the sample carries the concrete
    // key the publisher published on — only real ke-routing produces that.
    expect(sample.keyExpr.asStr).toBe('dialtone/test/route/leaf')
    expect(sample.payload.toString()).toBe('matched')
  })

  test('does not deliver a put whose key does not match the subscriber', async () => {
    await using session = await Session.open(loopbackConfig())
    await using sub = await session.declareSubscriber('dialtone/test/route/wanted')
    await using wanted = await session.declarePublisher('dialtone/test/route/wanted')
    await using other = await session.declarePublisher('dialtone/test/route/other')

    // Publish the non-matching key FIRST. If the binding wrongly delivered
    // everything, FIFO order would surface 'other' ahead of 'wanted', so
    // receiving 'wanted' is proof 'other' was filtered by key-expression match.
    await other.put('nope')
    await wanted.put('yes')

    const sample = await sub.handler.recvAsync()
    expect(sample.keyExpr.asStr).toBe('dialtone/test/route/wanted')
    expect(sample.payload.toString()).toBe('yes')
    // Nothing else was queued: the non-matching publication never arrived.
    expect(sub.handler.isEmpty).toBe(true)
  })

  test('routes a delete to a wildcard subscriber matched by key expression', async () => {
    await using session = await Session.open(loopbackConfig())
    await using sub = await session.declareSubscriber('dialtone/test/route/del/**')
    await using pub = await session.declarePublisher('dialtone/test/route/del/leaf')

    await pub.delete()

    const sample = await sub.handler.recvAsync()
    expect(sample.kind).toBe('Delete')
    expect(sample.keyExpr.asStr).toBe('dialtone/test/route/del/leaf')
  })
})
