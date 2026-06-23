# `@diskette/dialtone`

Node.js native bindings for [Zenoh](https://zenoh.io) — a pub/sub/query protocol — built with [NAPI-RS](https://napi.rs).

The bindings mirror **Zenoh 1.9**'s surface as faithfully as the JS boundary allows: the operations, options, and lifecycle semantics are Zenoh's. Advanced pub/sub (from `zenoh-ext`) is folded into the regular `Publisher`/`Subscriber` surface — every declared publisher and subscriber is an advanced one. This package is **Node.js only**; there are no WASM/WASI builds for the browser.

## Install

```bash
pnpm add @diskette/dialtone
```

```ts
import { Session } from '@diskette/dialtone'
```

## Requirements

- **Node.js ≥ 20.4** — the disposal helpers (`using` / `await using`) rely on `Symbol.dispose` / `Symbol.asyncDispose`, available from Node 20.4. The runtime API itself works on older Node, but explicit resource management does not.
- **TypeScript ≥ 5.2** (or an equivalent transpiler/bundler) if you use the `using` / `await using` syntax shown throughout these examples.
- Prebuilt native binaries ship for x86_64 Windows, x86_64 / arm64 macOS, and x86_64 Linux (glibc).

## API Reference

TODO
