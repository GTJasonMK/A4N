#[tokio::test]
async fn first_frame_chain_video_prompt_generation_resumes_from_saved_serial_prefix() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let first_frame_llm = SequenceLlmProvider::new_owned(vec![
        segment_plan_json_for_shot(1, None),
        segment_plan_json_for_shot(2, None),
        frame_prompt_json(1),
    ]);

    service
        .generate_chapter_first_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "first_frame_chain",
                &first_frame_llm,
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let interrupted_llm = SequenceLlmProvider::new_owned(vec![video_shot_json(1)]);
    let workflow_run_id = next_test_workflow_run_id(&llm_switch, chapter.id);
    let mut interrupted_options = frame_generation_options();
    interrupted_options.workflow_run_id = Some(workflow_run_id);
    service
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "first_frame_chain",
                &interrupted_llm,
                &engine,
                interrupted_options,
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("second segment should fail after first segment checkpoint");

    let saved_after_interrupt = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("first_frame_chain"))
        .all(db.as_ref())
        .await
        .unwrap();
    assert!(saved_after_interrupt.is_empty());
    let completed_checkpoints = a4n_core::models::generation::workflow_checkpoint::Entity::find()
        .filter(
            a4n_core::models::generation::workflow_checkpoint::Column::WorkflowRunId
                .eq(workflow_run_id),
        )
        .filter(a4n_core::models::generation::workflow_checkpoint::Column::Status.eq("completed"))
        .all(db.as_ref())
        .await
        .unwrap();
    assert!(
        completed_checkpoints
            .iter()
            .any(|checkpoint| checkpoint.step_key.contains("shot:1:draft"))
    );
    assert!(
        completed_checkpoints
            .iter()
            .all(|checkpoint| !checkpoint.step_key.contains(":review"))
    );

    let resume_llm = SequenceLlmProvider::new_owned(vec![video_shot_json(2)]);
    let mut resume_options = frame_generation_options();
    resume_options.workflow_run_id = Some(workflow_run_id);
    let rows = service
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "first_frame_chain",
                &resume_llm,
                &engine,
                resume_options,
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
