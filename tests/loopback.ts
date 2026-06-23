import { Config } from '../index.js'

// Hermetic single-session loopback config: a peer that neither scouts nor
// connects, so a session receives its own publications without touching the
// network. Zenoh delivers a session's own publications to its own matching
// subscribers (allowed_origin defaults to Any).
export function loopbackConfig(): Config {
  const config = Config.default()
  config.insertJson5('mode', '"peer"')
  config.insertJson5('scouting/multicast/enabled', 'false')
  config.insertJson5('listen/endpoints', '[]')
  config.insertJson5('connect/endpoints', '[]')
  return config
}
