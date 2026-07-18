#[tokio::test]
async fn first_frame_chain_video_prompt_rejects_duration_changed_after_frame_stage() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_first_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(vec![
                    segment_plan_json_for_shot(1, None),
                    segment_plan_json_for_shot(2, None),
                    frame_prompt_json(1),
                ]),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let mut video_options = frame_generation_options();
    video_options.duration_secs = 8;
    video_options.duration_secs_options = vec![4, 8];

    let err = service
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "first_frame_chain",
                &SequenceLlmProvider::new(Vec::new()),
                &engine,
                video_options,
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("video stage must keep the duration fixed by the saved segment plan");

    assert!(
        err.to_string().contains("duration_secs=4")
            && err.to_string().contains("当前请求 8 不一致")
    );
}

#[tokio::test]
async fn first_frame_chain_video_prompt_generation_respects_batch_size() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_first_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(vec![
                    segment_plan_json_for_shot(1, None),
                    segment_plan_json_for_shot(2, None),
                    frame_prompt_json(1),
                ]),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let batch_one_options = video_generation_options_with_batch_size(1);
    let first_rows = service
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(vec![video_shot_json(1)]),
                &engine,
                batch_one_options.clone(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        first_rows
            .iter()
            .filter(|row| row.prompt_type == "video_prompt")
            .count(),
        1
    );

    let rows = service
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(vec![video_shot_json(2)]),
                &engine,
                batch_one_options,
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();
    let video_rows = rows
        .iter()
        .filter(|row| row.prompt_type == "video_prompt")
        .collect::<Vec<_>>();
    assert_eq!(video_rows.len(), 2);
    assert_eq!(video_rows[0].shot_number, 1);
    assert_eq!(video_rows[1].shot_number, 2);
}
