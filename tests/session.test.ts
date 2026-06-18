import { describe, expect, it } from 'vitest'
import { open } from '../index.js'

describe('session pub/sub loopback', () => {
  it('put → subscriber.receive() yields a mapped Sample', async () => {
    const session = await open()
    const sub = await session.declareSubscriber('demo/test')
    // give the local subscription a tick to be wired up
    await new Promise((r) => setTimeout(r, 200))

    await session.put('demo/test', 'hello')
    const sample = await sub.receive()

    expect(sample).not.toBeNull()
    expect(sample!.keyExpr).toBe('demo/test')
    expect(sample!.kind).toBe('Put')
    expect(new TextDecoder().decode(sample!.payload)).toBe('hello')
    expect(sub.keyExpr).toBe('demo/test')

    await sub.undeclare()
    await session.close()
  }, 10_000)

  it('accepts a Uint8Array payload and exposes it as bytes', async () => {
    const session = await open()
    const sub = await session.declareSubscriber('demo/bytes')
    await new Promise((r) => setTimeout(r, 200))

    await session.put('demo/bytes', new Uint8Array([1, 2, 3]))
    const sample = await sub.receive()

    expect(sample).not.toBeNull()
    expect(Array.from(sample!.payload)).toEqual([1, 2, 3])

    await sub.undeclare()
    await session.close()
  }, 10_000)

  it('for await drains samples and undeclares on break', async () => {
    const session = await open()
    const sub = await session.declareSubscriber('demo/iter')
    await new Promise((r) => setTimeout(r, 200))

    await session.put('demo/iter', 'a')
    for await (const sample of sub) {
      expect(new TextDecoder().decode(sample.payload)).toBe('a')
      break // triggers AsyncGenerator.return() → undeclare
    }

    await session.close()
  }, 10_000)
})
