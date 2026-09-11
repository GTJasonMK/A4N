//! Storyboard 提示词链与生成图片匹配规则（无 DB、无 Host）。

use a4n_db_models::models::ai::generated_image;
use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_domain::image::{
    IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT, image_generation_prompt_id,
    image_generation_source,
};

use super::{PromptChainFields, prompt_chain_matches, prompt_source_type_matches};

pub fn storyboard_prompt_matches_video_prompt<P: PromptChainFields>(
    video_prompt: &chapter_video_prompt::Model,
    storyboard_prompt: &P,
) -> bool {
    prompt_chain_matches(
        chapter_video_prompt::WORKFLOW_MODE_STORYBOARD,
        video_prompt,
        storyboard_prompt,
    ) && prompt_source_type_matches(
        video_prompt,
        chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
    ) && prompt_source_type_matches(
        storyboard_prompt,
        chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
    )
}

pub fn latest_storyboard_image_id<P: PromptChainFields>(
    images: &[generated_image::Reference],
    storyboard_prompt: &P,
) -> Option<i64> {
    images
        .iter()
        .find(|image| {
            let metadata = image.metadata_json.as_ref();
            image_generation_prompt_id(metadata) == Some(storyboard_prompt.id())
                && image_generation_source(metadata)
                    == Some(IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT)
        })
        .map(|image| image.id)
}

#[cfg(test)]
mod tests {
    use a4n_video_contract::VIDEO_EXECUTION_CONTRACT_EPOCH;
    use chrono::Utc;
    use serde_json::json;

    use super::*;
    use crate::video::{
        STORYBOARD_PROMPT_CONTRACT_VERSION, STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION,
    };

    fn prompt(id: i64, prompt_type: &str, contract_version: &str) -> chapter_video_prompt::Model {
        let now = Utc::now().naive_utc();
        let recipe_fingerprint = if prompt_type == "video_prompt" {
            "video-recipe"
        } else {
            "frame-recipe"
        };
        chapter_video_prompt::Model {
            id,
            project_id: 1,
            chapter_id: 2,
            style: None,
            prompt_type: prompt_type.to_string(),
            workflow_mode: "storyboard".to_string(),
            shot_number: 3,
            title: None,
            scene_summary: None,
            visual_prompt: "测试提示词".to_string(),
            motion_prompt: None,
            negative_prompt: None,
            duration_secs: 4,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: None,
            status: "pending".to_string(),
            generated_video_id: None,
            timeline_item_id: None,
            metadata_json: Some(json!({
                "contract_version": contract_version,
                "source_prompt_type": chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
                "video_execution_identity": {
                    "contract_epoch": VIDEO_EXECUTION_CONTRACT_EPOCH,
                    "shooting_script_identity": "script",
                    "projection_sha256": "projection",
                    "segment_plan_sha256": "segment",
                    "unit_plan_sha256": "unit",
                    "source_content_sha256": "source",
                    "style_sha256": "style",
                    "duration_secs": 4,
                    "aspect_ratio": "16:9",
                    "segment_plan_prompt_id": 9,
                    "segment_number": 3,
                    "source_shot_id": "shot-3",
                    "segment_index_in_shot": 1,
                    "segment_sha256": "segment-3",
                    "segment_recipe_fingerprint": recipe_fingerprint
                }
            })),
            created_at: now,
            updated_at: now,
        }
    }

    fn image(
        id: i64,
        source_prompt_id: i64,
        generation_source: &str,
    ) -> generated_image::Reference {
        generated_image::Reference {
            id,
            image_type: "storyboard".to_string(),
            metadata_json: Some(json!({
                "generation_context": {
                    "prompt_id": source_prompt_id,
                    "source": generation_source
                }
            })),
        }
    }

    #[test]
    fn storyboard_prompt_uses_stable_segment_chain_and_source() {
        let video = prompt(10, "video_prompt", STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION);
        let storyboard = prompt(11, "storyboard_prompt", STORYBOARD_PROMPT_CONTRACT_VERSION);

        assert!(storyboard_prompt_matches_video_prompt(&video, &storyboard));
        let candidate = a4n_db::video::ChapterVideoPromptCandidateRecord {
            id: storyboard.id,
            prompt_type: storyboard.prompt_type.clone(),
            workflow_mode: storyboard.workflow_mode.clone(),
            shot_number: storyboard.shot_number,
            metadata_json: storyboard.metadata_json.clone(),
        };
        assert!(storyboard_prompt_matches_video_prompt(&video, &candidate));

        let mut stale = storyboard.clone();
        stale.metadata_json.as_mut().unwrap()["video_execution_identity"]["segment_sha256"] =
            json!("stale-segment");
        assert!(!storyboard_prompt_matches_video_prompt(&video, &stale));

        let mut wrong_source = storyboard;
        wrong_source.metadata_json.as_mut().unwrap()["source_prompt_type"] = json!("legacy");
        assert!(!storyboard_prompt_matches_video_prompt(
            &video,
            &wrong_source
        ));
    }

    #[test]
    fn storyboard_image_selection_uses_prompt_and_generation_source() {
        let storyboard = prompt(11, "storyboard_prompt", STORYBOARD_PROMPT_CONTRACT_VERSION);
        let images = vec![
            image(20, storyboard.id, "other"),
            image(21, 999, IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT),
            image(
                22,
                storyboard.id,
                IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT,
            ),
        ];

        assert_eq!(latest_storyboard_image_id(&images, &storyboard), Some(22));
    }
}
