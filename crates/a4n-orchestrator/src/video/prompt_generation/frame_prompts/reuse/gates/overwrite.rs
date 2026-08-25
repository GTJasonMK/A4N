use std::collections::{BTreeMap, HashSet};

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;

use crate::video::prompt_json::{ChatVideoPromptShot, VideoSegmentPlan};

pub(in crate::video::prompt_generation::frame_prompts) fn ensure_video_prompts_can_regenerate(
    rows: &[chapter_video_prompt::Model],
    label: &str,
) -> Result<(), AppError> {
    if let Some(row) = rows.iter().find(|row| row.generated_video_id.is_some()) {
        return Err(AppError::InvalidArgument(format!(
            "已存在由{label} #{} 生成的视频，不能重新生成视频提示词；请先删除对应视频",
            row.shot_number
        )));
    }
    Ok(())
}

pub(in crate::video::prompt_generation::frame_prompts) fn ensure_chat_video_prompts_can_generate_without_overwriting(
    all_existing_rows: &[chapter_video_prompt::Model],
    matching_rows: &[chapter_video_prompt::Model],
    reusable_prompts: &BTreeMap<i32, ChatVideoPromptShot>,
    segment_plan: &[VideoSegmentPlan],
    regenerate_existing: bool,
    generation_limit: usize,
) -> Result<(), AppError> {
    let mut allowed_reused_generated_row_ids = HashSet::new();
    let mut seen_matching_shots = HashSet::new();
    for row in matching_rows {
        if !seen_matching_shots.insert(row.shot_number) {
            continue;
        }
        let Some(reusable) = reusable_prompts.get(&row.shot_number) else {
            continue;
        };
        if !regenerate_existing
            && row.generated_video_id.is_some()
            && !reusable.chat_prompt.is_empty()
        {
            allowed_reused_generated_row_ids.insert(row.id);
        }
    }
    let mut generated_shots = HashSet::new();
    let mut blocked_generated_shots = HashSet::new();
    for row in all_existing_rows {
        if row.generated_video_id.is_some() {
            generated_shots.insert(row.shot_number);
            if !allowed_reused_generated_row_ids.contains(&row.id) {
                blocked_generated_shots.insert(row.shot_number);
            }
        }
    }
    let mut generated_this_run = 0usize;
    for segment in segment_plan {
        let shot_number = segment.segment_number;
        let reusable = reusable_prompts.get(&shot_number);
        if reusable.is_some() && (!regenerate_existing || generated_this_run >= generation_limit) {
            if blocked_generated_shots.contains(&shot_number) {
                return Err(AppError::InvalidArgument(format!(
                    "已存在由Chat 视频提示词 #{} 生成的视频，不能重新生成视频提示词；请先删除对应视频",
                    shot_number
                )));
            }
            continue;
        } else if reusable.is_some()
            && regenerate_existing
            && generated_this_run >= generation_limit
        {
            continue;
        }

        if generated_this_run >= generation_limit {
            continue;
        }
        if generated_shots.contains(&shot_number) {
            return Err(AppError::InvalidArgument(format!(
                "已存在由Chat 视频提示词 #{} 生成的视频，不能重新生成视频提示词；请先删除对应视频",
                shot_number
            )));
        }
        generated_this_run += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::prompt_json::ChatVideoPromptShot;
    use a4n_core::models::chapter::chapter_video_prompt;
    use chrono::Utc;
    use serde_json::json;

    fn row(id: i64, shot_number: i32, generated: bool) -> chapter_video_prompt::Model {
        let now = Utc::now().naive_utc();
        chapter_video_prompt::Model {
            id,
            project_id: 1,
            chapter_id: 1,
            style: None,
            prompt_type: "video_prompt".to_string(),
            workflow_mode: "chat_video".to_string(),
            shot_number,
            title: None,
            scene_summary: None,
            visual_prompt: String::new(),
            motion_prompt: None,
            negative_prompt: None,
            duration_secs: 5,
            aspect_ratio: None,
            reference_image_ids_json: None,
            status: "completed".to_string(),
            generated_video_id: generated.then_some(99),
            timeline_item_id: None,
            metadata_json: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn reusable(shot_number: i32) -> ChatVideoPromptShot {
        ChatVideoPromptShot {
            shot_number,
            title: String::new(),
            scene_summary: String::new(),
            chat_prompt: "reuse".to_string(),
            negative_prompt: String::new(),
            chat_video: json!({}),
            duration_secs: 5,
            aspect_ratio: "16:9".to_string(),
            reference_image_ids: Vec::new(),
        }
    }

    fn segment(shot_number: i32) -> VideoSegmentPlan {
        serde_json::from_value(json!({
            "schema_version": a4n_core::video_execution::VIDEO_SEGMENT_PLAN_V2,
            "segment_number": shot_number,
            "source_shot_id": format!("shot-{shot_number}"),
            "segment_index_in_shot": 1,
            "shot_segment_count": 1,
            "covered_beat_ids": [],
            "title": "测试片段",
            "scene_summary": "测试摘要",
            "blocking_summary": "测试站位",
            "spatial_anchors": [],
            "emotion_action_goals": [],
            "video_intent": {
                "hook": "测试",
                "audience_effect": "测试",
                "rhythm": "测试",
                "visual_priority": "测试"
            },
            "emotional_arc": {
                "start": "测试",
                "turn": "测试",
                "end": "测试",
                "function": "测试"
            },
            "performance_plan": {
                "movement_path": "测试",
                "body_performance": "测试",
                "gaze_and_hands": "测试",
                "pause_or_reaction": "测试",
                "camera_relationship": "测试",
                "time_blocks": [],
                "action_script": []
            },
            "material_coverage": {
                "required_entities": [],
                "missing_or_weak_references": [],
                "risk_note": "测试"
            },
            "transition_to_next": null,
            "source_span_ids": [],
            "duration_secs": 5,
            "aspect_ratio": "16:9",
            "reference_image_ids": []
        }))
        .expect("minimal video segment fixture should deserialize")
    }

    #[test]
    fn allows_only_reused_generated_row_and_blocks_mixed_duplicates() {
        let allowed = row(1, 1, true);
        let blocked = row(2, 1, true);
        let reusable_prompts = BTreeMap::from([(1, reusable(1))]);

        let error = ensure_chat_video_prompts_can_generate_without_overwriting(
            &[allowed.clone(), blocked],
            &[allowed],
            &reusable_prompts,
            &[segment(1)],
            false,
            1,
        )
        .expect_err("a non-reused duplicate must remain blocking");
        assert!(error.to_string().contains("#1"));
    }

    #[test]
    fn allows_reused_generated_row_without_scanning_rows_per_segment() {
        let existing = row(1, 1, true);
        let reusable_prompts = BTreeMap::from([(1, reusable(1))]);
        let segment = segment(1);

        ensure_chat_video_prompts_can_generate_without_overwriting(
            std::slice::from_ref(&existing),
            std::slice::from_ref(&existing),
            &reusable_prompts,
            &[segment],
            false,
            1,
        )
        .expect("the matching reused row is explicitly allowed");
    }
}
