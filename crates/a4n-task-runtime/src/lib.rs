//! Host-local task handles and live progress snapshots.
//!
//! This crate owns registration, cancellation and in-memory progress for
//! running commands. Persistent scheduling and concurrency permits remain in
//! `a4n-queue`.

pub mod channel;
