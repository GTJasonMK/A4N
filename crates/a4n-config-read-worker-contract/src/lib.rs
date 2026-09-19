//! Typed provider configuration read commands.

pub use a4n_api_contract::config::{
    ConfigProviderKind, CreateEmbeddingInput, CreateLlmInput, EmbeddingConfigSummary,
    ImageConfigForConnectivityTest, ImageConfigInput, ImageConfigSummary, LlmConfigSummary,
    UpdateEmbeddingInput, UpdateImageConfigInput, UpdateLlmInput, UpdateVideoConfigInput,
    VideoConfigInput, VideoConfigSummary,
};
pub use a4n_provider_contract::{EmbeddingProviderConfig, LlmProviderConfig};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const CONFIG_READ_WORKER_ID: &str = "config-read-worker";
pub const CONFIG_READ_WORKER_CONTRACT_VERSION: &str = "config-read-v1";
pub const CONFIG_READ_CAPABILITY: &str = "config.read";
pub const CONFIG_READ_LLM_EXISTS_OPERATION: &str = "config.read.llm_exists";
pub const CONFIG_READ_IMAGE_EXISTS_OPERATION: &str = "config.read.image_exists";
pub const CONFIG_READ_VIDEO_EXISTS_OPERATION: &str = "config.read.video_exists";
pub const CONFIG_READ_SAVED_API_KEY_OPERATION: &str = "config.read.saved_api_key";
pub const CONFIG_READ_LLM_CONNECTIVITY_OPERATION: &str = "config.read.llm_connectivity";
pub const CONFIG_READ_EMBEDDING_CONNECTIVITY_OPERATION: &str = "config.read.embedding_connectivity";
pub const CONFIG_READ_IMAGE_CONNECTIVITY_OPERATION: &str = "config.read.image_connectivity";
pub const CONFIG_READ_LLM_LIST_OPERATION: &str = "config.read.llm_list";
pub const CONFIG_READ_IMAGE_LIST_OPERATION: &str = "config.read.image_list";
pub const CONFIG_READ_VIDEO_LIST_OPERATION: &str = "config.read.video_list";
pub const CONFIG_READ_EMBEDDING_LIST_OPERATION: &str = "config.read.embedding_list";
pub const CONFIG_MUTATION_CAPABILITY: &str = "config.mutation";
pub const CONFIG_MUTATION_BASELINE_EMBEDDING_OPERATION: &str = "config.mutation.baseline_embedding";
pub const CONFIG_MUTATION_BASELINE_LLM_OPERATION: &str = "config.mutation.baseline_llm";
pub const CONFIG_MUTATION_BASELINE_IMAGE_OPERATION: &str = "config.mutation.baseline_image";
pub const CONFIG_MUTATION_BASELINE_VIDEO_OPERATION: &str = "config.mutation.baseline_video";
pub const CONFIG_MUTATION_CREATE_LLM_OPERATION: &str = "config.mutation.create_llm";
pub const CONFIG_MUTATION_UPDATE_LLM_OPERATION: &str = "config.mutation.update_llm";
pub const CONFIG_MUTATION_SET_DEFAULT_LLM_OPERATION: &str = "config.mutation.set_default_llm";
pub const CONFIG_MUTATION_DELETE_LLM_OPERATION: &str = "config.mutation.delete_llm";
pub const CONFIG_MUTATION_CREATE_IMAGE_OPERATION: &str = "config.mutation.create_image";
pub const CONFIG_MUTATION_UPDATE_IMAGE_OPERATION: &str = "config.mutation.update_image";
pub const CONFIG_MUTATION_SET_DEFAULT_IMAGE_OPERATION: &str = "config.mutation.set_default_image";
pub const CONFIG_MUTATION_DELETE_IMAGE_OPERATION: &str = "config.mutation.delete_image";
pub const CONFIG_MUTATION_CREATE_VIDEO_OPERATION: &str = "config.mutation.create_video";
pub const CONFIG_MUTATION_UPDATE_VIDEO_OPERATION: &str = "config.mutation.update_video";
pub const CONFIG_MUTATION_SET_DEFAULT_VIDEO_OPERATION: &str = "config.mutation.set_default_video";
pub const CONFIG_MUTATION_DELETE_VIDEO_OPERATION: &str = "config.mutation.delete_video";
pub const CONFIG_MUTATION_CREATE_EMBEDDING_OPERATION: &str = "config.mutation.create_embedding";
pub const CONFIG_MUTATION_UPDATE_EMBEDDING_OPERATION: &str = "config.mutation.update_embedding";
pub const CONFIG_MUTATION_SET_DEFAULT_EMBEDDING_OPERATION: &str =
    "config.mutation.set_default_embedding";
pub const CONFIG_MUTATION_DELETE_EMBEDDING_OPERATION: &str = "config.mutation.delete_embedding";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EmptyInput {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfigIdInput {
    pub config_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SavedApiKeyInput {
    pub kind: ConfigProviderKindWire,
    pub config_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateLlmCommand(pub CreateLlmInput);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateLlmCommand {
    pub config_id: i64,
    pub input: UpdateLlmInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateImageCommand(pub ImageConfigInput);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateImageCommand {
    pub config_id: i64,
    pub input: UpdateImageConfigInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateVideoCommand(pub VideoConfigInput);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateVideoCommand {
    pub config_id: i64,
    pub input: UpdateVideoConfigInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateEmbeddingCommand(pub CreateEmbeddingInput);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateEmbeddingCommand {
    pub config_id: i64,
    pub input: UpdateEmbeddingInput,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfigProviderKindWire {
    Llm,
    Image,
    Video,
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn config_read_scope_is_stable() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::Service,
                subject: "runtime".into(),
            },
            operation_id: CONFIG_READ_LLM_LIST_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: CONFIG_READ_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: EmptyInput {},
        };
        request.validate().expect("config read request validates");
    }
}
