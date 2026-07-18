#[tokio::test]
async fn frame_prompt_regeneration_replans_segment_for_changed_duration() {
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
    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .exec(db.as_ref())
        .await
        .unwrap();
    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
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
                &SequenceLlmProvider::new_owned({
                    let mut replies = vec![
                        segment_plan_json_for_shot(1, Some(5)),
                        segment_plan_json_for_shot(2, Some(5)),
                    ];
                    replies.extend([
                        frame_prompt_json(1),
                        frame_prompt_json(2),
                        frame_prompt_json(3),
                    ]);
                    replies
                }),
                &engine,
                regenerate_generation_options_with_duration(5),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let segment_plan = rows
        .iter()
        .find(|row| row.prompt_type == "frame_segment_plan")
        .expect("regeneration should keep a segment plan row");
    assert_eq!(segment_plan.duration_secs, 5);
    let frame_prompts = rows
        .iter()
        .filter(|row| row.prompt_type == "frame_prompt")
        .collect::<Vec<_>>();
    assert_eq!(frame_prompts.len(), 3);
    assert!(
        frame_prompts.iter().all(|row| row.duration_secs == 5),
        "regenerated frame prompts must use the user-selected duration"
    );
}

#[tokio::test]
async fn frame_prompt_regeneration_deletes_stale_frames_when_segment_count_shrinks() {
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
    replace_video_chapter_shooting_script_with_single_shot(db.as_ref(), project.id, &chapter).await;
    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .exec(db.as_ref())
        .await
        .unwrap();
    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
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
                &SequenceLlmProvider::new_owned({
                    let mut replies = vec![single_segment_plan_json(5)];
                    replies.extend(single_segment_frame_prompt_replies());
                    replies
                }),
                &engine,
                regenerate_generation_options_with_duration(5),
                None,
            ),
            None,
            None,
        )
        .await
        .expect("stale boundary frames should be deleted in the same sync batch");

    let frame_prompts = rows
        .iter()
        .filter(|row| row.prompt_type == "frame_prompt")
        .collect::<Vec<_>>();
    assert_eq!(frame_prompts.len(), 2);
    assert!(
        frame_prompts.iter().all(|row| row.duration_secs == 5),
        "remaining frame prompts must use the regenerated duration"
    );
    assert!(
        !frame_prompts.iter().any(|row| row.shot_number == 3),
        "stale third boundary frame must be removed"
    );
}

#[tokio::test]
async fn frame_prompt_regeneration_rejects_generated_frame_images_before_llm() {
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
    insert_boundary_frame_images(db.as_ref(), project.id, chapter.id).await;

    let err = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new(Vec::new()),
                &engine,
                regenerate_generation_options_with_duration(5),
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("generated frame images must block prompt regeneration before LLM");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("边界帧图片")
                && (message.contains("不能重新生成边界帧提示词")
                    || message.contains("不能删除边界帧提示词"))
    ));
}

#[tokio::test]
async fn frame_prompt_generation_rejects_duration_mismatch_without_regeneration() {
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

    let mut changed_options = frame_generation_options();
    changed_options.duration_secs = 5;
    let err = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new(Vec::new()),
                &engine,
                changed_options,
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("duration changes require explicit regeneration of the v5 execution plan");

    assert!(
        err.to_string().contains("duration_secs=4")
            && err.to_string().contains("当前请求 5 不一致")
    );
}
