import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    // These are integration tests that open real Zenoh sessions. Zenoh peers
    // discover each other across processes on the same host, so running test
    // files in parallel workers lets unrelated sessions interfere (multicast
    // scouting + shared ports). Run files sequentially for deterministic results.
    fileParallelism: false,
  },
})
