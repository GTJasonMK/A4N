async fn store_first_frame_chain_prompt_seed(
    service: &OrchestratorContext,
    llm_switch: &SwitchableLlmProvider,
    project_id: i64,
    chapter_id: i64,
    prompt_renderer: &dyn a4n_prompt::rendering::PromptRenderer,
) -> chapter_video_prompt::Model {
    service
        .generate_chapter_first_frame_prompts(
            prompt_stage_input(
                llm_switch,
                project_id,
                chapter_id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(vec![
                    segment_plan_json_for_shot(1, None),
                    segment_plan_json_for_shot(2, None),
                    frame_prompt_json(1),
                ]),
                prompt_renderer,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .expect("first-frame seed stage should succeed");

    let rows = service
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                llm_switch,
                project_id,
                chapter_id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(vec![video_shot_json(1)]),
                prompt_renderer,
                video_generation_options_with_batch_size(1),
                None,
            ),
            None,
            None,
        )
        .await
        .expect("first video-prompt seed stage should succeed");

    rows.into_iter()
        .find(|row| row.prompt_type == "video_prompt" && row.shot_number == 1)
        .expect("seeded zh video prompt should be saved")
}
