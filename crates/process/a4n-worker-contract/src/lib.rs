//! Transport-neutral contracts for stateful A4N workers.
//!
//! Capability sidecars use `a4n-service-protocol`. Stateful workers use this
//! smaller contract so task ownership, fencing, and business payloads do not
//! become part of the capability protocol.

// Bump this whenever the Control/worker HTTP payload changes. The server
// rejects a mixed pair before a stale worker can reach a stricter command
// route with an incompatible payload.
pub const WORKER_SCHEMA_VERSION: u32 = 6;
pub const AGENT_RUN_OPERATION: &str = "agent.run";
pub const AGENT_HOST_COMMAND_OPERATION: &str = "agent.host_command";
pub const AGENT_WORKER_AUTH_HEADER: &str = "x-a4n-agent-worker-token";

pub use a4n_api_contract::agent::AgentScope;

mod agent_operations;
mod agent_requests;
mod agent_runtime;
mod capabilities;
mod protocols;
mod wire;

pub use agent_operations::*;
pub use agent_requests::*;
pub use agent_runtime::*;
pub use capabilities::*;
pub use protocols::*;
pub use wire::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod wire_tests;
