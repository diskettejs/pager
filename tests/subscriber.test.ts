import { describe, expect, test } from 'vitest'
import { Session } from '../index.js'
import { loopbackConfig } from './loopback.js'

describe('Subscriber round-trip', () => {
  describe('fifo', () => {
    test('stream() yields the put sample, then ends when undeclared', async () => {
      await using session = await Session.open(loopbackConfig())
      const sub = await session.declareSubscriber('dialtone/test/fifo/stream')

      await session.put('dialtone/test/fifo/stream', 'hello')

      // Consume without `break`: after the sample, undeclare the subscription —
      // dropping the channel's sender disconnects it, so the next iteration ends
      // the stream. Reaching the assertion below is itself the proof the loop
      // terminated rather than blocking on a next sample that never arrives.
      const received: string[] = []
      for await (const sample of sub.handler.stream()) {
        expect(sample.keyExpr.asStr).toBe('dialtone/test/fifo/stream')
        received.push(sample.payload.toString())
        await sub.undeclare()
      }

      expect(received).toEqual(['hello'])
    })

    test('recvAsync() resolves the next sample', async () => {
      await using session = await Session.open(loopbackConfig())
      await using sub = await session.declareSubscriber('dialtone/test/fifo/recv')

      await session.put('dialtone/test/fifo/recv', new Uint8Array([1, 2, 3]))

      const sample = await sub.handler.recvAsync()
      expect(Array.from(sample.payload.toBytes())).toEqual([1, 2, 3])
    })

    test('exposes channel introspection', async () => {
      await using session = await Session.open(loopbackConfig())
      await using sub = await session.declareSubscriber('dialtone/test/fifo/introspect')

      // Default channel is a FIFO of 256.
      expect(sub.handler.capacity).toBe(256)
      expect(sub.handler.isEmpty).toBe(true)
      expect(sub.handler.isDisconnected).toBe(false)
    })
  })

  describe('ring', () => {
    test('recvAsync() resolves a sample', async () => {
      await using session = await Session.open(loopbackConfig())
      await using sub = await session.declareSubscriber('dialtone/test/ring/recv', {
        handler: { kind: 'Ring', capacity: 3 },
      })

      await session.put('dialtone/test/ring/recv', 'latest')

      const sample = await sub.handler.recvAsync()
      expect(sample.payload.toString()).toBe('latest')
    })
  })

  test('session exposes zid and isClosed', async () => {
    const session = await Session.open(loopbackConfig())
    expect(typeof session.zid).toBe('string')
    expect(session.zid.length).toBeGreaterThan(0)
    expect(session.isClosed).toBe(false)
    await session.close()
    expect(session.isClosed).toBe(true)
  })
})
