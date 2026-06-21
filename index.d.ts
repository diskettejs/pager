// Types for the entry wrapper (`index.js`). Re-exports the generated surface and
// merges in the disposal members that `index.js` attaches at runtime. This
// augmentation is the one thing that can't be expressed in Rust — NAPI-RS has no
// codegen for the dispose symbols.
//
// The split is deliberate: `Session.close()` is async, so the session is an
// `AsyncDisposable` (`await using`). Every entity's `undeclare()` / `stop()` is
// synchronous (it `wait()`s in Rust), so they are plain `Disposable` (`using`)
// and the member returns `void` to match the generated signature.
export * from './binding.js'

declare module './binding.js' {
  interface Session {
    [Symbol.asyncDispose](): Promise<void>
  }
  interface Publisher {
    [Symbol.dispose](): void
  }
  interface Subscriber {
    [Symbol.dispose](): void
  }
  interface Queryable {
    [Symbol.dispose](): void
  }
  interface Querier {
    [Symbol.dispose](): void
  }
  interface MatchingListener {
    [Symbol.dispose](): void
  }
  interface SampleMissListener {
    [Symbol.dispose](): void
  }
  interface LivelinessToken {
    [Symbol.dispose](): void
  }
  interface Scout {
    [Symbol.dispose](): void
  }
}
