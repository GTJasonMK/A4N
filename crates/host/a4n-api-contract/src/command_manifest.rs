mod model;
mod policy;
mod registry;
mod schemas;

#[cfg(test)]
mod tests;

pub use model::{
    AgentCommandPolicy, COMMAND_MANIFEST_VERSION, CommandDescriptor, CommandManifest,
    HOST_IDENTITY, RUNTIME_EQUIVALENTS, RuntimeConfigPolicy, RuntimeEquivalent,
};
pub use registry::{
    API_COMMANDS, capabilities, command_descriptor, command_manifest,
    command_runtime_config_policy, describe, is_destructive, list,
};
