//! Queue contracts and in-memory task scheduling.
//!
//! This crate is the canonical owner of task queue contracts and the in-memory
//! scheduler. Runtime and services should depend on `a4n-queue` directly.

pub mod queue;
pub mod scope;

pub use queue::{memory_queue, priority};
