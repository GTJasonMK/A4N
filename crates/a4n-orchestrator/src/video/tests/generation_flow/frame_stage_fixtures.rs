async fn generate_chat_video_frame_prompt_stage(
    service: &OrchestratorContext,
    llm_switch: &SwitchableLlmProvider,
    project_id: i64,
    chapter_id: i64,
    engine: &TeraPromptEngine,
) {
    let mut replies = vec![
        segment_plan_json_for_shot(1, None),
        segment_plan_json_for_shot(2, None),
    ];
    replies.extend(boundary_frame_prompt_replies());
    generate_chat_video_frame_prompt_stage_with_replies(
        service, llm_switch, project_id, chapter_id, engine, replies,
    )
    .await;
}

async fn generate_chat_video_frame_prompt_stage_from_existing_plan(
    service: &OrchestratorContext,
    llm_switch: &SwitchableLlmProvider,
    project_id: i64,
    chapter_id: i64,
    engine: &TeraPromptEngine,
) {
    generate_chat_video_frame_prompt_stage_with_replies(
        service,
        llm_switch,
        project_id,
        chapter_id,
        engine,
        boundary_frame_prompt_replies(),
    )
    .await;
}

async fn generate_chat_video_frame_prompt_stage_with_replies(
    service: &OrchestratorContext,
    llm_switch: &SwitchableLlmProvider,
    project_id: i64,
    chapter_id: i64,
    engine: &TeraPromptEngine,
    replies: Vec<String>,
) {
    service
        .generate_chapter_chat_video_frame_prompts(
            prompt_stage_input(
                llm_switch,
                project_id,
                chapter_id,
                "chat_video",
                &SequenceLlmProvider::new_owned(replies),
                engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();
}

async fn insert_boundary_frame_images(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> HashMap<i32, i64> {
    let frame_rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .all(db)
        .await
        .unwrap();
    let mut image_id_by_frame_number = HashMap::new();
    let source_metadata = video_source_metadata(&generation_source());
    for row in frame_rows {
        let image = insert_generated_image_with_binding(
            db,
            project_id,
            Some(chapter_id),
            Some(json!({
                "generation_context": {
                    "source": "video_boundary_frame_prompt",
                    "prompt_id": row.id
                },
                "source": source_metadata.clone()
            })),
        )
        .await;
        image_id_by_frame_number.insert(row.shot_number, image.id);
    }
    image_id_by_frame_number
}
