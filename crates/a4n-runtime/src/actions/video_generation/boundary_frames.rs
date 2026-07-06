use super::prompt_chain::{
    image_generation_prompt_id, image_generation_source, prompt_chain_matches,
    prompt_source_type_matches,
};
use crate::AppState;
use a4n_core::models::ai::generated_image;
use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::*;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct BoundaryFrameImageIds {
    pub(super) first: Option<i64>,
    pub(super) last: Option<i64>,
}

pub(super) async fn boundary_frame_image_ids_for_prompt(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    prompt: &chapter_video_prompt::Model,
) -> Result<BoundaryFrameImageIds, String> {
    let (first_frame_number, last_frame_number) = boundary_frame_numbers_for_prompt(prompt)?;
    let frame_prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .filter(
            chapter_video_prompt::Column::ShotNumber.is_in([first_frame_number, last_frame_number]),
        )
        .order_by_desc(chapter_video_prompt::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let first_prompt_id = frame_prompts
        .iter()
        .filter(|frame| frame_prompt_matches_video_prompt(prompt, frame))
        .find(|frame| frame.shot_number == first_frame_number)
        .map(|frame| frame.id);
    let last_prompt_id = frame_prompts
        .iter()
        .filter(|frame| frame_prompt_matches_video_prompt(prompt, frame))
        .find(|frame| frame.shot_number == last_frame_number)
        .map(|frame| frame.id);

    if first_prompt_id.is_none() && last_prompt_id.is_none() {
        return Ok(BoundaryFrameImageIds::default());
    }

    let frame_images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .order_by_desc(generated_image::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(BoundaryFrameImageIds {
        first: frame_prompts
            .iter()
            .find(|frame| Some(frame.id) == first_prompt_id)
            .and_then(|frame| latest_boundary_frame_image_id(&frame_images, frame)),
        last: frame_prompts
            .iter()
            .find(|frame| Some(frame.id) == last_prompt_id)
            .and_then(|frame| latest_boundary_frame_image_id(&frame_images, frame)),
    })
}

pub(super) fn boundary_frame_numbers_for_prompt(
    prompt: &chapter_video_prompt::Model,
) -> Result<(i32, i32), String> {
    let metadata = prompt.metadata_json.as_ref().ok_or_else(|| {
        format!(
            "视频提示词 #{} 缺少结构化元数据，请重新生成视频提示词后再生成视频",
            prompt.id
        )
    })?;
    let video_unit = metadata
        .get("video_unit")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            format!(
                "视频提示词 #{} 缺少 metadata_json.video_unit，请重新生成视频提示词后再生成视频",
                prompt.id
            )
        })?;
    let first = required_video_unit_frame_number(
        prompt,
        video_unit,
        "start_frame_number",
        "起始边界帧编号",
    )?;
    let last =
        required_video_unit_frame_number(prompt, video_unit, "end_frame_number", "结束边界帧编号")?;
    Ok((first, last))
}

fn required_video_unit_frame_number(
    prompt: &chapter_video_prompt::Model,
    video_unit: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    label: &str,
) -> Result<i32, String> {
    let value = video_unit
        .get(key)
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| {
            format!(
                "视频提示词 #{} metadata_json.video_unit.{key} 缺少有效{label}，请重新生成视频提示词后再生成视频",
                prompt.id
            )
        })?;
    i32::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            format!(
                "视频提示词 #{} metadata_json.video_unit.{key} 必须是正整数，实际为 {value}",
                prompt.id
            )
        })
}

fn latest_boundary_frame_image_id(
    images: &[generated_image::Model],
    frame_prompt: &chapter_video_prompt::Model,
) -> Option<i64> {
    images
        .iter()
        .find(|image| {
            let metadata = image.metadata_json.as_ref();
            image_generation_prompt_id(metadata) == Some(frame_prompt.id)
                && image_generation_source(metadata) == Some("video_boundary_frame_prompt")
        })
        .map(|image| image.id)
}

fn frame_prompt_matches_video_prompt(
    video_prompt: &chapter_video_prompt::Model,
    frame_prompt: &chapter_video_prompt::Model,
) -> bool {
    prompt_chain_matches("boundary_frames", video_prompt, frame_prompt)
        && prompt_source_type_matches(video_prompt, "frame_segment_plan")
        && prompt_source_type_matches(frame_prompt, "frame_segment_plan")
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_services::video::{
        BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION, FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
    };
    use serde_json::{Value, json};

    fn test_time() -> chrono::NaiveDateTime {
        chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc()
    }

    fn prompt_model(
        id: i64,
        prompt_type: &str,
        workflow_mode: &str,
        shot_number: i32,
        metadata_json: Value,
    ) -> chapter_video_prompt::Model {
        chapter_video_prompt::Model {
            id,
            project_id: 1,
            chapter_id: 1,
            style: Some("电影感写实".to_string()),
            prompt_type: prompt_type.to_string(),
            workflow_mode: workflow_mode.to_string(),
            shot_number,
            title: None,
            scene_summary: None,
            visual_prompt: "视觉提示词".to_string(),
            motion_prompt: None,
            negative_prompt: None,
            duration_secs: 6,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: None,
            status: "completed".to_string(),
            generated_video_id: None,
            metadata_json: Some(metadata_json),
            created_at: test_time(),
            updated_at: test_time(),
        }
    }

    #[test]
    fn frame_prompt_match_uses_structural_chain_not_generation_settings() {
        let video = prompt_model(
            1,
            "video_prompt",
            "boundary_frames",
            1,
            json!({
                "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );
        let matching = prompt_model(
            2,
            "frame_prompt",
            "boundary_frames",
            1,
            json!({
                "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );
        let old_source_hash = prompt_model(
            3,
            "frame_prompt",
            "boundary_frames",
            1,
            json!({
                "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "old"}
            }),
        );
        let wrong_workflow = prompt_model(
            4,
            "frame_prompt",
            "first_frame_chain",
            1,
            json!({
                "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );
        let mut changed_generation_settings = matching.clone();
        changed_generation_settings.duration_secs = 10;
        changed_generation_settings.style = Some("另一种风格".to_string());
        changed_generation_settings.aspect_ratio = Some("9:16".to_string());
        let wrong_video_source_type = prompt_model(
            5,
            "video_prompt",
            "boundary_frames",
            1,
            json!({
                "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
                "source_prompt_type": "chat_video_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );

        assert!(frame_prompt_matches_video_prompt(&video, &matching));
        assert!(frame_prompt_matches_video_prompt(
            &video,
            &changed_generation_settings
        ));
        assert!(frame_prompt_matches_video_prompt(&video, &old_source_hash));
        assert!(!frame_prompt_matches_video_prompt(&video, &wrong_workflow));
        assert!(!frame_prompt_matches_video_prompt(
            &wrong_video_source_type,
            &matching
        ));
    }

    #[test]
    fn frame_prompt_match_requires_contract_but_not_source_hash() {
        let video = prompt_model(
            1,
            "video_prompt",
            "boundary_frames",
            1,
            json!({
                "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );
        let missing_contract = prompt_model(
            2,
            "frame_prompt",
            "boundary_frames",
            1,
            json!({
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );
        let missing_source_hash = prompt_model(
            3,
            "frame_prompt",
            "boundary_frames",
            1,
            json!({
                "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan"
            }),
        );

        assert!(!frame_prompt_matches_video_prompt(
            &video,
            &missing_contract
        ));
        assert!(frame_prompt_matches_video_prompt(
            &video,
            &missing_source_hash
        ));
    }

    #[test]
    fn image_generation_prompt_id_requires_json_integer() {
        assert_eq!(
            image_generation_prompt_id(Some(&json!({
                "generation_context": {"prompt_id": 12}
            }))),
            Some(12)
        );
        assert_eq!(
            image_generation_prompt_id(Some(&json!({
                "generation_context": {"prompt_id": "12"}
            }))),
            None
        );
    }
}
