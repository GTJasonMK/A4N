#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::prompts::records::VideoExecutionIdentityMatch;
    use serde_json::json;

    fn prompt_row(id: i64, workflow_mode: &str, prompt_type: &str) -> chapter_video_prompt::Model {
        let now = chrono::Utc::now().naive_utc();
        chapter_video_prompt::Model {
            id,
            project_id: 1,
            chapter_id: 2,
            style: Some("测试风格".to_string()),
            prompt_type: prompt_type.to_string(),
            workflow_mode: workflow_mode.to_string(),
            shot_number: 0,
            title: Some("测试提示词".to_string()),
            scene_summary: None,
            visual_prompt: "{}".to_string(),
            motion_prompt: None,
            negative_prompt: None,
            duration_secs: 6,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: None,
            status: "completed".to_string(),
            generated_video_id: None,
            timeline_item_id: None,
            metadata_json: Some(json!({
                "source": {"content_sha256": "same-source"},
                "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION
            })),
            created_at: now,
            updated_at: now,
        }
    }

    fn test_generation_context_for_workflow<'a>(
        expected_workflow_mode: &'a str,
        prompt_type: &'a str,
        source_metadata: &'a Value,
        reference_ids: &'a Value,
        generation_asset_fingerprint: Option<&'a Value>,
        source_prompt_type: Option<&'a str>,
        prompt_output_language: PromptOutputLanguage,
    ) -> PromptGenerationContextForWorkflow<'a> {
        PromptGenerationContextForWorkflow {
            expected_workflow_mode,
            prompt_type,
            source_metadata,
            style: "测试风格",
            aspect_ratio: "16:9",
            reference_ids,
            generation_asset_fingerprint,
            source_prompt_type,
            video_execution_identity: VideoExecutionIdentityMatch::NotApplicable,
            prompt_output_language,
        }
    }

    fn test_generation_context<'a>(
        prompt_type: &'a str,
        source_metadata: &'a Value,
        reference_ids: &'a Value,
        generation_asset_fingerprint: Option<&'a Value>,
        source_prompt_type: Option<&'a str>,
        prompt_output_language: PromptOutputLanguage,
    ) -> PromptGenerationContext<'a> {
        PromptGenerationContext {
            prompt_type,
            source_metadata,
            style: "测试风格",
            aspect_ratio: "16:9",
            reference_ids,
            generation_asset_fingerprint,
            source_prompt_type,
            video_execution_identity: VideoExecutionIdentityMatch::NotApplicable,
            prompt_output_language,
        }
    }

    #[test]
    fn reference_ids_match_requires_exact_order_without_deduping() {
        assert!(reference_ids_match(
            Some(&json!([6, 1, 2])),
            &json!([6, 1, 2])
        ));
        assert!(!reference_ids_match(
            Some(&json!([1, 2, 6])),
            &json!([6, 1, 2])
        ));
        assert!(!reference_ids_match(
            Some(&json!([6, 1, 6, 2])),
            &json!([6, 1, 2])
        ));
    }

    #[test]
    fn shared_segment_plan_context_does_not_depend_on_generation_reference_ids() {
        let source_metadata = json!({"content_sha256": "same-source"});
        let mut segment = prompt_row(1, "shared_segment_plan", "frame_segment_plan");
        segment.reference_image_ids_json = Some(json!([1]));
        segment.metadata_json = Some(json!({
            "source": {"content_sha256": "same-source"},
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "generation_reference_image_ids": [1],
            "prompt_output_language": "en"
        }));

        assert!(prompt_matches_generation_context_for_workflow(
            &segment,
            test_generation_context_for_workflow(
                "boundary_frames",
                "frame_segment_plan",
                &source_metadata,
                &json!([2]),
                None,
                None,
                PromptOutputLanguage::ZhCn,
            ),
        ));
    }

    #[test]
    fn segment_lineage_allows_final_duration_override() {
        let expected = json!({
            "contract_epoch": 5,
            "source_content_sha256": "source",
            "style_sha256": "style",
            "duration_secs": 6,
            "aspect_ratio": "16:9",
            "segment_plan_prompt_id": 10,
            "source_shot_id": "shot-1",
            "segment_index_in_shot": 1,
            "segment_sha256": "segment",
            "segment_recipe_fingerprint": "recipe"
        });
        let mut stored = expected.clone();
        stored["duration_secs"] = json!(10);
        let metadata = json!({"video_execution_identity": stored});

        assert!(!video_execution_segment_identity_matches(
            Some(&metadata),
            &expected,
        ));
        assert!(video_execution_segment_lineage_matches(
            Some(&metadata),
            &expected,
        ));
    }

    #[test]
    fn chat_video_prompt_context_keeps_generation_reference_snapshot() {
        let source_metadata = json!({"content_sha256": "same-source"});
        let asset_fingerprint = json!({
            "schema_version": 1,
            "reference_image_ids": [1],
            "video_library_image_ids": [],
            "available_assets_sha256": "same-assets"
        });
        let mut chat_video_prompt = prompt_row(1, "chat_video", "video_prompt");
        chat_video_prompt.metadata_json = Some(json!({
            "source": {"content_sha256": "same-source"},
            "contract_version": CHAT_VIDEO_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "generation_reference_image_ids": [1],
            "prompt_output_language": "zh_cn",
            "generation_asset_fingerprint": asset_fingerprint.clone()
        }));

        assert!(prompt_matches_generation_context_for_workflow(
            &chat_video_prompt,
            test_generation_context_for_workflow(
                "chat_video",
                "video_prompt",
                &source_metadata,
                &json!([1]),
                Some(&asset_fingerprint),
                Some("frame_segment_plan"),
                PromptOutputLanguage::ZhCn,
            ),
        ));
        assert!(!prompt_matches_generation_context_for_workflow(
            &chat_video_prompt,
            test_generation_context_for_workflow(
                "chat_video",
                "video_prompt",
                &source_metadata,
                &json!([2]),
                Some(&asset_fingerprint),
                Some("frame_segment_plan"),
                PromptOutputLanguage::ZhCn,
            ),
        ));
    }

    #[test]
    fn final_prompt_context_matches_prompt_output_language() {
        let source_metadata = json!({"content_sha256": "same-source"});
        let asset_fingerprint = json!({
            "schema_version": 1,
            "reference_image_ids": [],
            "video_library_image_ids": [],
            "available_assets_sha256": "same-assets"
        });
        let mut video_prompt = prompt_row(1, "first_frame_chain", "video_prompt");
        video_prompt.metadata_json = Some(json!({
            "source": {"content_sha256": "same-source"},
            "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "generation_asset_fingerprint": asset_fingerprint.clone(),
            "prompt_output_language": "en"
        }));

        assert!(prompt_matches_generation_context_for_workflow(
            &video_prompt,
            test_generation_context_for_workflow(
                "first_frame_chain",
                "video_prompt",
                &source_metadata,
                &json!([]),
                Some(&asset_fingerprint),
                Some("frame_segment_plan"),
                PromptOutputLanguage::En,
            ),
        ));
        assert!(!prompt_matches_generation_context_for_workflow(
            &video_prompt,
            test_generation_context_for_workflow(
                "first_frame_chain",
                "video_prompt",
                &source_metadata,
                &json!([]),
                Some(&asset_fingerprint),
                Some("frame_segment_plan"),
                PromptOutputLanguage::ZhCn,
            ),
        ));
    }

    #[test]
    fn final_prompt_context_requires_explicit_workflow_after_shared_segment_plan() {
        let source_metadata = json!({"content_sha256": "same-source"});
        let asset_fingerprint = json!({
            "schema_version": 1,
            "reference_image_ids": [],
            "video_library_image_ids": [],
            "available_assets_sha256": "same-assets"
        });
        let mut video_prompt = prompt_row(1, "first_frame_chain", "video_prompt");
        video_prompt.metadata_json = Some(json!({
            "source": {"content_sha256": "same-source"},
            "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "generation_asset_fingerprint": asset_fingerprint.clone(),
            "prompt_output_language": "zh_cn"
        }));

        assert!(
            !prompt_matches_generation_context(
                &video_prompt,
                test_generation_context(
                    "video_prompt",
                    &source_metadata,
                    &json!([]),
                    Some(&asset_fingerprint),
                    Some("frame_segment_plan"),
                    PromptOutputLanguage::ZhCn,
                ),
            ),
            "source_prompt_type=frame_segment_plan is shared and must not imply boundary_frames"
        );
        assert!(prompt_matches_generation_context_for_workflow(
            &video_prompt,
            test_generation_context_for_workflow(
                "first_frame_chain",
                "video_prompt",
                &source_metadata,
                &json!([]),
                Some(&asset_fingerprint),
                Some("frame_segment_plan"),
                PromptOutputLanguage::ZhCn,
            ),
        ));
    }

    #[test]
    fn frame_prompt_context_rejects_changed_generation_asset_fingerprint() {
        let source_metadata = json!({"content_sha256": "same-source"});
        let stored_asset_fingerprint = json!({
            "schema_version": 1,
            "reference_image_ids": [1],
            "video_library_image_ids": [],
            "available_assets_sha256": "old-assets"
        });
        let current_asset_fingerprint = json!({
            "schema_version": 1,
            "reference_image_ids": [1],
            "video_library_image_ids": [9],
            "available_assets_sha256": "new-assets"
        });
        let mut first_frame_prompt = prompt_row(1, "first_frame_chain", "first_frame_prompt");
        first_frame_prompt.metadata_json = Some(json!({
            "source": {"content_sha256": "same-source"},
            "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "generation_asset_fingerprint": stored_asset_fingerprint
        }));

        assert!(!prompt_matches_generation_context_for_workflow(
            &first_frame_prompt,
            test_generation_context_for_workflow(
                "first_frame_chain",
                "first_frame_prompt",
                &source_metadata,
                &json!([1]),
                Some(&current_asset_fingerprint),
                Some("frame_segment_plan"),
                PromptOutputLanguage::ZhCn,
            ),
        ));
    }

    #[test]
    fn frame_prompt_edit_syncs_structured_metadata_copy() {
        let metadata = json!({
            "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
            "boundary_frame": {
                "frame_number": 1,
                "title": "旧标题",
                "scene_summary": "旧摘要",
                "frame_prompt": "旧帧提示词",
                "negative_prompt": "旧负面词",
                "duration_secs": 4,
                "aspect_ratio": "16:9",
                "reference_image_ids": [1],
                "segment_numbers": [1],
                "blocking_state": {},
                "lighting_state": {}
            }
        });

        let synced = a4n_db::video_prompt::sync_chapter_video_prompt_metadata_after_update(
            "frame_prompt",
            Some(metadata),
            " 新帧提示词 ",
            Some(" 新负面词 "),
            6,
            Some(&json!([2, 3])),
        )
        .unwrap()
        .unwrap();
        let frame = synced
            .get("boundary_frame")
            .and_then(Value::as_object)
            .expect("boundary frame metadata should remain an object");

        assert_eq!(
            frame.get("frame_prompt").and_then(Value::as_str),
            Some("新帧提示词")
        );
        assert_eq!(
            frame.get("negative_prompt").and_then(Value::as_str),
            Some("新负面词")
        );
        assert_eq!(frame.get("duration_secs").and_then(Value::as_i64), Some(6));
        assert_eq!(frame.get("reference_image_ids"), Some(&json!([2, 3])));
        assert_eq!(frame.get("title").and_then(Value::as_str), Some("旧标题"));
    }

    #[test]
    fn boundary_frame_prompt_uses_separate_contract_from_segment_plan() {
        assert_eq!(
            expected_contract_version("shared_segment_plan", "frame_segment_plan"),
            Some(FRAME_SEGMENT_PLAN_CONTRACT_VERSION)
        );
        assert_eq!(
            expected_contract_version("boundary_frames", "frame_prompt"),
            Some(BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION)
        );
        assert_eq!(
            expected_contract_version("first_frame_chain", "frame_segment_plan"),
            None
        );
        assert_eq!(
            expected_contract_version("chat_video", "frame_segment_plan"),
            None
        );
    }
}
