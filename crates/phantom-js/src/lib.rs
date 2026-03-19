//! # phantom-js
//!
//! JavaScript engine for Phantom Engine. Provides a sandboxed QuickJS
//! runtime with DOM bindings, browser API shims, and anti-detection
//! measures for AI agent automation.
//!
//! ## Architecture
//!
//! - **Decision D-07**: rquickjs (Tier 1) for 80% of sessions.
//! - **Decision D-08**: One isolate per agent task — "burn it down" on completion.
//! - **Decision D-09**: JS wrappers store `arena_id: u64` only — never Rust references.

pub mod processor;
pub mod quickjs;
pub mod shims;
pub mod v8;
