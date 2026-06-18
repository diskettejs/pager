// Types for the entry wrapper (`main.js`). Re-exports the generated surface and
// merges in the `Symbol.asyncDispose` members that `main.js` attaches at runtime.
// This augmentation is the one thing that can't be expressed in Rust — NAPI-RS has
// no codegen for the dispose symbols.
export * from './index.js'

declare module './index.js' {
  interface Session {
    [Symbol.asyncDispose](): Promise<void>
  }
  interface Publisher {
    [Symbol.asyncDispose](): Promise<void>
  }
  interface Subscriber {
    [Symbol.asyncDispose](): Promise<void>
  }
}
