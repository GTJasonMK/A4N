#[tokio::test]
async fn first_frame_chain_starts_independent_english_chain_without_overwriting_generated_zh() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let zh_video_prompt = store_first_frame_chain_prompt_seed(
        &service,
        &llm_switch,
        project.id,
        chapter.id,
        &engine,
    )
    .await;
    let mut active: chapter_video_prompt::ActiveModel = zh_video_prompt.into();
    active.generated_video_id = Set(Some(9301));
    active.status = Set("completed".to_string());
    active.update(db.as_ref()).await.unwrap();

    let en_video_rows = service
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(vec![english_video_shot_json(1)]),
                &engine,
                english_video_generation_options_with_batch_size(1),
                None,
            ),
            None,
            None,
        )
        .await
        .expect("English generation should start its own serial prompt chain");

    let en_video_prompt = en_video_rows
        .iter()
        .find(|row| {
            row.prompt_type == "video_prompt"
                && row.shot_number == 1
                && row.metadata_json.as_ref().is_some_and(|metadata| {
                    metadata
                        .get("prompt_output_language")
                        .and_then(serde_json::Value::as_str)
                        == Some("en")
                })
        })
        .expect("en video prompt should be saved");
    assert_eq!(en_video_prompt.generated_video_id, None);

    let stored_shot_1 = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("first_frame_chain"))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored_shot_1.len(), 2);
    assert!(stored_shot_1.iter().any(|row| {
        row.generated_video_id == Some(9301)
            && row.metadata_json.as_ref().is_some_and(|metadata| {
                metadata
                    .get("prompt_output_language")
                    .and_then(serde_json::Value::as_str)
                    == Some("zh_cn")
            })
    }));
    assert!(stored_shot_1.iter().any(|row| {
        row.generated_video_id.is_none()
            && row.metadata_json.as_ref().is_some_and(|metadata| {
                metadata
                    .get("prompt_output_language")
                    .and_then(serde_json::Value::as_str)
                    == Some("en")
            })
    }));
}

#[tokio::test]
async fn first_frame_chain_zh_video_prompt_continues_after_generated_zh_video_prompt() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let zh_video_prompt = store_first_frame_chain_prompt_seed(
        &service,
        &llm_switch,
        project.id,
        chapter.id,
        &engine,
    )
    .await;
    let first_prompt_id = zh_video_prompt.id;
    let mut active: chapter_video_prompt::ActiveModel = zh_video_prompt.into();
    active.generated_video_id = Set(Some(9301));
    active.status = Set("completed".to_string());
    active.update(db.as_ref()).await.unwrap();

    let mut options = video_generation_options_with_batch_size(1);
    options.prompt_output_language = PromptOutputLanguage::ZhCn;
    let rows = service
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(vec![video_shot_json(2)]),
                &engine,
                options,
                None,
            ),
            None,
            None,
        )
        .await
        .expect(
            "Chinese generation should continue after an existing Chinese generated video prompt",
        );

    let video_rows = rows
        .iter()
        .filter(|row| row.prompt_type == "video_prompt")
        .collect::<Vec<_>>();
    assert_eq!(video_rows.len(), 2);
    let reused_prompt = video_rows
        .iter()
        .find(|row| row.shot_number == 1)
        .expect("first prompt should be preserved");
    assert_eq!(reused_prompt.id, first_prompt_id);
    assert_eq!(reused_prompt.generated_video_id, Some(9301));
    let generated_next_prompt = video_rows
        .iter()
        .find(|row| row.shot_number == 2)
        .expect("second prompt should be generated");
    assert_eq!(generated_next_prompt.generated_video_id, None);
    assert!(
        generated_next_prompt
            .metadata_json
            .as_ref()
            .is_some_and(|metadata| {
                metadata
                    .get("prompt_output_language")
                    .and_then(serde_json::Value::as_str)
                    == Some("zh_cn")
            })
    );

    let stored_prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("first_frame_chain"))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored_prompts.len(), 2);
    assert_eq!(stored_prompts[0].id, first_prompt_id);
    assert_eq!(stored_prompts[0].generated_video_id, Some(9301));
    assert_eq!(stored_prompts[1].shot_number, 2);
    assert_eq!(stored_prompts[1].generated_video_id, None);
}
