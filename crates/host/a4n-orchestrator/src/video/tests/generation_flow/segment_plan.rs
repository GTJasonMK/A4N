#[tokio::test]
async fn segment_plan_requires_current_shooting_script_before_llm() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter_without_shooting_script(db.as_ref(), project.id, 1).await;

    let error = service
        .generate_chapter_segment_plan(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "shared_segment_plan",
                &SequenceLlmProvider::new(Vec::new()),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("segment planning must require a current shooting script");

    assert!(matches!(
        error,
        AppError::NotFound(message)
            if message.contains("拍摄剧本") && message.contains("同步来源")
    ));
}

#[tokio::test]
async fn scene_plan_uses_published_script_without_resumable_workflow() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "已发布场景规划").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let source = crate::shooting_script::ChapterShootingScriptSourceInput {
        project_id: project.id,
        source_chapter_id: chapter.id,
        visual_plan_id: None,
    };
    assert!(
        service
            .current_chapter_shooting_script_unit_progress(source.clone())
            .await
            .unwrap()
            .is_none()
    );
    let published = service
        .require_current_chapter_shooting_script(source)
        .await
        .unwrap();
    let scene = &published.shooting_script.script.scenes[0];
    let expected_shots = scene
        .shots
        .iter()
        .map(|shot| shot.shot_id.clone())
        .collect::<Vec<_>>();
    let responses = expected_shots
        .iter()
        .map(|id| {
            segment_plan_json_for_shot(id.strip_prefix("shot-").unwrap().parse().unwrap(), None)
        })
        .collect();
    let options = ChapterVideoPromptOptions {
        workflow_run_id: Some(chapter.id * 100 + 1),
        target_narrative_unit_index: Some(scene.narrative_unit_index),
        ..frame_generation_options()
    };
    let plans = service
        .generate_chapter_segment_plan(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "shared_segment_plan",
                &SequenceLlmProvider::new_owned(responses),
                &engine,
                options,
                None,
            ),
            None,
            None,
        )
        .await
        .expect("published scenes must not require resumable workflow checkpoints");
    assert_eq!(plans.len(), 1);
    let segments = plans[0].metadata_json.as_ref().unwrap()["segments"]
        .as_array()
        .unwrap();
    assert_eq!(
        segments
            .iter()
            .map(|segment| segment["source_shot_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        expected_shots
    );
}

#[tokio::test]
async fn segment_plan_publishes_each_call_and_resumes_after_cancellation() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let options = ChapterVideoPromptOptions {
        workflow_run_id: Some(chapter.id * 100 + 1),
        ..frame_generation_options()
    };
    let cancel_token = CancellationToken::new();
    let progress_cancel = cancel_token.clone();
    let mut progress = move |update: VideoPromptProgressUpdate| -> Result<(), AppError> {
        if update.stage_code == "video_prompt_item_committed"
            && update.item_key.as_deref() == Some("frame_segment_plan:0")
        {
            progress_cancel.cancel();
        }
        Ok(())
    };

    let error = service
        .generate_chapter_segment_plan(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "shared_segment_plan",
                &SequenceLlmProvider::new_owned(vec![
                    segment_plan_json_for_shot(1, None),
                    segment_plan_json_for_shot(2, None),
                ]),
                &engine,
                options.clone(),
                Some(&cancel_token),
            ),
            None,
            Some(&mut progress),
        )
        .await
        .expect_err("the test cancels after the first published shot");
    assert!(matches!(error, AppError::Cancelled));

    let plans = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(plans.len(), 1);
    let metadata = plans[0].metadata_json.as_ref().unwrap();
    assert_eq!(metadata["plan_complete"], json!(false));
    assert_eq!(metadata["segments"].as_array().unwrap().len(), 1);
    assert_eq!(metadata["segment_recipes"].as_array().unwrap().len(), 1);
    assert!(metadata["video_execution_identity"].is_object());

    let resumed = service
        .generate_chapter_segment_plan(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "shared_segment_plan",
                &SequenceLlmProvider::new_owned(vec![segment_plan_json_for_shot(2, None)]),
                &engine,
                options,
                None,
            ),
            None,
            None,
        )
        .await
        .expect("resume must reuse the first call and request only the remaining shot");
    assert_eq!(resumed.len(), 1);
    let metadata = resumed[0].metadata_json.as_ref().unwrap();
    assert_eq!(metadata["plan_complete"], json!(true));
    assert_eq!(metadata["segments"].as_array().unwrap().len(), 2);
}
