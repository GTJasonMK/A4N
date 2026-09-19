//! Typed initial character profile mutation commands.

pub use a4n_api_contract::character::{
    AddProfileAttributeInput, AttributeHistoryInput, CharacterProfileCreateResult,
    CharacterProfileDeleteResult, CharacterProfileMutationResult,
    CharacterProfileOverviewReadApplicationPort, CharacterProfileSummary,
    CharacterTimelineReadApplicationPort, CreateProfileInput, ExecuteAttributeDeletionInput,
    GetProfileStateAtChapterInput, ListBehaviorsInput, ListDeletionMarksInput, ListProfilesInput,
    ListSnapshotsInput, ModifyProfileAttributeInput, RequestAttributeDeletionInput,
    ResetDeletionMarksInput, UpdateCharacterProfileInput,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const CHARACTER_MUTATION_WORKER_ID: &str = "character-mutation-worker";
pub const CHARACTER_MUTATION_WORKER_CONTRACT_VERSION: &str = "character-mutation-v3";
pub const CHARACTER_MUTATION_CREATE_OPERATION: &str = "character.mutation.create";
pub const CHARACTER_MUTATION_UPDATE_OPERATION: &str = "character.mutation.update";
pub const CHARACTER_MUTATION_DELETE_OPERATION: &str = "character.mutation.delete";
pub const CHARACTER_OVERVIEW_PROJECT_OPERATION: &str = "character.overview.project";
pub const CHARACTER_OVERVIEW_LIST_OPERATION: &str = "character.overview.list";
pub const CHARACTER_OVERVIEW_ARC_OPERATION: &str = "character.overview.arc";
pub const CHARACTER_OVERVIEW_VOICE_OPERATION: &str = "character.overview.voice";
pub const CHARACTER_MUTATION_CAPABILITY: &str = "character.mutation";
pub const CHARACTER_OVERVIEW_CAPABILITY: &str = "character.overview";
pub const CHARACTER_TIMELINE_ATTRIBUTE_OPERATION: &str = "character.timeline.attribute_history";
pub const CHARACTER_TIMELINE_BEHAVIOR_OPERATION: &str = "character.timeline.behavior_history";
pub const CHARACTER_TIMELINE_DELETION_OPERATION: &str = "character.timeline.deletion_marks";
pub const CHARACTER_TIMELINE_SNAPSHOTS_OPERATION: &str = "character.timeline.snapshots";
pub const CHARACTER_TIMELINE_STATE_OPERATION: &str = "character.timeline.state_at_chapter";
pub const CHARACTER_TIMELINE_CAPABILITY: &str = "character.timeline.read";
pub const CHARACTER_ATTRIBUTE_ADD_OPERATION: &str = "character.attribute.add";
pub const CHARACTER_ATTRIBUTE_MODIFY_OPERATION: &str = "character.attribute.modify";
pub const CHARACTER_ATTRIBUTE_REQUEST_DELETION_OPERATION: &str =
    "character.attribute.request_deletion";
pub const CHARACTER_ATTRIBUTE_EXECUTE_DELETION_OPERATION: &str =
    "character.attribute.execute_deletion";
pub const CHARACTER_ATTRIBUTE_RESET_DELETION_OPERATION: &str = "character.attribute.reset_deletion";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AttributeEventMutationResult {
    pub project_id: i64,
    pub event_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateInput(pub CreateProfileInput);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateInput(pub UpdateCharacterProfileInput);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileIdInput {
    pub profile_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedProfileInput {
    pub project_id: i64,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };
    use serde_json::json;

    #[test]
    fn character_mutation_scope_is_stable() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: CHARACTER_MUTATION_CREATE_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(9),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: CHARACTER_MUTATION_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: CreateInput(CreateProfileInput {
                project_id: 9,
                name: "林夏".into(),
                base_attributes: json!({}),
            }),
        };
        request.validate().expect("character mutation validates");
    }
}
