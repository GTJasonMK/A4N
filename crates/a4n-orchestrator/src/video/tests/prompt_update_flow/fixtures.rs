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
