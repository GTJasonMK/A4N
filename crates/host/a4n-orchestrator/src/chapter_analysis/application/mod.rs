use a4n_api_contract::chapter::{ChapterAnalysisMutationResult, ChapterSummary};
use a4n_api_contract::character::DiscoverCharacterProfilesInput;
use a4n_chapter_analysis::validate_saved_chapter_analysis;
use a4n_chapter_analysis_host_contract::{
    ChapterAnalysisApplicationProgress, ChapterAnalysisGenerationApplicationPort,
    ChapterAnalysisGenerationPreviewApplicationPort, ChapterAnalysisHostPort,
    ChapterAnalysisRequestPreview, ChapterAnalysisRequestStep,
    ChapterAnalysisResumeApplicationPort, ChapterAnalysisResumePreviewApplicationPort,
};
use a4n_db_models::models::{
    chapter::chapter,
    generation::{workflow_checkpoint, workflow_run},
};
use a4n_domain::project::ChapterDerivedCleanupArtifacts;
use a4n_domain::workflow::WORKFLOW_MODE_DERIVED_REPLAY;
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    character::{
        CharacterDiscoveryProgress, CharacterProfileSyncProgress,
        CharacterProfileSyncStageTracePort,
    },
    object_discovery::{
        OBJECT_DISCOVERY_CALL_KEY, OBJECT_DISCOVERY_KIND, ObjectDiscoveryInput,
        ObjectDiscoveryProgress,
    },
    scene_profile::SceneProfileSyncProgress,
    workflow::{
        CompletedWorkflowCheckpointInput, UpsertWorkflowCheckpointInput, WORKFLOW_STATUS_COMPLETED,
        required_checkpoint_field, required_checkpoint_json,
        workflow_checkpoint_fingerprint_sha256,
    },
};

use super::outputs::{
    ensure_chapter_analysis_character_sync_completed, retrospective_chapter_outline_input,
};
use super::{
    CHAPTER_ANALYSIS_REPLACEMENT_CLEANUP_STEP, CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP,
    ChapterAnalysisInput, build_chapter_analysis_initial_request,
    chapter_analysis_phase_fingerprint, chapter_analysis_resume_block_reason,
    chapter_analysis_run_input_matches, chapter_analysis_run_uses_replacement_commit,
    chapter_analysis_workflow_business_key, require_completed_chapter_analysis_phase,
};

const STEP_FACTS: &str = "chapter_analysis.facts";
const STEP_DERIVED_SYNC: &str = "chapter_analysis.derived_sync";
pub use a4n_chapter_content_core::{
    CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY, CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
    CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY, CHAPTER_ANALYSIS_STEP_OUTLINE_SYNC,
    CHAPTER_ANALYSIS_STEP_SCENE_SYNC, CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS,
};
const STEP_REFRESH_STATUS: &str = "chapter_analysis.refresh_status";

include!("analysis.rs");
include!("contracts.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_commit_cleanup_artifacts_round_trip() {
        let checkpoint = json!({
            "completed": true,
            "cleanup_artifacts": {
                "stale_profile_ids": [7],
                "media_file_paths": ["projects/1/old.png"],
                "scene_records_changed": true
            }
        });

        let artifacts = replacement_cleanup_artifacts_from_checkpoint(&checkpoint)
            .expect("valid replacement cleanup artifacts should restore");

        assert_eq!(artifacts.stale_profile_ids, vec![7]);
        assert_eq!(artifacts.media_file_paths, vec!["projects/1/old.png"]);
        assert!(artifacts.scene_records_changed);
    }

    #[test]
    fn replacement_commit_requires_cleanup_artifacts() {
        let error = replacement_cleanup_artifacts_from_checkpoint(&json!({"completed": true}))
            .expect_err("missing replacement cleanup artifacts must fail explicitly");

        assert!(error.to_string().contains("cleanup_artifacts"));
    }
}
