async fn insert_stage_prompt(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    prompt_type: &str,
    workflow_mode: &str,
    shot_number: i32,
    metadata_json: Value,
) -> chapter_video_prompt::Model {
    let now = fixed_time(4);
    let mut metadata_json = metadata_json;
    if let Some(metadata) = metadata_json.as_object_mut() {
        metadata
            .entry("prompt_output_language".to_string())
            .or_insert_with(|| Value::String("zh_cn".to_string()));
        metadata
            .entry("video_execution_identity".to_string())
            .or_insert_with(|| {
                json!({
                    "contract_epoch": 1,
                    "source_content_sha256": "same",
                    "style_sha256": "style",
                    "duration_secs": 6,
                    "aspect_ratio": "16:9",
                    "segment_plan_prompt_id": 1,
                    "source_shot_id": "shot-1",
                    "segment_index_in_shot": 1,
                    "segment_sha256": "segment-1",
                    "segment_recipe_fingerprint": "recipe-1"
                })
            });
    }
    chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set(prompt_type.to_string()),
        workflow_mode: Set(workflow_mode.to_string()),
        shot_number: Set(shot_number),
        title: Set(Some("阶段提示词".to_string())),
        scene_summary: Set(Some("阶段摘要".to_string())),
        visual_prompt: Set("阶段视觉提示词".to_string()),
        motion_prompt: Set(Some("阶段动作提示词".to_string())),
        negative_prompt: Set(None),
        duration_secs: Set(6),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(None),
        status: Set("completed".to_string()),
        generated_video_id: Set(None),
        metadata_json: Set(Some(metadata_json)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}
