//! Node.js native bindings for Zenoh, built with NAPI-RS.
//!
//! The surface mirrors `zenoh`'s public API 1:1; only runtime mechanics
//! (async resolution, ownership, JS value marshaling) are adapted.

mod bytes;
mod config;
mod error;
mod handlers;
mod keyexpr;
mod liveliness;
mod matching;
mod publisher;
mod qos;
mod querier;
mod query;
mod queryable;
mod sample;
mod scouting;
mod session;
mod subscriber;
mod time;
