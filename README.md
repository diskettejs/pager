# `@diskette/dialtone`

Node.js native bindings for [Zenoh](https://zenoh.io) — a pub/sub/query protocol for
data in motion — built with [NAPI-RS](https://napi.rs).

> **Status:** early prototype. Today it covers the **session + pub/sub** surface
> (open a session, publish, subscribe, configure QoS). Queryables, `get`, and
> liveliness are not implemented yet. Node.js only — there is no browser build.

Every operation is asynchronous and returns a `Promise`; there are no blocking
variants. Payloads are bytes on the wire — you send `string | Uint8Array` and
always receive `Uint8Array`.

## Install

```bash
pnpm add @diskette/dialtone
```

## Quick start

A self-contained loopback example — one session both publishes and subscribes:

```ts
import { Session } from '@diskette/dialtone'

const session = await Session.open()
const sub = await session.declareSubscriber('demo/example')

await session.put('demo/example', 'hello zenoh')

const sample = await sub.receive()
console.log(sample?.keyExpr, new TextDecoder().decode(sample!.payload))
// → demo/example hello zenoh

await sub.undeclare()
await session.close()
```

> **Import from the package root** (`@diskette/dialtone`), not the generated
> `index.js`. The root entry adds `Symbol.asyncDispose` to `Session`, `Publisher`,
> and `Subscriber` so they work with `await using` (see
> [Resource management](#resource-management)).

---

## Concepts

- **Key expressions** — every value is published and subscribed on a slash-separated
  key like `demo/example` or `robot/+/temperature`. Subscribers can use wildcards
  (`*`, `**`); publishers use a concrete key.
- **Payloads** — pass a `string` (UTF-8 encoded) or a `Uint8Array` (raw bytes;
  Node `Buffer` works, since it is a `Uint8Array`). A received `Sample` always
  carries `payload` as a `Uint8Array` — decode with `TextDecoder` when you expect
  text.
- **QoS & metadata** — `priority`, `congestionControl`, `express`,
  `allowedDestination`, plus an `encoding` hint and an opaque `attachment` ride
  along with each message.
- **Lifecycle** — `Session`, `Publisher`, and `Subscriber` hold native resources.
  Release them with `close()` / `undeclare()`, or let `await using` do it for you.

---

## API reference

The package exports four classes — `Session`, `Config`, `Publisher`, `Subscriber` —
and four string-union enums — `CongestionControl`, `Priority`, `Locality`,
`SampleKind`.

### `Session`

The entry point. Open one, then publish, subscribe, or declare publishers on it.

#### `Session.open(config?): Promise<Session>` _(static)_

Open a session. Defaults to **peer** mode with multicast scouting.

| `config`            | Behaviour                                                         |
| ------------------- | ----------------------------------------------------------------- |
| _omitted_           | Default peer configuration.                                       |
| `string`            | A JSON5 (or plain JSON) config string, e.g. `'{ mode: "peer" }'`. |
| [`Config`](#config) | A configuration object built via the `Config` API.                |

```ts
const a = await Session.open() // default peer
const b = await Session.open('{ mode: "peer" }')
const c = await Session.open(Config.default())
```

#### Members

| Member                                                    | Description                                                                                             |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `get zid(): string`                                       | This session's Zenoh ID (hex string).                                                                   |
| `info(): Promise<SessionInfo>`                            | Snapshot of the network as this session sees it.                                                        |
| `put(keyExpr, payload, options?): Promise<void>`          | Publish a value on `keyExpr`. `options`: [`PutOptions`](#putoptions).                                   |
| `delete(keyExpr, options?): Promise<void>`                | Publish a delete (tombstone) on `keyExpr`. `options`: [`DeleteOptions`](#deleteoptions).                |
| `declarePublisher(keyExpr, options?): Promise<Publisher>` | Declare a [`Publisher`](#publisher) with fixed QoS. `options`: [`PublisherOptions`](#publisheroptions). |
| `declareSubscriber(keyExpr): Promise<Subscriber>`         | Declare a [`Subscriber`](#subscriber) on `keyExpr` (wildcards allowed).                                 |
| `close(): Promise<void>`                                  | Close the session and release its resources. Operations reject afterwards.                              |

`session.put` / `session.delete` are the convenient one-shot path — they take the
full QoS options per call. For repeated sends on the same key with fixed QoS,
declare a [`Publisher`](#publisher) instead.

### `Config`

Builds the configuration passed to `Session.open()`. Useful when multicast
discovery isn't available and you need explicit `connect` / `listen` endpoints.

| Member                                               | Description                                                           |
| ---------------------------------------------------- | --------------------------------------------------------------------- |
| `Config.default(): Config` _(static)_                | Default configuration (peer mode).                                    |
| `Config.fromJson5(json5: string): Config` _(static)_ | Parse a JSON5 / JSON config string.                                   |
| `Config.fromFile(path: string): Config` _(static)_   | Load from a file (format inferred from `.json5` / `.json` / `.yaml`). |
| `Config.fromEnv(): Config` _(static)_                | Load from the file named by the `ZENOH_CONFIG` env var.               |
| `insertJson5(key: string, value: string): void`      | Set a value at a key path; `value` is a JSON5 fragment.               |
| `toString(): string`                                 | Serialize to a JSON string (private fields elided).                   |

```ts
const config = Config.default()
config.insertJson5('scouting/multicast/enabled', 'false')
config.insertJson5('connect/endpoints', '["tcp/127.0.0.1:7447"]')
```

### `Publisher`

A handle bound to one key expression, with the QoS chosen at declare time. Per-call
you can only vary `encoding` and `attachment`.

| Member                                  | Description                                                                                   |
| --------------------------------------- | --------------------------------------------------------------------------------------------- |
| `get keyExpr(): string`                 | The key expression this publisher sends on.                                                   |
| `put(payload, options?): Promise<void>` | Publish a value. `options`: [`PublisherPutOptions`](#publisherputoptions).                    |
| `delete(options?): Promise<void>`       | Publish a delete (tombstone). `options`: [`PublisherDeleteOptions`](#publisherdeleteoptions). |
| `undeclare(): Promise<void>`            | Undeclare and release resources.                                                              |

### `Subscriber`

Receives samples for its key expression. Consume it two ways — pull one at a time
with `receive()`, or drive it as an **async iterable**.

| Member                               | Description                                                                                 |
| ------------------------------------ | ------------------------------------------------------------------------------------------- |
| `get keyExpr(): string`              | The key expression this subscriber listens on.                                              |
| `receive(): Promise<Sample \| null>` | Resolve with the next [`Sample`](#sample), or `null` once closed/undeclared.                |
| `undeclare(): Promise<void>`         | Undeclare and release resources.                                                            |
| `[Symbol.asyncIterator]()`           | `for await (const sample of sub) { … }`. Breaking out of the loop undeclares automatically. |

```ts
// Pull loop
for (let s = await sub.receive(); s !== null; s = await sub.receive()) {
  console.log(new TextDecoder().decode(s.payload))
}

// Async iteration (auto-undeclares on break/return)
for await (const sample of sub) {
  console.log(sample.kind, new TextDecoder().decode(sample.payload))
  break
}
```

---

## Data types

### `Sample`

Delivered to a subscriber for each published value or delete.

| Field               | Type                          | Description                                      |
| ------------------- | ----------------------------- | ------------------------------------------------ |
| `keyExpr`           | `string`                      | The key the value was published on.              |
| `payload`           | `Uint8Array`                  | The raw value bytes.                             |
| `kind`              | [`SampleKind`](#enums)        | `'Put'` for a value, `'Delete'` for a tombstone. |
| `encoding`          | `string`                      | Encoding hint, e.g. `'text/plain'`.              |
| `timestamp?`        | [`Timestamp`](#timestamp)     | Present when the source attached one.            |
| `congestionControl` | [`CongestionControl`](#enums) | QoS it was sent with.                            |
| `priority`          | [`Priority`](#enums)          | QoS it was sent with.                            |
| `express`           | `boolean`                     | Whether it bypassed batching.                    |
| `attachment?`       | `Uint8Array`                  | Opaque metadata, if any.                         |

### `Timestamp`

| Field   | Type     | Description                                                       |
| ------- | -------- | ----------------------------------------------------------------- |
| `id`    | `string` | Zenoh ID of the source that created the timestamp.                |
| `ntp64` | `bigint` | Raw NTP64 value (32.32 fixed-point seconds since the UNIX epoch). |

There is no built-in `Date` converter; decode the NTP64 fixed-point yourself when
you need wall-clock time:

```ts
function ntp64ToDate(ntp64: bigint): Date {
  const seconds = Number(ntp64 >> 32n)
  const millisFraction = Number(((ntp64 & 0xffffffffn) * 1000n) >> 32n)
  return new Date(seconds * 1000 + millisFraction)
}
```

### `SessionInfo`

Returned by `session.info()`.

| Field     | Type       | Description                     |
| --------- | ---------- | ------------------------------- |
| `zid`     | `string`   | This session's Zenoh ID.        |
| `routers` | `string[]` | Zenoh IDs of connected routers. |
| `peers`   | `string[]` | Zenoh IDs of connected peers.   |

### Options

All option fields are optional.

#### `PutOptions`

`encoding?: string` · `congestionControl?` · `priority?` · `express?: boolean` ·
`attachment?: string | Uint8Array` · `allowedDestination?`

#### `DeleteOptions`

Same as `PutOptions` minus `encoding` (a delete carries no payload).

#### `PublisherOptions`

QoS fixed at declare time: `encoding?: string` · `congestionControl?` ·
`priority?` · `express?: boolean` · `allowedDestination?`

#### `PublisherPutOptions`

`encoding?: string` · `attachment?: string | Uint8Array` _(QoS is fixed by the publisher)._

#### `PublisherDeleteOptions`

`attachment?: string | Uint8Array`

### Enums

Each is a TypeScript string-literal union (and a runtime object of the same name).

| Enum                | Values                                                                                                     | Meaning                                         |
| ------------------- | ---------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| `SampleKind`        | `'Put'`, `'Delete'`                                                                                        | Value vs. tombstone.                            |
| `CongestionControl` | `'Drop'`, `'Block'`                                                                                        | Drop on congestion, or block until sendable.    |
| `Priority`          | `'RealTime'`, `'InteractiveHigh'`, `'InteractiveLow'`, `'DataHigh'`, `'Data'`, `'DataLow'`, `'Background'` | Highest → lowest transmission priority.         |
| `Locality`          | `'SessionLocal'`, `'Remote'`, `'Any'`                                                                      | Restrict which subscribers a message can reach. |

---

## Usage patterns

### Declared publisher with fixed QoS

When you publish repeatedly on the same key, declare a publisher once. Its QoS is
fixed at declare time; only `encoding` / `attachment` vary per `put`.

```ts
const pub = await session.declarePublisher('robot/telemetry', {
  priority: 'DataHigh',
  congestionControl: 'Block',
  express: true,
})

await pub.put(JSON.stringify({ battery: 0.82 }), { encoding: 'application/json' })
await pub.put('low-battery', { attachment: 'alert' })

await pub.undeclare()
```

### One-shot put with per-message QoS

`session.put` takes the full QoS options inline — no publisher needed.

```ts
await session.put('scan/events', 'job-done', {
  encoding: 'text/plain',
  priority: 'DataHigh',
  congestionControl: 'Block',
  express: true,
  attachment: 'meta',
  allowedDestination: 'Any',
})
```

### Deletes (tombstones)

A delete propagates as a `Sample` with `kind === 'Delete'` and no payload.

```ts
await session.delete('robot/telemetry') // session-level
// or, on a publisher:
await pub.delete({ attachment: 'reason: shutdown' })

const s = await sub.receive()
if (s?.kind === 'Delete') console.log('removed', s.keyExpr)
```

### Connecting two peers over explicit TCP (multicast off)

When multicast discovery isn't available, wire peers together with explicit
endpoints — one listens, the other connects.

```ts
const endpoint = 'tcp/127.0.0.1:7447'

const listenerCfg = Config.default()
listenerCfg.insertJson5('scouting/multicast/enabled', 'false')
listenerCfg.insertJson5('listen/endpoints', `["${endpoint}"]`)

const connectorCfg = Config.default()
connectorCfg.insertJson5('scouting/multicast/enabled', 'false')
connectorCfg.insertJson5('connect/endpoints', `["${endpoint}"]`)

const listener = await Session.open(listenerCfg)
const connector = await Session.open(connectorCfg)

const sub = await listener.declareSubscriber('scan/events')
await connector.put('scan/events', 'job-done')

console.log((await listener.info()).peers) // includes connector.zid once linked
```

### Resource management with `await using`

`Session`, `Publisher`, and `Subscriber` implement `Symbol.asyncDispose`, so
`await using` releases them in reverse order on scope exit — no manual
`close()` / `undeclare()`.

```ts
import { Session } from '@diskette/dialtone'

{
  await using session = await Session.open()
  await using sub = await session.declareSubscriber('demo/dispose')
  await using pub = await session.declarePublisher('demo/dispose')

  await pub.put('bye')
  const sample = await sub.receive()
  console.log(new TextDecoder().decode(sample!.payload))
} // ← pub.undeclare() → sub.undeclare() → session.close(), automatically
```

---

## Releasing

Releases are **tag-driven**, handled by GitHub Actions — do not run `npm publish`
by hand. Bump the version and push:

```bash
npm version <patch | minor | major>
git push --follow-tags
```

The `publish` CI job runs when the latest commit message is a version string, and
requires the `NPM_TOKEN` repository secret.
