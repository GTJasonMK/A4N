#[tokio::test]
async fn boundary_frame_generation_resumes_after_deleted_middle_frame_without_replanning() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(boundary_frame_generation_replies()),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();
    let segment_plan = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("segment plan should exist");
    let first_frame = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("first frame prompt should exist");
    let first_frame_image = insert_generated_image_with_binding(
        db.as_ref(),
        project.id,
        Some(chapter.id),
        Some(json!({
            "generation_context": {
                "source": "video_boundary_frame_prompt",
                "prompt_id": first_frame.id
            },
            "source": video_source_metadata(&generation_source())
        })),
    )
    .await;
    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(2))
        .exec(db.as_ref())
        .await
        .unwrap();

    let rows = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(vec![frame_prompt_json(2)]),
                &engine,
                video_generation_options_with_batch_size(1),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    assert!(rows.iter().any(|row| {
        row.prompt_type == "frame_prompt"
            && row.shot_number == 2
            && row.reference_image_ids_json == Some(json!([first_frame_image.id]))
    }));
    let current_segment_plan_id = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("segment plan should still exist")
        .id;
    assert_eq!(current_segment_plan_id, segment_plan.id);
    let frame_numbers = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .all(db.as_ref())
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.shot_number)
        .collect::<Vec<_>>();
    assert_eq!(frame_numbers, vec![1, 2]);
}

#[tokio::test]
async fn boundary_frame_partial_generation_preserves_later_prompt_with_generated_image() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(boundary_frame_generation_replies()),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();
    let third_frame = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(3))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("third frame prompt should exist");
    let third_frame_image = insert_generated_image_with_binding(
        db.as_ref(),
        project.id,
        Some(chapter.id),
        Some(json!({
            "generation_context": {
                "source": "video_boundary_frame_prompt",
                "prompt_id": third_frame.id
            },
            "source": video_source_metadata(&generation_source())
        })),
    )
    .await;
    let reference_image = insert_generated_image(db.as_ref(), project.id).await;
    let mut options = video_generation_options_with_batch_size(1);
    options.reference_image_ids = vec![reference_image.id];
    options.video_library_image_ids = vec![reference_image.id];

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(vec![frame_prompt_json(1)]),
                &engine,
                options,
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let preserved_third_frame = chapter_video_prompt::Entity::find_by_id(third_frame.id)
        .one(db.as_ref())
        .await
        .unwrap();
    assert!(
        preserved_third_frame.is_some(),
        "partial boundary frame checkpoint must preserve later frame prompts that already produced images"
    );
    let preserved_image = generated_image::Entity::find_by_id(third_frame_image.id)
        .one(db.as_ref())
        .await
        .unwrap();
    assert!(preserved_image.is_some());
}
