use super::prompt_chain::{
    image_generation_prompt_id, image_generation_shot_number, image_generation_source,
    prompt_chain_matches, prompt_source_type_matches,
};
use crate::AppState;
use a4n_core::models::ai::generated_image;
use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::*;

pub(super) async fn first_frame_chain_start_image_id_for_prompt(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    prompt: &chapter_video_prompt::Model,
) -> Result<Option<i64>, String> {
    if prompt.shot_number <= 1 {
        return latest_generated_first_frame_image_id(state, project_id, chapter_id, prompt).await;
    }
    let Some(previous_prompt) =
        latest_previous_video_prompt(state, project_id, chapter_id, prompt).await?
    else {
        return Ok(None);
    };
    latest_tail_frame_image_id_for_prompt(state, project_id, chapter_id, &previous_prompt).await
}

async fn latest_generated_first_frame_image_id(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    video_prompt: &chapter_video_prompt::Model,
) -> Result<Option<i64>, String> {
    let first_frame_prompt = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.eq("first_frame_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .order_by_desc(chapter_video_prompt::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let Some(first_frame_prompt) = first_frame_prompt
        .iter()
        .find(|frame| first_frame_prompt_matches_video_prompt(video_prompt, frame))
    else {
        return Ok(None);
    };
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .order_by_desc(generated_image::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(images
        .iter()
        .find(|image| {
            let metadata = image.metadata_json.as_ref();
            image_generation_prompt_id(metadata) == Some(first_frame_prompt.id)
                && image_generation_source(metadata) == Some("video_first_frame_prompt")
        })
        .map(|image| image.id))
}

async fn previous_video_prompt_candidates(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<chapter_video_prompt::Model>, String> {
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("first_frame_chain"))
        .order_by_desc(chapter_video_prompt::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())
}

async fn latest_previous_video_prompt(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    current_prompt: &chapter_video_prompt::Model,
) -> Result<Option<chapter_video_prompt::Model>, String> {
    Ok(
        previous_video_prompt_candidates(state, project_id, chapter_id)
            .await?
            .into_iter()
            .find(|candidate| previous_video_prompt_matches_current(current_prompt, candidate)),
    )
}

async fn latest_tail_frame_image_id_for_prompt(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    previous_prompt: &chapter_video_prompt::Model,
) -> Result<Option<i64>, String> {
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .order_by_desc(generated_image::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(images
        .iter()
        .find(|image| tail_frame_image_matches_prompt(image, previous_prompt))
        .map(|image| image.id))
}

fn first_frame_prompt_matches_video_prompt(
    video_prompt: &chapter_video_prompt::Model,
    first_frame_prompt: &chapter_video_prompt::Model,
) -> bool {
    prompt_chain_matches("first_frame_chain", video_prompt, first_frame_prompt)
        && prompt_uses_shared_segment_source(video_prompt)
        && prompt_uses_shared_segment_source(first_frame_prompt)
}

fn previous_video_prompt_matches_current(
    current_prompt: &chapter_video_prompt::Model,
    previous_prompt: &chapter_video_prompt::Model,
) -> bool {
    prompt_chain_matches("first_frame_chain", current_prompt, previous_prompt)
        && current_prompt.prompt_type == "video_prompt"
        && previous_prompt.prompt_type == "video_prompt"
        && previous_prompt.shot_number == current_prompt.shot_number - 1
        && prompt_uses_shared_segment_source(current_prompt)
        && prompt_uses_shared_segment_source(previous_prompt)
}

fn prompt_uses_shared_segment_source(prompt: &chapter_video_prompt::Model) -> bool {
    prompt_source_type_matches(prompt, "frame_segment_plan")
}

fn tail_frame_image_matches_prompt(
    image: &generated_image::Model,
    previous_prompt: &chapter_video_prompt::Model,
) -> bool {
    let metadata = image.metadata_json.as_ref();
    image_generation_source(metadata) == Some("video_tail_frame_capture")
        && image_generation_prompt_id(metadata) == Some(previous_prompt.id)
        && image_generation_shot_number(metadata) == Some(previous_prompt.shot_number)
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_services::video::FIRST_FRAME_CHAIN_CONTRACT_VERSION;
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

    fn tail_frame_image(id: i64, metadata_json: Value) -> generated_image::Model {
        generated_image::Model {
            id,
            project_id: 1,
            image_type: "video_tail_frame".to_string(),
            file_path: format!("/tmp/tail-{id}.png"),
            prompt: "尾帧".to_string(),
            model: "ffmpeg".to_string(),
            width: 1280,
            height: 720,
            file_size: Some(1),
            ai_score: None,
            is_favorite: false,
            chapter_id: Some(1),
            panel_id: None,
            metadata_json: Some(metadata_json),
            created_at: test_time(),
        }
    }

    #[test]
    fn first_frame_prompt_match_uses_structural_chain_not_generation_settings() {
        let video = prompt_model(
            1,
            "video_prompt",
            "first_frame_chain",
            1,
            json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );
        let matching = prompt_model(
            2,
            "first_frame_prompt",
            "first_frame_chain",
            1,
            json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );
        let old_source_hash = prompt_model(
            3,
            "first_frame_prompt",
            "first_frame_chain",
            1,
            json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "old"}
            }),
        );
        let mut changed_generation_settings = matching.clone();
        changed_generation_settings.duration_secs = 15;
        changed_generation_settings.style = Some("另一种风格".to_string());
        changed_generation_settings.aspect_ratio = Some("9:16".to_string());
        let wrong_workflow = prompt_model(
            4,
            "first_frame_prompt",
            "boundary_frames",
            1,
            json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );
        let wrong_video_source_type = prompt_model(
            5,
            "video_prompt",
            "first_frame_chain",
            1,
            json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": "chat_video_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );

        assert!(first_frame_prompt_matches_video_prompt(&video, &matching));
        assert!(first_frame_prompt_matches_video_prompt(
            &video,
            &changed_generation_settings
        ));
        assert!(first_frame_prompt_matches_video_prompt(
            &video,
            &old_source_hash
        ));
        assert!(!first_frame_prompt_matches_video_prompt(
            &video,
            &wrong_workflow
        ));
        assert!(!first_frame_prompt_matches_video_prompt(
            &wrong_video_source_type,
            &matching
        ));
    }

    #[test]
    fn first_frame_prompt_match_requires_contract_but_not_source_hash() {
        let video = prompt_model(
            1,
            "video_prompt",
            "first_frame_chain",
            1,
            json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );
        let missing_contract = prompt_model(
            2,
            "first_frame_prompt",
            "first_frame_chain",
            1,
            json!({
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );
        let missing_source_hash = prompt_model(
            3,
            "first_frame_prompt",
            "first_frame_chain",
            1,
            json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan"
            }),
        );

        assert!(!first_frame_prompt_matches_video_prompt(
            &video,
            &missing_contract
        ));
        assert!(first_frame_prompt_matches_video_prompt(
            &video,
            &missing_source_hash
        ));
    }

    #[test]
    fn first_frame_prompt_match_accepts_shared_segment_plan_source() {
        let video = prompt_model(
            1,
            "video_prompt",
            "first_frame_chain",
            1,
            json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );
        let first_frame = prompt_model(
            2,
            "first_frame_prompt",
            "first_frame_chain",
            1,
            json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );

        assert!(first_frame_prompt_matches_video_prompt(
            &video,
            &first_frame
        ));
    }

    #[test]
    fn previous_video_prompt_match_requires_same_explicit_chain() {
        let current = prompt_model(
            12,
            "video_prompt",
            "first_frame_chain",
            2,
            json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );
        let matching_previous = prompt_model(
            11,
            "video_prompt",
            "first_frame_chain",
            1,
            json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );
        let old_source_hash_previous = prompt_model(
            10,
            "video_prompt",
            "first_frame_chain",
            1,
            json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "old"}
            }),
        );
        let wrong_source_type = prompt_model(
            9,
            "video_prompt",
            "first_frame_chain",
            1,
            json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": "chat_video_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );
        let wrong_shot = prompt_model(
            8,
            "video_prompt",
            "first_frame_chain",
            3,
            json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );

        assert!(previous_video_prompt_matches_current(
            &current,
            &matching_previous
        ));
        assert!(previous_video_prompt_matches_current(
            &current,
            &old_source_hash_previous
        ));
        assert!(!previous_video_prompt_matches_current(
            &current,
            &wrong_source_type
        ));
        assert!(!previous_video_prompt_matches_current(
            &current,
            &wrong_shot
        ));
    }

    #[test]
    fn tail_frame_must_bind_previous_prompt_id_and_shot_number() {
        let previous_prompt = prompt_model(
            11,
            "video_prompt",
            "first_frame_chain",
            1,
            json!({
                "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "current"}
            }),
        );
        let matching = tail_frame_image(
            1,
            json!({
                "generation_context": {
                    "source": "video_tail_frame_capture",
                    "prompt_id": 11,
                    "shot_number": 1,
                    "source_content_sha256": "current"
                }
            }),
        );
        let hash_only = tail_frame_image(
            2,
            json!({
                "generation_context": {
                    "source": "video_tail_frame_capture",
                    "shot_number": 1,
                    "source_content_sha256": "current"
                }
            }),
        );
        let wrong_prompt = tail_frame_image(
            3,
            json!({
                "generation_context": {
                    "source": "video_tail_frame_capture",
                    "prompt_id": 99,
                    "shot_number": 1,
                    "source_content_sha256": "current"
                }
            }),
        );
        let wrong_shot = tail_frame_image(
            4,
            json!({
                "generation_context": {
                    "source": "video_tail_frame_capture",
                    "prompt_id": 11,
                    "shot_number": 2,
                    "source_content_sha256": "current"
                }
            }),
        );

        assert!(tail_frame_image_matches_prompt(&matching, &previous_prompt));
        assert!(!tail_frame_image_matches_prompt(
            &hash_only,
            &previous_prompt
        ));
        assert!(!tail_frame_image_matches_prompt(
            &wrong_prompt,
            &previous_prompt
        ));
        assert!(!tail_frame_image_matches_prompt(
            &wrong_shot,
            &previous_prompt
        ));
    }

    #[test]
    fn image_generation_context_ids_require_json_integers() {
        let numeric = json!({
            "generation_context": {
                "prompt_id": 11,
                "shot_number": 1
            }
        });
        let strings = json!({
            "generation_context": {
                "prompt_id": "11",
                "shot_number": "1"
            }
        });

        assert_eq!(image_generation_prompt_id(Some(&numeric)), Some(11));
        assert_eq!(image_generation_shot_number(Some(&numeric)), Some(1));
        assert_eq!(image_generation_prompt_id(Some(&strings)), None);
        assert_eq!(image_generation_shot_number(Some(&strings)), None);
    }
}
