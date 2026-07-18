fn video_generation_run_input(project_id: i64) -> RecordStartedGenerationRunInput {
    RecordStartedGenerationRunInput {
        project_id,
        workflow: "generate_chapter_video".to_string(),
        step_key: "video_provider".to_string(),
        title: "章节视频 provider 生成".to_string(),
        model_kind: "video".to_string(),
        provider: Some("test-provider".to_string()),
        model: Some("test-model".to_string()),
        prompt_template_names_json: None,
        prompt_template_hashes_json: None,
        rendered_messages_json: None,
        rendered_prompts_json: None,
        input_variables_json: None,
        source_refs_json: None,
        media_refs_json: None,
        request_json: Some(json!({"request": true})),
        targets: Vec::new(),
    }
}
