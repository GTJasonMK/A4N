use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::policy::NO_CONTEXT;
use crate::agent::{
    AgentToolCostClass, AgentToolExposure, AgentToolReplayPolicy, AgentToolReversibility,
    AgentToolRisk,
};
use crate::package::{PackageCommandCategory, PackageId};

pub const COMMAND_MANIFEST_VERSION: u32 = 20;
pub const HOST_IDENTITY: &str = "a4n-server";
pub const RUNTIME_EQUIVALENTS: &[(&str, &str)] = &[
    ("get_generation_task_snapshot", "task snapshot"),
    ("get_generation_task_summary", "task get"),
    ("list_active_generation_tasks", "task active"),
    ("list_generation_task_history", "task history"),
    (
        "local_cancel_embedding_model_download",
        "call cancel_embedding_model_download",
    ),
    (
        "local_delete_embedding_model",
        "call delete_local_embedding_model",
    ),
    (
        "local_download_embedding_model",
        "call download_embedding_model",
    ),
    (
        "local_embed_texts",
        "call rag_enhanced_search / call rag_ingest_all（由 Server 执行 embedding）",
    ),
    (
        "local_get_embedding_model_download_info",
        "call get_embedding_model_download_info",
    ),
    (
        "local_list_embedding_models",
        "call list_local_embedding_models",
    ),
    (
        "local_test_embedding_config",
        "call test_embedding_config（使用 Server 配置）",
    ),
    ("subscribe_generation_task", "task watch"),
    ("unsubscribe_generation_task", "结束 task watch（Ctrl-C）"),
];

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct CommandManifest {
    pub manifest_version: u32,
    pub total_commands: usize,
    pub matched_commands: usize,
    pub commands: Vec<CommandDescriptor>,
    pub runtime_equivalents: Vec<RuntimeEquivalent>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct CommandDescriptor {
    pub name: String,
    pub kind: String,
    pub domain: String,
    pub summary: String,
    pub auth: String,
    pub effect: String,
    pub execution: String,
    pub args_schema: Value,
    pub result_schema: Value,
    pub secret_pointers: Vec<String>,
    pub file_inputs: Vec<String>,
    pub file_outputs: Vec<String>,
    pub result_references: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub runtime_config_policy: RuntimeConfigPolicy,
    pub idempotency: String,
    pub invoke: String,
    pub page_scopes: Vec<String>,
    pub required_ui_context: Vec<String>,
    pub approval_policy: String,
    pub navigation_hint: Option<String>,
    pub agent_policy: Option<AgentCommandPolicy>,
    pub package_id: PackageId,
    pub package_category: PackageCommandCategory,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct AgentCommandPolicy {
    pub exposure: AgentToolExposure,
    pub risk: AgentToolRisk,
    pub reversibility: AgentToolReversibility,
    pub replay_policy: AgentToolReplayPolicy,
    pub cost_class: AgentToolCostClass,
    pub tags: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeConfigPolicy {
    None,
    WithoutSecrets,
    Full,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct RuntimeEquivalent {
    pub desktop_runtime_command: String,
    pub cli: String,
}

pub(crate) type SchemaFactory = fn() -> Value;

#[derive(Clone, Copy)]
pub(crate) struct Identity {
    pub(crate) auth: &'static str,
    pub(crate) context_fields: &'static [&'static str],
}

#[derive(Clone, Copy)]
pub(crate) struct Effect {
    pub(crate) effect: &'static str,
    pub(crate) idempotency: &'static str,
}

#[derive(Clone, Copy)]
pub(crate) struct CommandSpec {
    pub(crate) name: &'static str,
    pub(crate) args_schema: SchemaFactory,
    pub(crate) result_schema: SchemaFactory,
    pub(crate) auth: &'static str,
    pub(crate) domain: &'static str,
    pub(crate) effect: &'static str,
    pub(crate) execution: &'static str,
    pub(crate) context_fields: &'static [&'static str],
    pub(crate) secret_pointers: &'static [&'static str],
    pub(crate) file_inputs: &'static [&'static str],
    pub(crate) file_outputs: &'static [&'static str],
    pub(crate) result_references: &'static [&'static str],
    pub(crate) required_capabilities: &'static [&'static str],
    pub(crate) runtime_config_policy: RuntimeConfigPolicy,
    pub(crate) idempotency: &'static str,
}

#[derive(Clone, Copy)]
pub(crate) struct AgentPolicySpec {
    pub(crate) exposure: AgentToolExposure,
    pub(crate) risk: AgentToolRisk,
    pub(crate) reversibility: AgentToolReversibility,
    pub(crate) replay_policy: AgentToolReplayPolicy,
    pub(crate) cost_class: AgentToolCostClass,
    pub(crate) tags: &'static [&'static str],
}

impl CommandSpec {
    pub(crate) const fn new(
        name: &'static str,
        args_schema: SchemaFactory,
        result_schema: SchemaFactory,
        domain: &'static str,
    ) -> Self {
        Self {
            name,
            args_schema,
            result_schema,
            auth: "user",
            domain,
            effect: "write",
            execution: "immediate",
            context_fields: NO_CONTEXT,
            secret_pointers: NO_CONTEXT,
            file_inputs: NO_CONTEXT,
            file_outputs: NO_CONTEXT,
            result_references: NO_CONTEXT,
            required_capabilities: NO_CONTEXT,
            runtime_config_policy: RuntimeConfigPolicy::None,
            idempotency: "none",
        }
    }

    pub(crate) const fn identity(mut self, identity: Identity) -> Self {
        self.auth = identity.auth;
        self.context_fields = identity.context_fields;
        self
    }

    pub(crate) const fn effect(mut self, effect: Effect) -> Self {
        self.effect = effect.effect;
        self.idempotency = effect.idempotency;
        self
    }

    pub(crate) const fn execution(mut self, execution: &'static str) -> Self {
        self.execution = execution;
        self
    }

    pub(crate) const fn context(mut self, fields: &'static [&'static str]) -> Self {
        self.context_fields = fields;
        self
    }

    pub(crate) const fn secrets(mut self, values: &'static [&'static str]) -> Self {
        self.secret_pointers = values;
        self
    }

    pub(crate) const fn file_inputs(mut self, values: &'static [&'static str]) -> Self {
        self.file_inputs = values;
        self
    }

    pub(crate) const fn file_outputs(mut self, values: &'static [&'static str]) -> Self {
        self.file_outputs = values;
        self
    }

    pub(crate) const fn references(mut self, values: &'static [&'static str]) -> Self {
        self.result_references = values;
        self
    }

    pub(crate) const fn capabilities(mut self, values: &'static [&'static str]) -> Self {
        self.required_capabilities = values;
        self
    }

    pub(crate) const fn runtime(mut self, policy: RuntimeConfigPolicy) -> Self {
        self.runtime_config_policy = policy;
        self
    }

    pub(crate) const fn idempotency(mut self, value: &'static str) -> Self {
        self.idempotency = value;
        self
    }
}
