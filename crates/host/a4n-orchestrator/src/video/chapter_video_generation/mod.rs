use a4n_domain::media::GeneratedVideoRecord;
use a4n_domain::video_prompt::ChapterVideoPromptRecord;
use a4n_error::AppError;
use std::collections::HashMap;

use a4n_api_contract::video::{GenerateChapterVideoInput, PreviewChapterVideoPromptOverrideInput};
use a4n_db::collaboration::ProjectAccessRole;
use a4n_db_models::models::chapter::{chapter_video_prompt, chapter_video_timeline_item};
use a4n_domain::image::{
    IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT, image_generation_prompt_id,
    image_generation_source, reference_image_ids,
};
use a4n_domain::source_spans::metadata_source_content_sha256;
use a4n_domain::source_text_hash;
use a4n_service_protocol::GenerationArtifactRef;
use a4n_video::{
    ProviderImageInputTransport, VideoGenerationRequest,
    append_video_request_prompt_length_context, provider_request_snapshot_value,
    video_request_prompt_length_context,
};
use a4n_video_contract::{
    VIDEO_EXECUTION_CONTRACT_EPOCH, VideoSegmentExecutionRecipeV1, VideoSegmentGuidanceModeV1,
    VideoSegmentPlanV2, validate_video_segment_recipes_for_plan,
    video_segment_recipes_from_metadata,
};
use a4n_video_host_contract::{
    ChapterVideoGenerationHostPort, ChapterVideoImageInput, ChapterVideoPreviewHostPort,
    ChapterVideoRequestPreview, VideoProviderRequestInput, VideoProviderValidationMode,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::OrchestratorContext;

use super::boundary_frame_rules::latest_boundary_frame_image_ids;
use super::chapter_video_request_rules::{
    BoundaryFrameImageIds, ChapterVideoProviderRequestContext, ProviderPromptImageBinding,
    append_provider_input_note, apply_chapter_video_prompt_preview_override,
    build_video_generation_provider_prompt_with_renderer, chapter_video_provider_descriptor,
    enforce_provider_image_binding_contract, enforce_provider_prompt_length_contract,
    provider_specific_negative_prompt, sanitized_chapter_video_request_for_snapshot,
};
use super::{
    NEWAPI_VIDEO_BACKEND, PromptChainFields, ProviderGenerationWorkflow,
    VIDEO_GENERATION_PROGRESS_TOTAL, VideoGenerationProgress, VideoGenerationRecordInput,
    VideoProviderExecutionInput, boundary_frame_numbers_for_prompt,
    ensure_newapi_first_frame_image_supported, first_frame_prompt_matches_video_prompt,
    frame_prompt_matches_video_prompt, latest_storyboard_image_id, linked_video_preview_config_id,
    prompt_provider_workflow_for_recipe, reference_ids_with_storyboard_image,
    required_prompt_source_prompt_type, required_video_execution_identity,
    storyboard_prompt_matches_video_prompt, tail_frame_reference_belongs_to_video,
    validate_video_duration_for_provider_config_secs,
};

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterVideosBatchInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub prompt_ids: Vec<i64>,
}

#[derive(Default)]
struct VideoPromptValidationCache {
    shared_segment_plans: HashMap<i64, ChapterVideoPromptRecord>,
    shared_segment_indexes: HashMap<i64, SharedSegmentValidationIndex>,
    inserted_timeline_items: HashMap<(i64, i64), HashMap<i64, chapter_video_timeline_item::Model>>,
}

#[derive(Default)]
struct SharedSegmentValidationIndex {
    segments_by_number: HashMap<i32, VideoSegmentPlanV2>,
    segment_numbers_by_shot: HashMap<String, HashMap<i32, i32>>,
    recipes_by_number: HashMap<i32, VideoSegmentExecutionRecipeV1>,
}

struct PreparedChapterVideoGeneration {
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    context: ChapterVideoProviderRequestContext,
    staging_artifacts: Vec<GenerationArtifactRef>,
}

struct ChapterProviderReferences {
    boundary_frame_ids: BoundaryFrameImageIds,
    storyboard_image_id: Option<i64>,
    provider_reference_image_ids_json: Option<Value>,
    provider_reference_image_ids: Vec<i64>,
    provider_reference_image_count: usize,
}

struct VideoPromptExecutionContract {
    source_prompt_type: String,
    execution_identity: Value,
    recipe: VideoSegmentExecutionRecipeV1,
}

include!("generation.rs");
include!("helpers.rs");

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> SharedSegmentValidationIndex {
        SharedSegmentValidationIndex {
            segments_by_number: HashMap::new(),
            segment_numbers_by_shot: HashMap::from([(
                "shot-a".to_string(),
                HashMap::from([(1, 7), (2, 8)]),
            )]),
            recipes_by_number: HashMap::new(),
        }
    }

    #[test]
    fn cached_segment_lookup_keeps_contract() {
        let index = index();
        assert_eq!(resolve_cached_segment_number(&index, " shot-a ", 1), Ok(7));
        assert_eq!(
            resolve_cached_segment_number(&index, "missing", 1),
            Err("当前片段规划不存在镜头 missing 的第 1 个片段".to_string())
        );
        assert_eq!(
            resolve_cached_segment_number(&index, "shot-a", 0),
            Err("稳定片段目标必须提供镜头 ID 和正整数镜头内序号".to_string())
        );
    }

    #[test]
    fn cached_timeline_item_lookup_keeps_first_duplicate() {
        let first = chapter_video_timeline_item::Model {
            id: 7,
            timeline_id: 1,
            position: Some(1),
            item_kind: "inserted".to_string(),
            status: "active".to_string(),
            native_source_shot_id: None,
            native_segment_index_in_shot: None,
            insert_plan_prompt_id: Some(10),
            workflow_run_id: None,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };
        let second = chapter_video_timeline_item::Model {
            insert_plan_prompt_id: Some(11),
            ..first.clone()
        };
        let items = timeline_items_by_id(vec![first, second]);
        assert_eq!(
            items.get(&7).and_then(|item| item.insert_plan_prompt_id),
            Some(10)
        );
    }
}
