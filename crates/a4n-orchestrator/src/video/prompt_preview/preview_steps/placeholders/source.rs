use a4n_core::error::AppError;
use a4n_core::video_execution::{
    VIDEO_SEGMENT_PLAN_V2, VideoEmotionActionGoalV1, VideoEmotionalArcV1, VideoIntentV1,
    VideoMaterialCoverageV1, VideoPerformanceActionV1, VideoPerformancePlanV2,
    VideoPerformanceTimeBlockV1, VideoSpatialAnchorV1,
};
use serde_json::{Value, json};

use crate::video::prompt_json::VideoSegmentPlan;

pub(in crate::video::prompt_preview) fn placeholder_segments() -> Vec<VideoSegmentPlan> {
    vec![VideoSegmentPlan {
        schema_version: VIDEO_SEGMENT_PLAN_V2.to_string(),
        segment_number: 1,
        source_shot_id: "placeholder-shot-1".to_string(),
        segment_index_in_shot: 1,
        shot_segment_count: 1,
        covered_beat_ids: vec!["placeholder-beat-1".to_string()],
        title: "占位片段标题".to_string(),
        scene_summary: "占位：这里会替换为真实片段剧情摘要。".to_string(),
        blocking_summary: "占位：这里会替换为真实人物站位、移动目标和空间关系。".to_string(),
        spatial_anchors: vec![VideoSpatialAnchorV1 {
            anchor: "占位固定参考物".to_string(),
            usage: "占位：约束人物位置和移动路径。".to_string(),
        }],
        emotion_action_goals: vec![VideoEmotionActionGoalV1 {
            character: "占位人物".to_string(),
            emotion: "占位情绪".to_string(),
            visible_action: "占位可见动作".to_string(),
        }],
        video_intent: VideoIntentV1 {
            hook: "占位：这里会替换为真实片段钩子。".to_string(),
            audience_effect: "占位：这里会替换为观众感受目标。".to_string(),
            rhythm: "占位节奏".to_string(),
            visual_priority: "占位：这里会替换为最重要的可见主体或动作。".to_string(),
        },
        emotional_arc: VideoEmotionalArcV1 {
            start: "占位起始情绪".to_string(),
            turn: "占位情绪转折".to_string(),
            end: "占位结尾情绪".to_string(),
            function: "占位段落功能".to_string(),
        },
        performance_plan: VideoPerformancePlanV2 {
            movement_path: "占位：这里会替换为真实起点、移动方向、终点和空间锚点。".to_string(),
            body_performance: "占位：这里会替换为身体重心、姿态变化、步幅或停顿。".to_string(),
            gaze_and_hands: "占位：这里会替换为视线落点、手部动作和道具状态。".to_string(),
            pause_or_reaction: "占位：这里会替换为关键停顿、反应或情绪余韵。".to_string(),
            camera_relationship: "占位：这里会替换为机位、景别、镜头运动与表演关系。".to_string(),
            time_blocks: vec![VideoPerformanceTimeBlockV1 {
                start_sec: 0.0,
                end_sec: 6.0,
                camera: "占位镜头关系".to_string(),
                subject_action: "占位主体动作".to_string(),
                continuity_target: "占位可续接尾态".to_string(),
            }],
            action_script: vec![VideoPerformanceActionV1 {
                character: "占位人物".to_string(),
                body: "占位身体变化".to_string(),
                movement: "占位移动方向".to_string(),
                hands: "占位手部动作".to_string(),
                gaze: "占位视线变化".to_string(),
                timing: "全段".to_string(),
            }],
        },
        material_coverage: VideoMaterialCoverageV1 {
            required_entities: vec!["占位人物".to_string(), "占位场景".to_string()],
            missing_or_weak_references: vec!["占位人物".to_string(), "占位场景".to_string()],
            risk_note: "占位：真实生成时这里会记录素材覆盖风险。".to_string(),
        },
        transition_to_next: None,
        source_span_ids: vec!["ch1:s1:f1".to_string()],
        duration_secs: 6,
        aspect_ratio: "16:9".to_string(),
        reference_image_ids: Vec::new(),
    }]
}

pub(in crate::video::prompt_preview) fn first_cinematic_shot_for_preview(
    shooting_script_projection: &Value,
) -> Result<Value, AppError> {
    shooting_script_projection
        .pointer("/scenes/0/shots/0")
        .cloned()
        .ok_or_else(|| {
            AppError::InvalidArgument("视频提示词预览缺少 current_shot 上下文".to_string())
        })
}

pub(in crate::video::prompt_preview) fn placeholder_segment_plan() -> Value {
    json!({
        "preview_placeholder": true,
        "placeholder_source": "chapter_video_segment_plan",
        "note": "真实生成时这里会替换为上一步视频片段规划模型输出。",
        "segments": placeholder_segments()
    })
}
