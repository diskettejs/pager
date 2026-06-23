#![allow(dead_code, unused)] // TODO: remove this once API surface is settled

//! Node.js native bindings for Zenoh, built with NAPI-RS.
//!
//! The surface mirrors `zenoh`'s public API 1:1; only runtime mechanics
//! (async resolution, ownership, JS value marshaling) are adapted.

mod bytes;
mod cancellation;
mod config;
mod encoding;
mod endpoint;
mod entity_global_id;
mod handlers;
mod hello;
mod keyexpr;
mod liveliness;
mod liveliness_subscriber;
mod liveliness_token;
mod locator;
mod matching_listener;
mod matching_status;
mod metadata;
mod miss;
mod options;
mod parameters;
mod publisher;
mod qos;
mod querier;
mod query;
mod queryable;
mod reply;
mod reply_error;
mod sample;
mod sample_miss_listener;
mod selector;
mod serialization;
mod session;
mod source_info;
mod subscriber;
mod time;
mod time_range;
mod whatami;
mod whatami_matcher;
