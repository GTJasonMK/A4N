use crate::source_sync::{
    SourceSyncAnchorMappingRecord, SourceSyncEntityRefRecordInput,
    SourceSyncGalgameSceneWriteRecordInput, SourceSyncMediaRefRecordInput,
    SourceSyncReplacementSpecRecordInput, SourceSyncTextAnchorRefRecordInput,
};
use a4n_domain::source_sync::{SourceSyncConflictStatus, SourceSyncResolutionStrategy};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct SourceSyncRefreshSpecRecordInput {
    pub generation_spec_id: i64,
    pub prompt_kind: String,
    pub params_json: Value,
    pub text_anchor_refs: Vec<SourceSyncTextAnchorRefRecordInput>,
    pub entity_refs: Vec<SourceSyncEntityRefRecordInput>,
    pub media_refs: Vec<SourceSyncMediaRefRecordInput>,
}

#[derive(Debug, Clone)]
pub enum PreparedSourceSyncDecisionRecordInput {
    KeepLocal {
        conflict_id: i64,
        decision_payload_json: Value,
    },
    Detach {
        conflict_id: i64,
        decision_payload_json: Value,
    },
    RebindAnchor {
        conflict_id: i64,
        generation_spec_id: i64,
        anchor_mappings: Vec<SourceSyncAnchorMappingRecord>,
        decision_payload_json: Value,
    },
    Manual {
        conflict_id: i64,
        prompt_artifact_id: i64,
        prompt_text: String,
        decision_payload_json: Value,
    },
    DeleteLocal {
        conflict_id: i64,
        decision_payload_json: Value,
    },
    AdoptSource {
        conflict_id: i64,
        input: SourceSyncRefreshSpecRecordInput,
        decision_payload_json: Value,
    },
    RebuildPrompt {
        conflict_id: i64,
        input: SourceSyncRefreshSpecRecordInput,
        decision_payload_json: Value,
    },
    CreateReplacement {
        conflict_id: i64,
        input: SourceSyncReplacementSpecRecordInput,
        decision_payload_json: Value,
    },
    RegenerateScene {
        conflict_id: i64,
        galgame_scene_id: i64,
        reason: Option<String>,
        expected_scene_index: i32,
        expected_source_snapshot_chapter_id: i64,
        generated_scene: SourceSyncGalgameSceneWriteRecordInput,
        stage_outputs_json: Value,
        decision_payload_json: Value,
    },
}

impl PreparedSourceSyncDecisionRecordInput {
    pub(super) fn conflict_id(&self) -> i64 {
        match self {
            Self::KeepLocal { conflict_id, .. }
            | Self::Detach { conflict_id, .. }
            | Self::RebindAnchor { conflict_id, .. }
            | Self::Manual { conflict_id, .. }
            | Self::DeleteLocal { conflict_id, .. }
            | Self::AdoptSource { conflict_id, .. }
            | Self::RebuildPrompt { conflict_id, .. }
            | Self::CreateReplacement { conflict_id, .. }
            | Self::RegenerateScene { conflict_id, .. } => *conflict_id,
        }
    }

    pub(super) fn strategy(&self) -> SourceSyncResolutionStrategy {
        match self {
            Self::KeepLocal { .. } => SourceSyncResolutionStrategy::KeepLocal,
            Self::Detach { .. } => SourceSyncResolutionStrategy::Detach,
            Self::RebindAnchor { .. } => SourceSyncResolutionStrategy::RebindAnchor,
            Self::Manual { .. } => SourceSyncResolutionStrategy::Manual,
            Self::DeleteLocal { .. } => SourceSyncResolutionStrategy::DeleteLocal,
            Self::AdoptSource { .. } => SourceSyncResolutionStrategy::AdoptSource,
            Self::RebuildPrompt { .. } => SourceSyncResolutionStrategy::RebuildPrompt,
            Self::CreateReplacement { .. } => SourceSyncResolutionStrategy::CreateReplacement,
            Self::RegenerateScene { .. } => SourceSyncResolutionStrategy::RegenerateScene,
        }
    }

    pub(super) fn decision_payload_json(&self) -> &Value {
        match self {
            Self::KeepLocal {
                decision_payload_json,
                ..
            }
            | Self::Detach {
                decision_payload_json,
                ..
            }
            | Self::RebindAnchor {
                decision_payload_json,
                ..
            }
            | Self::Manual {
                decision_payload_json,
                ..
            }
            | Self::DeleteLocal {
                decision_payload_json,
                ..
            }
            | Self::AdoptSource {
                decision_payload_json,
                ..
            }
            | Self::RebuildPrompt {
                decision_payload_json,
                ..
            }
            | Self::CreateReplacement {
                decision_payload_json,
                ..
            }
            | Self::RegenerateScene {
                decision_payload_json,
                ..
            } => decision_payload_json,
        }
    }

    pub(super) fn conflict_status(&self) -> SourceSyncConflictStatus {
        match self {
            Self::KeepLocal { .. } | Self::Detach { .. } => SourceSyncConflictStatus::Ignored,
            Self::RebindAnchor { .. }
            | Self::Manual { .. }
            | Self::DeleteLocal { .. }
            | Self::AdoptSource { .. }
            | Self::RebuildPrompt { .. }
            | Self::CreateReplacement { .. }
            | Self::RegenerateScene { .. } => SourceSyncConflictStatus::Resolved,
        }
    }
}
