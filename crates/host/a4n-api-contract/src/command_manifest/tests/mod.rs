use super::registry::COMMAND_SPECS;
use super::{
    API_COMMANDS, RUNTIME_EQUIVALENTS, RuntimeConfigPolicy, command_descriptor,
    command_runtime_config_policy, is_destructive,
};
use crate::generation_tasks::GENERATION_START_ROUTES;
use std::collections::HashSet;

fn assert_commands_use_authenticated_host_identity(commands: &[&str]) {
    for &command in commands {
        let descriptor = command_descriptor(command).unwrap();
        assert!(
            descriptor.args_schema["x-a4n-context-fields"]
                .as_array()
                .unwrap()
                .iter()
                .all(|field| field.as_str() != Some("userId")),
            "{command} must use the authenticated Host identity"
        );
        assert_eq!(
            descriptor.args_schema["additionalProperties"], false,
            "{command} must reject retired identity args"
        );
        assert!(descriptor.args_schema["properties"].get("userId").is_none());
    }
}

include!("manifest.rs");
include!("contracts.rs");
