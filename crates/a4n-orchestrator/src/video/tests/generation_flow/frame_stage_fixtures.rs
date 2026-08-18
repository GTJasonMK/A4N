async fn generate_shared_segment_plan_stage(
    service: &OrchestratorContext,
    llm_switch: &SwitchableLlmProvider,
    project_id: i64,
    chapter_id: i64,
    engine: &TeraPromptEngine,
) {
    service
        .generate_chapter_segment_plan(
            prompt_stage_input(
                llm_switch,
                project_id,
                chapter_id,
                "shared_segment_plan",
                &SequenceLlmProvider::new_owned(vec![
                    segment_plan_json_for_shot(1, None),
                    segment_plan_json_for_shot(2, None),
                ]),
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

async fn set_segment_recipe_mode(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    segment_number: i32,
    workflow_mode: a4n_core::video_execution::VideoSegmentWorkflowModeV1,
    guidance_mode: a4n_core::video_execution::VideoSegmentGuidanceModeV1,
) {
    let plan = service
        .current_shared_segment_plan(project_id, chapter_id)
        .await
        .unwrap();
    let mut recipe = a4n_core::video_execution::video_segment_recipes_for_plan_from_metadata(
        plan.metadata_json.as_ref(),
        plan.style.as_deref().unwrap(),
    )
    .unwrap()
    .into_iter()
    .find(|recipe| recipe.segment_number == segment_number)
    .unwrap();
    recipe.workflow_mode = workflow_mode;
    recipe.guidance_mode = guidance_mode;
    if guidance_mode == a4n_core::video_execution::VideoSegmentGuidanceModeV1::SystemFrames {
        recipe.reference_image_ids.clear();
    }
    service
        .update_chapter_video_segment_recipe(plan.id, recipe)
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
                    "source": IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
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
