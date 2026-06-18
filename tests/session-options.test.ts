import { describe, expect, it } from 'vitest'
import { Config, open } from '../index.js'

const settle = (ms = 200) => new Promise((r) => setTimeout(r, ms))

describe('session put/delete options', () => {
  it('put options are reflected on the received sample', async () => {
    const session = await open()
    const sub = await session.declareSubscriber('demo/putopts')
    await settle()

    await session.put('demo/putopts', 'x', {
      encoding: 'text/plain',
      priority: 'DataHigh',
      congestionControl: 'Block',
      express: true,
      attachment: 'meta',
      allowedDestination: 'Any',
    })
    const s = await sub.receive()

    expect(s!.encoding).toBe('text/plain')
    expect(s!.priority).toBe('DataHigh')
    expect(s!.congestionControl).toBe('Block')
    expect(s!.express).toBe(true)
    expect(new TextDecoder().decode(s!.attachment!)).toBe('meta')

    await sub.undeclare()
    await session.close()
  }, 10_000)

  it('delete options are reflected on the Delete sample', async () => {
    const session = await open()
    const sub = await session.declareSubscriber('demo/delopts')
    await settle()

    await session.delete('demo/delopts', {
      priority: 'DataLow',
      congestionControl: 'Block',
      express: true,
      attachment: 'gone',
    })
    const s = await sub.receive()

    expect(s!.kind).toBe('Delete')
    expect(s!.priority).toBe('DataLow')
    expect(s!.congestionControl).toBe('Block')
    expect(s!.express).toBe(true)
    expect(new TextDecoder().decode(s!.attachment!)).toBe('gone')

    await sub.undeclare()
    await session.close()
  }, 10_000)
})

describe('session.info()', () => {
  it('reports this session zid and array-shaped router/peer lists', async () => {
    const session = await open()
    const info = await session.info()

    expect(info.zid).toBe(session.zid)
    expect(info.zid).toMatch(/^[0-9a-f]+$/i)
    expect(Array.isArray(info.routers)).toBe(true)
    expect(Array.isArray(info.peers)).toBe(true)

    await session.close()
  }, 10_000)

  it('lists a directly-connected peer once linked over TCP', async () => {
    const endpoint = 'tcp/127.0.0.1:17448'

    const listenerCfg = Config.default()
    listenerCfg.insertJson5('scouting/multicast/enabled', 'false')
    listenerCfg.insertJson5('listen/endpoints', `["${endpoint}"]`)

    const connectorCfg = Config.default()
    connectorCfg.insertJson5('scouting/multicast/enabled', 'false')
    connectorCfg.insertJson5('connect/endpoints', `["${endpoint}"]`)

    const a = await open(listenerCfg)
    const b = await open(connectorCfg)
    await settle(500) // let the TCP link establish

    const aInfo = await a.info()
    const bInfo = await b.info()

    // Each peer should see the other's zid in its peer list.
    expect(aInfo.peers).toContain(b.zid)
    expect(bInfo.peers).toContain(a.zid)

    await b.close()
    await a.close()
  }, 15_000)
})
