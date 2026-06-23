import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    // Every `get` in the suite is bounded (see tests/helpers.ts `bounded`), so
    // nothing should approach Zenoh's 10s default query timeout. A tight cap
    // turns a genuine hang (a delivery that never arrives, a channel that never
    // closes) into a fast failure instead of a 15s stall.
    testTimeout: 5_000,
    hookTimeout: 5_000,
  },
})
