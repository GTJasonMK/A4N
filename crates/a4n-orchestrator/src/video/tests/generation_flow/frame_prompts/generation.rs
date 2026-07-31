#[tokio::test]
async fn frame_prompt_generation_requires_current_shooting_script_before_llm() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter_without_shooting_script(db.as_ref(), project.id, 1).await;

    let err = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new(Vec::new()),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("generation must require a shooting script before any LLM call");

    assert!(matches!(
        err,
        AppError::NotFound(message)
            if message.contains("第 1 章")
                && message.contains(&format!("章节 ID {}", chapter.id))
                && message.contains("未找到与当前正文版本匹配的结构化拍摄剧本")
                && message.contains("正文页")
                && message.contains("剧本改编")
    ));
}

#[tokio::test]
async fn frame_prompt_generation_writes_only_segment_plan_and_boundary_frames() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let llm = SequenceLlmProvider::new_owned(boundary_frame_generation_replies());
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let project_id = project.id;
    let chapter_id = chapter.id;
    let mut progress_events = Vec::new();
    let mut progress = |update: VideoPromptProgressUpdate| -> Result<(), AppError> {
        progress_events.push(update);
        Ok(())
    };

    let rows = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project_id,
                chapter_id,
                "boundary_frames",
                &llm,
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            Some(&mut progress),
        )
        .await
        .unwrap();

    let video_rows = rows
        .iter()
        .filter(|row| row.prompt_type == "video_prompt")
        .collect::<Vec<_>>();
    let frame_rows = rows
        .iter()
        .filter(|row| row.prompt_type == "frame_prompt")
        .collect::<Vec<_>>();

    assert_eq!(video_rows.len(), 0);
    assert_eq!(frame_rows.len(), 3);
    assert!(
        rows.iter()
            .any(|row| row.prompt_type == "frame_segment_plan")
    );
    assert!(progress_events.iter().any(|update| {
        update.total == 100 && update.message.contains("边界帧提示词记录已更新")
    }));
    assert!(progress_events.iter().any(|update| {
        update.stage_code == "video_prompt_item_committed"
            && update.item_key.as_deref() == Some("frame_prompt:1")
    }));
}

#[tokio::test]
async fn video_prompt_generation_counts_boundary_frames_by_generation_context_not_frame_references()
{
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let reference_image = insert_generated_image(db.as_ref(), project.id).await;
    let mut options = frame_generation_options();
    options.reference_image_ids = vec![reference_image.id];

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(boundary_frame_generation_replies()),
                &engine,
                options.clone(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let frame_rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(frame_rows.len(), 3);
    assert!(
        frame_rows
            .iter()
            .all(|row| row.reference_image_ids_json.as_ref() == Some(&json!([reference_image.id])))
    );

    let rows = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(serial_video_shot_replies()),
                &engine,
                options,
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(
        rows.iter()
            .filter(|row| row.prompt_type == "video_prompt")
            .count(),
        2
    );
    let video_rows = rows
        .iter()
        .filter(|row| row.prompt_type == "video_prompt")
        .collect::<Vec<_>>();
    assert!(
        video_rows
            .iter()
            .all(|row| row.reference_image_ids_json.as_ref() == Some(&json!([reference_image.id]))),
        "最终视频提示词应继承本次用户选择的素材参考图，而不是依赖片段规划旧字段"
    );
}
