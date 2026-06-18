#![deny(clippy::all)]

//! Node.js native bindings for Zenoh (NAPI-RS).
//!
//! Module layout mirrors Zenoh's concepts: one file per entity / value type.
//! Zenoh types are referenced by fully-qualified path so the JS-facing structs can
//! keep their clean, unprefixed names (`Session`, `Sample`, `Publisher`, …).
//!
//! Async only: every resolving op routes its future onto NAPI's Tokio runtime.
//! Zenoh's blocking `.wait()` terminators are never used (nested-runtime hazard).

mod config;
mod enums;
mod error;
mod macros;
mod payload;
mod publisher;
mod sample;
mod session;
mod subscriber;
