//! 首帧链提示词与尾帧图片匹配规则（无 DB、无 Host）。

use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_domain::video_prompt::ChapterVideoPromptRecord;

use super::{
    PromptChainFields, prompt_chain_matches_for_initial_frame_image, prompt_source_type_matches,
};

pub fn first_frame_prompt_matches_video_prompt<P: PromptChainFields>(
    video_prompt: &ChapterVideoPromptRecord,
    first_frame_prompt: &P,
) -> bool {
    prompt_chain_matches_for_initial_frame_image(
        chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN,
        video_prompt,
        first_frame_prompt,
    ) && prompt_uses_shared_segment_source(video_prompt)
        && prompt_uses_shared_segment_source(first_frame_prompt)
}

fn prompt_uses_shared_segment_source<P: PromptChainFields>(prompt: &P) -> bool {
    prompt_source_type_matches(prompt, chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN)
}

#[cfg(test)]
mod tests {
    use a4n_video_contract::VIDEO_EXECUTION_CONTRACT_EPOCH;
    use chrono::Utc;
    use serde_json::json;

    use super::*;
    use crate::video::FIRST_FRAME_CHAIN_CONTRACT_VERSION;

    fn prompt(id: i64, prompt_type: &str, duration_secs: i32) -> ChapterVideoPromptRecord {
        let now = Utc::now().naive_utc();
        ChapterVideoPromptRecord {
            id,
            project_id: 1,
            chapter_id: 2,
            style: Some("测试风格".to_string()),
            prompt_type: prompt_type.to_string(),
            workflow_mode: chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN.to_string(),
            shot_number: 1,
            title: Some("测试提示词".to_string()),
            scene_summary: None,
            visual_prompt: "测试提示词".to_string(),
            motion_prompt: None,
            negative_prompt: None,
            duration_secs,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: None,
            status: "pending".to_string(),
            generated_video_id: None,
            timeline_item_id: None,
            metadata_json: Some(json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
                "video_execution_identity": {
                    "contract_epoch": VIDEO_EXECUTION_CONTRACT_EPOCH,
                    "source_content_sha256": "source",
                    "style_sha256": "style",
                    "duration_secs": duration_secs,
                    "aspect_ratio": "16:9",
                    "segment_plan_prompt_id": 13,
                    "segment_number": 1,
                    "source_shot_id": "shot-1",
                    "segment_index_in_shot": 1,
                    "segment_sha256": "segment"
                }
            })),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn initial_frame_image_survives_duration_only_video_change() {
        let video_prompt = prompt(15, chapter_video_prompt::PROMPT_TYPE_VIDEO, 7);
        let first_frame_prompt = prompt(14, chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME, 6);

        assert!(first_frame_prompt_matches_video_prompt(
            &video_prompt,
            &first_frame_prompt
        ));
    }
}
