import { describe, expect, it } from 'vitest'
import { open } from '../index.js'

const settle = () => new Promise((r) => setTimeout(r, 200))

describe('publisher', () => {
  it('declarePublisher → put → subscriber receives', async () => {
    const session = await open()
    const sub = await session.declareSubscriber('demo/pub')
    const pub = await session.declarePublisher('demo/pub')
    await settle()

    await pub.put('hi')
    const s = await sub.receive()

    expect(pub.keyExpr).toBe('demo/pub')
    expect(s!.keyExpr).toBe('demo/pub')
    expect(new TextDecoder().decode(s!.payload)).toBe('hi')

    await pub.undeclare()
    await sub.undeclare()
    await session.close()
  }, 10_000)

  it('put options set encoding and attachment', async () => {
    const session = await open()
    const sub = await session.declareSubscriber('demo/pubopts')
    const pub = await session.declarePublisher('demo/pubopts')
    await settle()

    await pub.put('hi', { encoding: 'text/plain', attachment: 'meta' })
    const s = await sub.receive()

    expect(s!.encoding).toBe('text/plain')
    expect(s!.attachment).toBeDefined()
    expect(new TextDecoder().decode(s!.attachment!)).toBe('meta')

    await pub.undeclare()
    await sub.undeclare()
    await session.close()
  }, 10_000)

  it('declare-time QoS is reflected on the received sample', async () => {
    const session = await open()
    const sub = await session.declareSubscriber('demo/qos')
    const pub = await session.declarePublisher('demo/qos', {
      priority: 'DataHigh',
      express: true,
      congestionControl: 'Block',
    })
    await settle()

    await pub.put('x')
    const s = await sub.receive()

    expect(s!.priority).toBe('DataHigh')
    expect(s!.express).toBe(true)
    expect(s!.congestionControl).toBe('Block')

    await pub.undeclare()
    await sub.undeclare()
    await session.close()
  }, 10_000)

  it('publisher.delete yields a Delete sample', async () => {
    const session = await open()
    const sub = await session.declareSubscriber('demo/del')
    const pub = await session.declarePublisher('demo/del')
    await settle()

    await pub.delete()
    const s = await sub.receive()

    expect(s!.kind).toBe('Delete')

    await pub.undeclare()
    await sub.undeclare()
    await session.close()
  }, 10_000)
})
