#[tokio::test]
async fn first_frame_chain_frame_prompt_generation_does_not_silently_reuse_existing_prompt() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    let initial_rows = service
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
    let initial_first_frame = initial_rows
        .iter()
        .find(|row| row.prompt_type == "first_frame_prompt")
        .unwrap();

    let mut regenerated_frame: serde_json::Value =
        serde_json::from_str(&frame_prompt_json(1)).unwrap();
    regenerated_frame["frames"][0]["title"] = json!("重新生成的初始首帧");
    let mut progress_events = Vec::new();
    let mut progress = |current: u32, total: u32, message: &str| -> Result<(), AppError> {
        progress_events.push((current, total, message.to_string()));
        Ok(())
    };

    let regenerated_rows = service
        .generate_chapter_first_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(vec![regenerated_frame.to_string()]),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            Some(&mut progress),
        )
        .await
        .unwrap();

    let regenerated_first_frame = regenerated_rows
        .iter()
        .find(|row| row.prompt_type == "first_frame_prompt")
        .unwrap();
    assert_eq!(regenerated_first_frame.id, initial_first_frame.id);
    assert_eq!(
        regenerated_first_frame.title.as_deref(),
        Some("重新生成的初始首帧")
    );
    assert!(
        progress_events
            .iter()
            .any(|(_, _, message)| message.contains("正在生成初始首帧提示词"))
    );
    assert!(
        !progress_events
            .iter()
            .any(|(_, _, message)| message.contains("已复用")),
        "普通生成首帧不能静默复用旧记录，否则界面会显示完成但数量不变"
    );
}
