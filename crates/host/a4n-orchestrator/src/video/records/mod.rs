use a4n_domain::media::GeneratedVideoRecord;
use a4n_domain::video_prompt::ChapterVideoPromptRecord;
use a4n_error::AppError;
use std::collections::{HashMap, HashSet};

use a4n_domain::source_spans::metadata_source_content_sha256;
use a4n_domain::{
    AssemblyPostprocessOperation, CanvasFrameKind, EXTERNAL_IMPORT_MODEL,
    image::{
        IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE, IMAGE_TYPE_CANVAS_FRAME,
        IMAGE_TYPE_VIDEO_TAIL_FRAME,
    },
    source_text_hash,
    video_workflow::{
        WORKFLOW_MODE_CANVAS_ASSEMBLY, WORKFLOW_MODE_CANVAS_TRIM, WORKFLOW_MODE_CANVAS_VIDEO,
        WORKFLOW_MODE_MIXED, is_assembly_video_workflow_mode, is_generated_video_workflow_mode,
    },
};
use a4n_provider_contract::VideoProviderConfig;
use a4n_video::{
    assembly_media_properties, canvas_frame_timestamp, imported_chapter_video_metadata,
    tail_frame_metadata_belongs_to_video, trimmed_video_metadata, validate_video_trim_range,
};
use a4n_video_contract::VideoSegmentWorkflowModeV1;
use a4n_video_host_contract::{
    ImportedVideoProperties, VideoProviderHostPort, VideoProviderPollTarget,
};
use chrono::Utc;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use a4n_api_contract::video::{
    AssemblyPushAction, SelectSegmentVideoInput, UpdateAssemblyPushStatusInput,
    UpdateGeneratedVideoInput,
};
use a4n_api_contract::{
    image::GeneratedImageSummary,
    video::{
        AssemblyClipRecord, AssemblyPostprocessEditRecordInput, AssemblyPostprocessPlan,
        AssemblyPostprocessSourcePlan, AssemblyPushClipInput, AssemblyVideoImportApplicationPort,
        AssemblyVideoRecords, CanvasAssemblyPlan, CanvasFramePlan,
        CanvasVideoImportApplicationPort, ChapterVideoImportApplicationPort,
        CompleteVideoRecordInput, GeneratedVideoSummary, PreparedAssemblyPostprocessEditRecord,
        PreparedAssemblyPushRecord, ReferenceMediaVideoImportApplicationPort,
        VideoAssemblyPlanningApplicationPort, VideoAssemblyPostprocessApplicationPort,
        VideoAssemblyPushApplicationPort, VideoCanvasMediaApplicationPort,
        VideoImportRecordSummary, VideoProviderExecutionApplicationPort, VideoRecordCompletion,
        VideoRecordCompletionApplicationPort, VideoRecordCompletionError,
        VideoRecordFailureApplicationPort, VideoRecordLifecycleApplicationPort,
        VideoRecordMutationApplicationPort, VideoRecordReadApplicationPort,
        VideoTailFrameApplicationPort, VideoTailFrameInput, VideoTailFrameSummary,
        VideoTrimApplicationPort, VideoTrimPlan,
    },
};

use super::{
    ASSEMBLY_PUSH_STATUS_ACCEPTED, ASSEMBLY_PUSH_STATUS_PENDING, ASSEMBLY_PUSH_STATUS_REJECTED,
    AssemblyClipOrderAnchor, active_assembly_push_source_ids, assembly_clip_display_shot_number,
    assembly_clip_insert_after_video_id, assembly_clip_sort_order,
    assembly_insert_metadata_with_position, assembly_push_metadata_with_status,
    assembly_push_records, ensure_insert_position_not_inside_pushed_prefix,
    is_assembly_clip_video_type, is_canvas_video_record, latest_non_rejected_push_index,
    next_assembly_insert_sort_order, parse_assembly_push_metadata, parse_postprocess_edit_metadata,
    postprocess_edit_timing, selected_assembly_push_end_index, validate_assembly_workflow_mode,
    validate_reference_media_import_options, video_shot_number,
};

#[derive(Debug, Clone)]
pub(crate) struct PreparedImportedChapterVideoRecord {
    pub video: GeneratedVideoRecord,
    pub prompt: ChapterVideoPromptRecord,
}

#[derive(Debug, Clone)]
pub struct VideoGenerationRecordInput {
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub video_type: String,
    pub workflow_mode: Option<String>,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
    pub source_video_id: Option<i64>,
    pub source_image_id: Option<i64>,
    pub metadata_json: Option<Value>,
    pub provider_request_snapshot: Value,
    pub provider_backend: String,
    pub step_key: String,
    pub title: String,
    pub prompt_template_name: Option<String>,
    pub prompt_id: Option<i64>,
}

pub(crate) fn generated_video_summary(video: &GeneratedVideoRecord) -> GeneratedVideoSummary {
    GeneratedVideoSummary {
        id: video.id,
        project_id: video.project_id,
        chapter_id: video.chapter_id,
        video_type: video.video_type.clone(),
        workflow_mode: video.workflow_mode.clone(),
        file_path: video.file_path.clone(),
        thumbnail_path: video.thumbnail_path.clone(),
        prompt: video.prompt.clone(),
        negative_prompt: video.negative_prompt.clone(),
        model: video.model.clone(),
        width: video.width,
        height: video.height,
        duration_secs: video.duration_secs,
        fps: video.fps,
        file_size: video.file_size,
        source_video_id: video.source_video_id,
        source_image_id: video.source_image_id,
        status: video.status.clone(),
        provider_job_id: video.provider_job_id.clone(),
        error_message: video.error_message.clone(),
        metadata_json: video.metadata_json.clone(),
        timeline_item_id: video.timeline_item_id,
        is_favorite: video.is_favorite,
        created_at: video.created_at.and_utc().to_rfc3339(),
        updated_at: video.updated_at.and_utc().to_rfc3339(),
    }
}

fn video_import_record_summary(
    video: &GeneratedVideoRecord,
    prompt_id: Option<i64>,
) -> VideoImportRecordSummary {
    VideoImportRecordSummary {
        id: video.id,
        project_id: video.project_id,
        chapter_id: video.chapter_id,
        video_type: video.video_type.clone(),
        workflow_mode: video.workflow_mode.clone(),
        prompt_id,
    }
}

mod assembly;
mod imports;
mod lifecycle;
mod ports;

pub(super) use helpers::*;
mod helpers;

#[cfg(test)]
mod tests;
