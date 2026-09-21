//! Queue contracts and in-memory task scheduling.
//!
//! This crate is the canonical owner of task queue contracts and the in-memory
//! scheduler. Runtime and services should depend on `a4n-queue` directly.

mod queue;
mod scope;

pub use queue::{TaskFamilyContext, TaskWorkflowContext, memory_queue, priority};
pub use scope::{with_image_slot, with_llm_slot, with_video_slot};
