import { describe, expect, it } from 'vitest'
import { open } from '../main.js'

const settle = (ms = 200) => new Promise((r) => setTimeout(r, ms))

describe('asyncDispose (await using)', () => {
  it('disposes a full session/publisher/subscriber stack on scope exit', async () => {
    await using session = await open()
    await using sub = await session.declareSubscriber('demo/dispose')
    await using pub = await session.declarePublisher('demo/dispose')
    await settle()

    expect(typeof session[Symbol.asyncDispose]).toBe('function')
    expect(typeof sub[Symbol.asyncDispose]).toBe('function')
    expect(typeof pub[Symbol.asyncDispose]).toBe('function')

    await pub.put('bye')
    const sample = await sub.receive()
    expect(new TextDecoder().decode(sample!.payload)).toBe('bye')
    // scope exit: pub.undeclare() → sub.undeclare() → session.close() (LIFO)
  }, 10_000)

  it('await using actually closes the session (ops reject afterwards)', async () => {
    // The `await using` inside the IIFE is disposed (awaited) before the returned
    // promise resolves, so `disposed` is an already-closed session.
    const disposed = await (async () => {
      await using session = await open()
      return session
    })()

    await expect(disposed.put('demo/after', 'x')).rejects.toThrow()
  }, 10_000)
})
