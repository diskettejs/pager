import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    // Every `get` in the suite is bounded (see tests/helpers.ts `bounded`), so
    // nothing should approach Zenoh's 10s default query timeout. A tight cap
    // turns a genuine hang (a delivery that never arrives, a channel that never
    // closes) into a fast failure instead of a 15s stall.
    testTimeout: 5_000,
    hookTimeout: 5_000,
    // Run one test file at a time. Each test opens a real Zenoh session, and the
    // default config has peers discover one another; letting every file's worker
    // call Session.open() at once produces a discovery storm that makes opens
    // crawl. Tests within a file already run sequentially, so this caps live
    // sessions at ~one, keeping opens fast and the suite deterministic.
    fileParallelism: false,
  },
})
