use super::builders::*;
use serde_json::{Value, json};

pub(crate) fn args_media_authoring() -> Value {
    strict_typed_input_schema::<crate::media_authoring::MediaAuthoringInput>()
}
pub(crate) fn args_save_authoring_document() -> Value {
    strict_typed_input_schema::<crate::media_authoring::SaveAuthoringDocumentInput>()
}
pub(crate) fn args_adopt_authoring_document() -> Value {
    strict_typed_input_schema::<crate::media_authoring::AdoptAuthoringDocumentInput>()
}
pub(crate) fn args_delete_authoring_document() -> Value {
    strict_typed_input_schema::<crate::media_authoring::DeleteAuthoringDocumentInput>()
}
pub(crate) fn args_preview_delete_authoring_document() -> Value {
    strict_typed_input_schema::<crate::media_authoring::PreviewDeleteAuthoringDocumentInput>()
}
pub(crate) fn args_generate_adaptation_plan() -> Value {
    strict_runtime_typed_input_schema::<crate::media_authoring::GenerateAdaptationPlanInput>()
}
pub(crate) fn result_media_authoring_workspace() -> Value {
    typed_schema::<crate::media_authoring::MediaAuthoringWorkspace>()
}
pub(crate) fn result_authoring_document() -> Value {
    typed_schema::<crate::media_authoring::AuthoringDocumentRecord>()
}
pub(crate) fn result_authoring_document_deletion_impact() -> Value {
    typed_schema::<crate::media_authoring::AuthoringDocumentDeletionImpact>()
}
pub(crate) fn result_video_reference_candidates() -> Value {
    typed_schema::<Vec<crate::media_authoring::VideoScriptReferenceCandidate>>()
}

pub(crate) fn args_apply_assembly_postprocess_edit() -> Value {
    json!({
        "type": "object",
        "properties": {
            "pushVideoId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<crate::video::ApplyAssemblyPostprocessEditInput>("input"),
        },
        "required": ["pushVideoId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_assemble_chapter_videos() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "workflowMode": { "type": "string", "minLength": 1 },
        },
        "required": ["projectId", "chapterId", "workflowMode"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_bind_galgame_media() -> Value {
    strict_typed_input_schema::<crate::galgame::BindGalgameMediaInput>()
}
pub(crate) fn args_bind_manga_reference_image() -> Value {
    strict_typed_input_schema::<crate::manga::BindMangaReferenceImageInput>()
}
pub(crate) fn args_cancel_galgame_pipeline() -> Value {
    strict_typed_input_schema::<crate::galgame::GalgameProjectInput>()
}
pub(crate) fn args_cancel_manga_pipeline() -> Value {
    strict_typed_input_schema::<crate::manga::MangaChapterInput>()
}
pub(crate) fn args_manga_chapter() -> Value {
    strict_typed_input_schema::<crate::manga::MangaChapterInput>()
}
pub(crate) fn args_delete_manga_pipeline_artifact() -> Value {
    strict_typed_input_schema::<crate::manga::DeleteMangaPipelineArtifactInput>()
}
pub(crate) fn args_preview_delete_manga_pipeline_artifact() -> Value {
    strict_typed_input_schema::<crate::manga::PreviewMangaPipelineArtifactDeletionInput>()
}
pub(crate) fn args_delete_manga_page() -> Value {
    strict_typed_input_schema::<crate::manga::DeleteMangaPageInput>()
}
pub(crate) fn args_save_manga_pipeline_artifact() -> Value {
    strict_typed_input_schema::<crate::manga::SaveMangaPipelineArtifactInput>()
}
pub(crate) fn args_list_manga_pipeline_statuses() -> Value {
    strict_typed_input_schema::<crate::manga::MangaPipelineStatusesInput>()
}
pub(crate) fn args_commit_inserted_segment_draft() -> Value {
    typed_input_schema::<crate::video::CommitInsertedSegmentDraftInput>()
}
pub(crate) fn args_delete_chapter_video_prompt() -> Value {
    typed_input_schema::<crate::video::DeleteChapterVideoPromptInput>()
}
pub(crate) fn args_delete_inserted_segment() -> Value {
    typed_input_schema::<crate::video::DeleteInsertedSegmentInput>()
}
pub(crate) fn args_delete_chapter_shooting_script() -> Value {
    strict_typed_input_schema::<crate::shooting_script::DeleteChapterShootingScriptInput>()
}
pub(crate) fn args_delete_galgame_line() -> Value {
    project_typed_input_schema::<crate::galgame::DeleteGalgameLineInput>()
}
pub(crate) fn args_delete_image() -> Value {
    strict_typed_input_schema::<crate::image::DeleteImageInput>()
}
pub(crate) fn args_delete_manga_prompt() -> Value {
    strict_typed_input_schema::<crate::manga::DeleteMangaPromptInput>()
}
pub(crate) fn args_load_video_segment_plan_draft() -> Value {
    strict_typed_input_schema::<crate::video::LoadVideoSegmentPlanDraftInput>()
}
pub(crate) fn args_video_id() -> Value {
    json!({
        "type": "object",
        "properties": { "videoId": { "type": "integer", "minimum": 1 } },
        "required": ["videoId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_poll_video_job() -> Value {
    json!({
        "type": "object",
        "properties": {
            "videoId": { "type": "integer", "minimum": 1 },
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["videoId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_edit_chapter_shooting_script() -> Value {
    strict_typed_input_schema::<crate::shooting_script::EditChapterShootingScriptInput>()
}
pub(crate) fn args_edit_chapter_shooting_script_progress() -> Value {
    strict_typed_input_schema::<crate::shooting_script::EditChapterShootingScriptProgressInput>()
}
pub(crate) fn args_chapter_visual_plan_source() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::ChapterVisualPlanSourceInput>()
}
pub(crate) fn args_save_chapter_visual_plan_draft() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::SaveChapterVisualPlanDraftInput>()
}
pub(crate) fn args_chapter_visual_plan_record() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::ChapterVisualPlanRecordInput>()
}
pub(crate) fn args_read_chapter_visual_plan() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::ReadChapterVisualPlanInput>()
}
pub(crate) fn args_read_visual_plan_prompt_candidate() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::ReadVisualPlanPromptCandidateInput>()
}
pub(crate) fn result_visual_plan_prompt_candidate() -> Value {
    typed_schema::<crate::chapter_visual_plan::VisualPlanPromptCandidate>()
}
pub(crate) fn args_preview_visual_plan_point_generation() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::PreviewVisualPlanPointInput>()
}
pub(crate) fn result_visual_plan_point_generation_preview() -> Value {
    typed_schema::<crate::chapter_visual_plan::VisualPlanPointGenerationPreview>()
}
pub(crate) fn args_start_visual_plan_prompt_generation() -> Value {
    strict_runtime_typed_input_schema::<
        crate::chapter_visual_plan::StartVisualPlanPromptGenerationInput,
    >()
}
pub(crate) fn args_edit_chapter_visual_plan_draft() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::EditChapterVisualPlanDraftInput>()
}
pub(crate) fn args_validate_chapter_visual_plan() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::ValidateChapterVisualPlanInput>()
}
pub(crate) fn args_start_chapter_shooting_script_generation() -> Value {
    strict_runtime_typed_input_schema::<
        crate::shooting_script::StartChapterShootingScriptGenerationInput,
    >()
}
pub(crate) fn args_generate_object_profile_image_prompt() -> Value {
    strict_typed_input_schema::<crate::image_generation::ObjectProfileImageInput>()
}
pub(crate) fn args_generate_portrait_prompt() -> Value {
    strict_runtime_typed_input_schema::<crate::portrait::GeneratePortraitPromptInput>()
}
pub(crate) fn args_generate_scene_profile_image_prompt() -> Value {
    strict_typed_input_schema::<crate::image_generation::SceneProfileImageInput>()
}
pub(crate) fn args_get_canvas_graph() -> Value {
    strict_typed_input_schema::<crate::canvas::GetCanvasGraphInput>()
}
pub(crate) fn args_get_canvas_node_contracts() -> Value {
    strict_typed_input_schema::<crate::canvas::GetCanvasNodeContractsInput>()
}
pub(crate) fn args_get_chapter_shooting_script() -> Value {
    strict_typed_input_schema::<crate::shooting_script::ChapterShootingScriptSourceInput>()
}
pub(crate) fn args_get_primary_portrait() -> Value {
    strict_typed_input_schema::<crate::portrait::GetPrimaryPortraitInput>()
}
pub(crate) fn args_import_canvas_media() -> Value {
    strict_typed_input_schema::<crate::canvas::ImportCanvasMediaInput>()
}
pub(crate) fn args_import_galgame_audio() -> Value {
    strict_typed_input_schema::<crate::galgame::ImportGalgameAudioInput>()
}
pub(crate) fn args_insert_galgame_line() -> Value {
    project_typed_input_schema::<crate::galgame::InsertGalgameLineInput>()
}
pub(crate) fn args_list_chapter_video_prompts() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
        },
        "required": ["projectId", "chapterId"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_list_videos_by_chapter() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
        },
        "required": ["projectId", "chapterId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_load_video_scrub_playback_data() -> Value {
    json!({
        "type": "object",
        "properties": {
            "videoId": { "type": "integer", "minimum": 1 },
            "includeAudio": { "type": "boolean" },
        },
        "required": ["videoId", "includeAudio"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_get_video_timeline() -> Value {
    typed_input_schema::<crate::video::GetVideoTimelineInput>()
}
pub(crate) fn args_move_inserted_segment() -> Value {
    typed_input_schema::<crate::video::MoveInsertedSegmentInput>()
}
pub(crate) fn args_list_images_by_chapter() -> Value {
    strict_typed_input_schema::<crate::image::ListImagesByChapterInput>()
}
pub(crate) fn args_list_images_by_project() -> Value {
    strict_typed_input_schema::<crate::image::ListImagesByProjectInput>()
}
pub(crate) fn args_list_images_by_projects() -> Value {
    strict_typed_input_schema::<crate::image::ListImagesByProjectsInput>()
}
pub(crate) fn args_list_portraits_by_project() -> Value {
    strict_typed_input_schema::<crate::portrait::ListPortraitsByProjectInput>()
}
pub(crate) fn args_list_video_reference_images() -> Value {
    strict_typed_input_schema::<crate::image::ListVideoReferenceImagesInput>()
}
pub(crate) fn args_optimize_chapter_video_prompt() -> Value {
    json!({
        "type": "object",
        "properties": {
            "promptId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<crate::video::OptimizeChapterVideoPromptInput>("input"),
        },
        "required": ["promptId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_optimize_manga_prompt() -> Value {
    strict_typed_input_schema::<crate::manga::OptimizeMangaPromptInput>()
}
pub(crate) fn args_preview_canvas_node_request() -> Value {
    strict_typed_input_schema::<crate::canvas::PreviewCanvasNodeRequestInput>()
}
pub(crate) fn args_preview_chapter_video_request() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "promptId": { "type": "integer", "minimum": 1 },
            "configId": { "type": ["integer", "null"], "minimum": 1 },
            "promptOverride": property_schema::<crate::video::PreviewChapterVideoPromptOverrideInput>("promptOverride"),
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["projectId", "chapterId", "promptId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_preview_reference_media_video_request() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<crate::video::ReferenceMediaVideoInput>("input"),
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["projectId", "chapterId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_preview_video_provider_request_by_video_id() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "videoId": { "type": "integer", "minimum": 1 },
        },
        "required": ["projectId", "chapterId", "videoId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_preview_galgame_pipeline_request() -> Value {
    strict_typed_input_schema::<crate::galgame::PreviewGalgamePipelineInput>()
}
pub(crate) fn args_preview_object_profile_image_request() -> Value {
    strict_runtime_typed_input_schema::<crate::image_generation::ObjectProfileImageInput>()
}
pub(crate) fn args_preview_scene_image_request() -> Value {
    strict_runtime_typed_input_schema::<crate::image_generation::GenerateSceneImageInput>()
}
pub(crate) fn args_preview_page_image_request() -> Value {
    strict_runtime_typed_input_schema::<crate::image_generation::GeneratePageImageInput>()
}
pub(crate) fn args_preview_batch_images_request() -> Value {
    strict_runtime_typed_input_schema::<crate::image_generation::GenerateBatchImagesInput>()
}
pub(crate) fn args_preview_generate_age_variant_request() -> Value {
    strict_runtime_typed_input_schema::<crate::portrait::GenerateAgeVariantInput>()
}
pub(crate) fn args_preview_generate_missing_portraits_request() -> Value {
    strict_runtime_typed_input_schema::<crate::portrait::GenerateMissingPortraitsInput>()
}
pub(crate) fn args_preview_generate_portrait_prompt_request() -> Value {
    strict_runtime_typed_input_schema::<crate::portrait::GeneratePortraitPromptInput>()
}
pub(crate) fn args_preview_generate_portrait_request() -> Value {
    strict_runtime_typed_input_schema::<crate::portrait::GeneratePortraitInput>()
}
pub(crate) fn args_preview_regenerate_portrait_request() -> Value {
    strict_runtime_typed_input_schema::<crate::portrait::RegeneratePortraitInput>()
}
pub(crate) fn args_preview_resume_manga_pipeline_request() -> Value {
    strict_typed_input_schema::<crate::manga::MangaPipelineResumeInput>()
}
pub(crate) fn args_preview_start_manga_pipeline_request() -> Value {
    strict_typed_input_schema::<crate::manga::MangaPipelineStartInput>()
}
pub(crate) fn args_prepare_standalone_comic_source() -> Value {
    strict_typed_input_schema::<crate::manga::PrepareStandaloneComicSourceInput>()
}
pub(crate) fn args_preview_test_image_config_request() -> Value {
    json!({
        "type": "object",
        "properties": {
            "configId": {
                "type": "integer",
                "anyOf": [{ "const": -1 }, { "minimum": 1 }],
            },
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["configId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_push_assembled_chapter_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "workflowMode": { "type": "string", "minLength": 1 },
            "input": property_schema::<crate::video::PushAssembledChapterVideoInput>("input"),
        },
        "required": ["projectId", "chapterId", "workflowMode", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_reorder_galgame_lines() -> Value {
    project_typed_input_schema::<crate::galgame::ReorderGalgameLinesInput>()
}
pub(crate) fn args_run_canvas_graph() -> Value {
    strict_typed_input_schema::<crate::canvas::RunCanvasGraphInput>()
}
pub(crate) fn args_run_canvas_node() -> Value {
    strict_typed_input_schema::<crate::canvas::RunCanvasNodeInput>()
}
pub(crate) fn args_run_galgame_asset_generation() -> Value {
    strict_typed_input_schema::<crate::galgame::RunGalgameAssetGenerationInput>()
}
pub(crate) fn args_save_canvas_graph() -> Value {
    strict_typed_input_schema::<crate::canvas::SaveCanvasGraphInput>()
}
pub(crate) fn args_save_galgame_story_graph() -> Value {
    project_typed_input_schema::<crate::galgame::SaveGalgameStoryGraphInput>()
}
pub(crate) fn args_get_galgame_story_runtime_state() -> Value {
    project_typed_input_schema::<crate::galgame::GetGalgameStoryRuntimeInput>()
}
pub(crate) fn args_start_galgame_story_runtime() -> Value {
    project_typed_input_schema::<crate::galgame::StartGalgameStoryRuntimeInput>()
}
pub(crate) fn args_advance_galgame_story_runtime() -> Value {
    project_typed_input_schema::<crate::galgame::AdvanceGalgameStoryRuntimeInput>()
}
pub(crate) fn args_read_canvas_graph() -> Value {
    strict_typed_input_schema::<crate::canvas::ReadCanvasGraphInput>()
}
pub(crate) fn result_read_canvas_graph() -> Value {
    typed_schema::<crate::canvas::CanvasGraphReadResult>()
}
pub(crate) fn args_edit_canvas_graph() -> Value {
    strict_typed_input_schema::<crate::canvas::EditCanvasGraphInput>()
}
pub(crate) fn result_edit_canvas_graph() -> Value {
    typed_schema::<crate::canvas::CanvasGraphEditResult>()
}
pub(crate) fn args_save_canvas_text_asset() -> Value {
    strict_typed_input_schema::<crate::canvas::SaveCanvasTextAssetInput>()
}
pub(crate) fn args_list_canvas_text_assets() -> Value {
    strict_typed_input_schema::<crate::canvas::ListCanvasTextAssetsInput>()
}
pub(crate) fn args_list_canvas_text_asset_versions() -> Value {
    strict_typed_input_schema::<crate::canvas::ListCanvasTextAssetVersionsInput>()
}
pub(crate) fn args_canvas_text_asset_draft() -> Value {
    strict_typed_input_schema::<crate::canvas::CanvasTextAssetDraftInput>()
}
pub(crate) fn args_select_segment_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "videoId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<crate::video::SelectSegmentVideoInput>("input"),
        },
        "required": ["projectId", "chapterId", "videoId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_set_primary_portrait() -> Value {
    strict_typed_input_schema::<crate::portrait::SetPrimaryPortraitInput>()
}
pub(crate) fn args_start_galgame_pipeline() -> Value {
    strict_typed_input_schema::<crate::galgame::StartGalgamePipelineInput>()
}
pub(crate) fn args_start_image_generation() -> Value {
    strict_runtime_typed_input_schema::<crate::image_generation::StartImageGenerationInput>()
}
pub(crate) fn args_start_portrait_generation() -> Value {
    strict_runtime_typed_input_schema::<crate::portrait::StartPortraitGenerationInput>()
}
pub(crate) fn args_start_video_generation() -> Value {
    strict_runtime_typed_input_schema::<crate::video::StartVideoGenerationInput>()
}
pub(crate) fn args_unbind_galgame_media() -> Value {
    strict_typed_input_schema::<crate::galgame::UnbindGalgameMediaInput>()
}
pub(crate) fn args_update_chapter_video_prompt() -> Value {
    json!({
        "type": "object",
        "properties": {
            "promptId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<crate::video::UpdateChapterVideoPromptInput>("input"),
        },
        "required": ["promptId", "input"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_update_chapter_video_segment_plan() -> Value {
    json!({
        "type": "object",
        "properties": {
            "promptId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<crate::video::UpdateChapterVideoSegmentPlanInput>("input"),
        },
        "required": ["promptId", "input"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_update_chapter_video_segment_recipe() -> Value {
    json!({
        "type": "object",
        "properties": {
            "promptId": { "type": "integer", "minimum": 1 },
            "recipe": property_schema::<crate::video::VideoSegmentExecutionRecipeV1>("recipe"),
        },
        "required": ["promptId", "recipe"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_update_galgame_choices() -> Value {
    project_typed_input_schema::<crate::galgame::UpdateGalgameChoicesInput>()
}
pub(crate) fn args_update_galgame_line() -> Value {
    project_typed_input_schema::<crate::galgame::UpdateGalgameLineInput>()
}
pub(crate) fn args_update_galgame_scene() -> Value {
    project_typed_input_schema::<crate::galgame::UpdateGalgameSceneInput>()
}
pub(crate) fn args_update_image() -> Value {
    strict_typed_input_schema::<crate::image::UpdateGeneratedImageInput>()
}
pub(crate) fn args_read_image() -> Value {
    typed_schema::<crate::image_read::ReadImageInput>()
}
pub(crate) fn args_update_manga_prompt() -> Value {
    strict_typed_input_schema::<crate::manga::UpdateMangaPromptInput>()
}
pub(crate) fn args_trim_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "videoId": { "type": "integer", "minimum": 1 },
            "startSecs": { "type": "number", "minimum": 0 },
            "endSecs": { "type": "number", "minimum": 0 },
        },
        "required": ["videoId", "startSecs", "endSecs"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_update_assembly_push_status() -> Value {
    json!({
        "type": "object",
        "properties": {
            "videoId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<crate::video::UpdateAssemblyPushStatusInput>("input"),
        },
        "required": ["videoId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_update_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "videoId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<crate::video::UpdateGeneratedVideoInput>("input"),
        },
        "required": ["videoId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_update_video_segment_plan_draft_recipe() -> Value {
    strict_typed_input_schema::<crate::video::UpdateVideoSegmentPlanDraftRecipeInput>()
}
pub(crate) fn args_upload_assembly_insert_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "workflowMode": { "type": "string", "minLength": 1 },
            "input": property_schema::<crate::video::UploadAssemblyInsertVideoInput>("input"),
        },
        "required": ["projectId", "chapterId", "workflowMode", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_upload_chapter_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<crate::video::UploadChapterVideoInput>("input"),
        },
        "required": ["projectId", "chapterId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_upload_image() -> Value {
    strict_typed_input_schema::<crate::image::UploadImageInput>()
}
pub(crate) fn args_upload_portrait() -> Value {
    strict_typed_input_schema::<crate::portrait::UploadPortraitInput>()
}
pub(crate) fn args_upload_reference_media_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<crate::video::UploadReferenceMediaVideoInput>("input"),
        },
        "required": ["projectId", "chapterId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn result_video_segment_plan_draft() -> Value {
    typed_schema::<crate::video::VideoSegmentPlanDraftSummary>()
}
pub(crate) fn result_assembled_chapter_video_preview() -> Value {
    typed_schema::<crate::video::AssembledChapterVideoPreview>()
}
pub(crate) fn result_assembly_push() -> Value {
    typed_schema::<crate::video::AssemblyPushResult>()
}
pub(crate) fn result_bind_galgame_media() -> Value {
    typed_schema::<crate::galgame::GalgameMediaBindingSummary>()
}
pub(crate) fn result_bind_manga_reference_image() -> Value {
    typed_schema::<crate::manga::MangaPagePromptSummary>()
}
pub(crate) fn result_cancel_galgame_pipeline() -> Value {
    typed_schema::<crate::galgame::GalgamePipelineStatusSummary>()
}
pub(crate) fn result_cancel_manga_pipeline() -> Value {
    typed_schema::<i64>()
}
pub(crate) fn result_edit_chapter_shooting_script() -> Value {
    typed_schema::<crate::shooting_script::EditedChapterShootingScriptView>()
}
pub(crate) fn result_edit_chapter_shooting_script_progress() -> Value {
    typed_schema::<crate::shooting_script::EditedChapterShootingScriptProgressView>()
}
pub(crate) fn result_chapter_visual_plan() -> Value {
    typed_schema::<crate::chapter_visual_plan::ChapterVisualPlanView>()
}
pub(crate) fn result_chapter_visual_plan_set() -> Value {
    typed_schema::<crate::chapter_visual_plan::ChapterVisualPlanSetView>()
}
pub(crate) fn result_read_chapter_visual_plan() -> Value {
    typed_schema::<crate::chapter_visual_plan::ChapterVisualPlanReadResult>()
}
pub(crate) fn result_edit_chapter_visual_plan_draft() -> Value {
    typed_schema::<crate::chapter_visual_plan::ChapterVisualPlanEditResult>()
}
pub(crate) fn result_validate_chapter_visual_plan() -> Value {
    typed_schema::<crate::chapter_visual_plan::ChapterVisualPlanValidationResult>()
}
pub(crate) fn result_confirmed_chapter_visual_plan() -> Value {
    typed_schema::<crate::chapter_visual_plan::ConfirmedChapterVisualPlanView>()
}
pub(crate) fn result_get_canvas_graph() -> Value {
    typed_schema::<crate::canvas::CanvasGraphDocument>()
}
pub(crate) fn result_save_canvas_graph() -> Value {
    typed_schema::<crate::canvas::CanvasGraphSaveResult>()
}
pub(crate) fn result_save_galgame_story_graph() -> Value {
    typed_schema::<a4n_domain::galgame::StoryGraph>()
}
pub(crate) fn result_get_galgame_story_runtime_state() -> Value {
    typed_schema::<Option<crate::galgame::GalgameStoryRuntimeView>>()
}
pub(crate) fn result_galgame_story_runtime() -> Value {
    typed_schema::<crate::galgame::GalgameStoryRuntimeView>()
}
pub(crate) fn result_get_canvas_node_contracts() -> Value {
    typed_schema::<Vec<crate::canvas::CanvasNodeContract>>()
}
pub(crate) fn result_get_chapter_shooting_script() -> Value {
    typed_schema::<Option<crate::shooting_script::ChapterShootingScriptView>>()
}
pub(crate) fn result_get_chapter_shooting_script_unit_progress() -> Value {
    typed_schema::<Option<crate::shooting_script::ChapterShootingScriptUnitProgressView>>()
}
pub(crate) fn result_find_galgame_script() -> Value {
    typed_schema::<Option<crate::galgame::GalgameScriptDetail>>()
}
pub(crate) fn result_read_image() -> Value {
    typed_schema::<crate::image_read::ReadImageResult>()
}
pub(crate) fn result_get_manga_pipeline_status() -> Value {
    typed_schema::<crate::manga::MangaPipelineStatusSummary>()
}
pub(crate) fn result_list_manga_pipeline_statuses() -> Value {
    typed_schema::<Vec<crate::manga::MangaPipelineStatusSummary>>()
}
pub(crate) fn result_list_manga_pipeline_artifacts() -> Value {
    typed_schema::<Vec<crate::manga::MangaPipelineArtifactRecord>>()
}
pub(crate) fn result_manga_pipeline_artifact_deletion_preview() -> Value {
    typed_schema::<crate::manga::MangaPipelineArtifactDeletionPreview>()
}
pub(crate) fn result_manga_pipeline_artifact() -> Value {
    typed_schema::<crate::manga::MangaPipelineArtifactRecord>()
}
pub(crate) fn result_prepare_standalone_comic_source() -> Value {
    typed_schema::<crate::manga::MangaStandaloneSourceSummary>()
}
pub(crate) fn result_get_primary_portrait() -> Value {
    typed_schema::<Option<crate::portrait::PortraitSummary>>()
}
pub(crate) fn result_import_canvas_media() -> Value {
    typed_schema::<crate::canvas::ImportedCanvasMedia>()
}
pub(crate) fn result_insert_galgame_line() -> Value {
    typed_schema::<crate::galgame::GalgameLine>()
}
pub(crate) fn result_list_chapter_video_prompts() -> Value {
    typed_schema::<Vec<crate::video::ChapterVideoPromptSummary>>()
}
pub(crate) fn result_video_timeline() -> Value {
    typed_schema::<crate::video::VideoTimelineSummary>()
}
pub(crate) fn result_video_segment_execution_recipes() -> Value {
    typed_schema::<Vec<crate::video::VideoSegmentExecutionRecipeV1>>()
}
pub(crate) fn result_list_images_by_chapter() -> Value {
    typed_schema::<Vec<crate::image::GeneratedImageSummary>>()
}
pub(crate) fn result_list_manga_page_prompts() -> Value {
    typed_schema::<Vec<crate::manga::MangaPagePromptSummary>>()
}
pub(crate) fn result_list_portraits() -> Value {
    typed_schema::<Vec<crate::portrait::PortraitSummary>>()
}
pub(crate) fn result_list_videos_by_chapter() -> Value {
    typed_schema::<Vec<crate::video::GeneratedVideoSummary>>()
}
pub(crate) fn result_optimize_chapter_video_prompt() -> Value {
    typed_schema::<crate::video::ChapterVideoPromptSummary>()
}
pub(crate) fn result_plan_galgame_assets() -> Value {
    typed_schema::<crate::galgame::GalgameAssetPlan>()
}
pub(crate) fn result_get_galgame_asset_plan() -> Value {
    typed_schema::<Option<crate::galgame::GalgameAssetPlan>>()
}
pub(crate) fn result_optional_assembled_chapter_video_preview() -> Value {
    typed_schema::<Option<crate::video::AssembledChapterVideoPreview>>()
}
pub(crate) fn result_object_profile_image_prompt() -> Value {
    typed_schema::<crate::image_generation::ObjectProfileImagePromptOutput>()
}
pub(crate) fn result_scene_profile_image_prompt() -> Value {
    typed_schema::<crate::image_generation::SceneProfileImagePromptOutput>()
}
pub(crate) fn result_portrait_summary() -> Value {
    typed_schema::<crate::portrait::PortraitSummary>()
}
pub(crate) fn result_reorder_galgame_lines() -> Value {
    typed_schema::<Vec<crate::galgame::GalgameLine>>()
}
pub(crate) fn result_run_canvas_graph() -> Value {
    typed_schema::<crate::canvas::CanvasRunSummary>()
}
pub(crate) fn result_run_galgame_asset_generation() -> Value {
    typed_schema::<crate::galgame::GalgameAssetGenerationSummary>()
}
pub(crate) fn result_save_canvas_text_asset() -> Value {
    typed_schema::<crate::canvas::CanvasTextAssetSummary>()
}
pub(crate) fn result_canvas_text_assets() -> Value {
    typed_schema::<Vec<crate::canvas::CanvasTextAssetSummary>>()
}
pub(crate) fn result_canvas_text_asset_versions() -> Value {
    typed_schema::<Vec<crate::canvas::CanvasTextAssetVersionSummary>>()
}
pub(crate) fn result_canvas_text_asset_draft() -> Value {
    typed_schema::<crate::canvas::CanvasTextAssetDraftSummary>()
}
pub(crate) fn result_generated_video() -> Value {
    typed_schema::<crate::video::GeneratedVideoSummary>()
}
pub(crate) fn result_video_playback_data() -> Value {
    typed_schema::<crate::video::VideoPlaybackData>()
}
pub(crate) fn result_unbind_galgame_media() -> Value {
    typed_schema::<u64>()
}
pub(crate) fn result_update_galgame_choices() -> Value {
    typed_schema::<Vec<crate::galgame::GalgameChoice>>()
}
pub(crate) fn result_update_galgame_scene() -> Value {
    typed_schema::<crate::galgame::GalgameScene>()
}
pub(crate) fn result_update_image() -> Value {
    typed_schema::<crate::image::GeneratedImageSummary>()
}
