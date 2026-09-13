//! Durable Agent control-plane application.
//!
//! This crate owns Agent persistence, authorization, leases, and projections.
//! Execution runtimes depend on its narrow worker ports instead of the full
//! application orchestrator.

pub mod agent;
