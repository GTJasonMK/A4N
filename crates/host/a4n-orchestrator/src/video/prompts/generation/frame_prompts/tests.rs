mod assets;
mod boundary_validation;
mod database;
mod reuse;
mod source_and_units;

use super::*;
use a4n_db::project::{
    CreateDerivedProjectParams, CreateProjectRecordInput, DerivedChapterScope, PROJECT_KIND_VIDEO,
    create_derived_project_record, create_project_record,
};
use a4n_db_models::models::ai::generated_image;
use a4n_db_models::models::chapter::{chapter, chapter_video_prompt};
use a4n_domain::image::IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT;
use sea_orm::{ActiveModelTrait, Set};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

const PROJECT_TYPE_NOVEL_LONG: &str = "novel_long";

fn source_span(id: &str, paragraph_index: usize) -> SourceSpan {
    SourceSpan {
        id: id.to_string(),
        chapter_id: 1,
        chapter_number: 1,
        paragraph_index,
        sentence_index: 1,
        fragment_index: 1,
        text: id.to_string(),
        char_start: 0,
        char_end: id.chars().count(),
    }
}

fn frame(frame_number: i32, reference_image_ids: Vec<i64>) -> VideoFramePrompt {
    VideoFramePrompt {
        frame_number,
        title: format!("边界帧 {frame_number}"),
        scene_summary: format!("画面状态 {frame_number}"),
        storyboard_panels: Vec::new(),
        frame_prompt: format!("提示词 {frame_number}"),
        blocking_state: json!({}),
        lighting_state: json!({}),
        negative_prompt: "低质量".to_string(),
        duration_secs: 5,
        aspect_ratio: "16:9".to_string(),
        reference_image_ids,
        segment_numbers: vec![frame_number],
    }
}

pub(super) fn segment(segment_number: i32) -> VideoSegmentPlan {
    use a4n_video_contract::*;
    VideoSegmentPlan {
        director_constraints: None,
        schema_version: VIDEO_SEGMENT_PLAN_V2.to_string(),
        segment_number,
        source_shot_id: format!("shot-{segment_number}"),
        segment_index_in_shot: 1,
        shot_segment_count: 1,
        covered_beat_ids: vec![format!("shot-{segment_number}-beat-1")],
        title: format!("片段 {segment_number}"),
        scene_summary: format!("片段摘要 {segment_number}"),
        blocking_summary: "站位".to_string(),
        spatial_anchors: vec![VideoSpatialAnchorV1 {
            anchor: "门口".to_string(),
            usage: "固定空间参照".to_string(),
        }],
        emotion_action_goals: vec![VideoEmotionActionGoalV1 {
            character: "角色".to_string(),
            emotion: "克制".to_string(),
            visible_action: "停在门口".to_string(),
        }],
        video_intent: VideoIntentV1 {
            hook: "停顿".to_string(),
            audience_effect: "看清犹豫".to_string(),
            rhythm: "克制".to_string(),
            visual_priority: "手部和视线".to_string(),
        },
        emotional_arc: VideoEmotionalArcV1 {
            start: "迟疑".to_string(),
            turn: "抬眼".to_string(),
            end: "决定行动".to_string(),
            function: "完成情绪转折".to_string(),
        },
        performance_plan: VideoPerformancePlanV2 {
            movement_path: "门外到门口".to_string(),
            body_performance: "重心前移".to_string(),
            gaze_and_hands: "视线看向门内，手停在门把附近".to_string(),
            pause_or_reaction: "停半拍".to_string(),
            camera_relationship: "固定中景观察".to_string(),
            time_blocks: vec![VideoPerformanceTimeBlockV1 {
                start_sec: 0.0,
                end_sec: 5.0,
                camera: "固定中景".to_string(),
                subject_action: "角色停在门口并抬眼".to_string(),
                continuity_target: "稳定在门口".to_string(),
            }],
            action_script: vec![VideoPerformanceActionV1 {
                character: "角色".to_string(),
                body: "重心前移".to_string(),
                movement: "走到门口停住".to_string(),
                hands: "手停在门把附近".to_string(),
                gaze: "看向门内".to_string(),
                timing: "5 秒内完成".to_string(),
            }],
        },
        material_coverage: VideoMaterialCoverageV1 {
            required_entities: vec!["角色".to_string(), "门".to_string()],
            missing_or_weak_references: Vec::new(),
            risk_note: "无".to_string(),
        },
        transition_to_next: None,
        source_span_ids: vec![format!("s{segment_number}")],
        duration_secs: 5,
        aspect_ratio: "16:9".to_string(),
        reference_image_ids: vec![3, 9],
    }
}

pub(super) fn shooting_script_projection() -> VideoShootingScriptProjection {
    serde_json::from_value(json!({
        "schema_version": a4n_video_contract::VIDEO_EXECUTION_PROJECTION_V2,
        "contract_epoch": a4n_video_contract::VIDEO_EXECUTION_CONTRACT_EPOCH,
        "shooting_script_identity": {
            "record_id": 1,
            "revision": 1,
            "source_revision_key": "test-revision",
            "source_content_sha256": "current-source",
            "script_sha256": "test-script"
        },
        "directing_statement": {
            "central_dramatic_question": "测试问题",
            "protagonist_throughline": "测试主线",
            "audience_experience": "测试观众体验",
            "viewpoint_strategy": "限制视点",
            "visual_strategy": "稳定空间",
            "spatial_strategy": "保持轴线",
            "sound_strategy": "环境声",
            "rhythm_strategy": "克制节奏"
        },
        "scenes": [],
        "planning_quality": {
            "total_estimated_seconds": 0.0,
            "total_recommended_segments": 0,
            "total_minimum_readable_segments": 0,
            "risk_flags": [],
            "directing_audit": {
                "total_directing_beats": 0,
                "must_show_beats": 0,
                "directing_order_preserved": true,
                "viewpoint_metadata_complete": true,
                "audience_causality_complete": true
            }
        }
    }))
    .expect("test shooting script projection should deserialize")
}

fn segment_plan_row(id: i64, source_hash: Option<&str>) -> chapter_video_prompt::Model {
    let now = chrono::Utc::now().naive_utc();
    let mut segment = segment(1);
    segment.reference_image_ids.clear();
    let projection = shooting_script_projection();
    let execution_identity = video_execution_chain_identity(
        &projection,
        std::slice::from_ref(&segment),
        "测试风格",
        5,
        "16:9",
    )
    .unwrap();
    let recipes = a4n_video_contract::default_video_segment_recipes(
        std::slice::from_ref(&segment),
        "测试风格",
        "zh_cn",
    )
    .unwrap();
    let metadata_json = Some(match source_hash {
        Some(hash) => json!({
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "video_execution_identity": execution_identity,
            "segments": [segment.clone()],
            "segment_recipes": recipes,
            "plan_complete": true,
            "source": {
                "content_sha256": hash
            }
        }),
        None => json!({
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "video_execution_identity": execution_identity,
            "segments": [segment.clone()],
            "segment_recipes": recipes,
            "plan_complete": true
        }),
    });

    chapter_video_prompt::Model {
        id,
        project_id: 2,
        chapter_id: 1,
        style: Some("测试风格".to_string()),
        prompt_type: "frame_segment_plan".to_string(),
        workflow_mode: PromptWorkflowMode::SharedSegmentPlan.as_str().to_string(),
        shot_number: 0,
        title: Some("视频片段规划".to_string()),
        scene_summary: None,
        visual_prompt: json!({ "segments": [segment] }).to_string(),
        motion_prompt: None,
        negative_prompt: None,
        duration_secs: 5,
        aspect_ratio: Some("16:9".to_string()),
        reference_image_ids_json: Some(json!([])),
        status: "completed".to_string(),
        generated_video_id: None,
        timeline_item_id: None,
        metadata_json,
        created_at: now,
        updated_at: now,
    }
}

pub(super) fn boundary_frame(
    frame_number: i32,
    reference_image_ids: Vec<i64>,
    segment_numbers: Vec<i32>,
) -> VideoFramePrompt {
    let mut frame = frame(frame_number, reference_image_ids);
    frame.segment_numbers = segment_numbers;
    frame
}

pub(super) fn row_for_boundary_frame(
    id: i64,
    frame: VideoFramePrompt,
) -> chapter_video_prompt::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter_video_prompt::Model {
        id,
        project_id: 2,
        chapter_id: 1,
        style: Some("测试风格".to_string()),
        prompt_type: "frame_prompt".to_string(),
        workflow_mode: PromptWorkflowMode::BoundaryFrames.as_str().to_string(),
        shot_number: frame.frame_number,
        title: Some(frame.title.clone()),
        scene_summary: Some(frame.scene_summary.clone()),
        visual_prompt: frame.frame_prompt.clone(),
        motion_prompt: None,
        negative_prompt: Some(frame.negative_prompt.clone()),
        duration_secs: frame.duration_secs,
        aspect_ratio: Some(frame.aspect_ratio.clone()),
        reference_image_ids_json: Some(json!(frame.reference_image_ids.clone())),
        status: "pending".to_string(),
        generated_video_id: None,
        timeline_item_id: None,
        metadata_json: Some(json!({
            "boundary_frame": frame
        })),
        created_at: now,
        updated_at: now,
    }
}
