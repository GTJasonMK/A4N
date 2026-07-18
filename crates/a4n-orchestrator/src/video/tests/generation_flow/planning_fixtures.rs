fn frame_generation_options() -> ChapterVideoPromptOptions {
    ChapterVideoPromptOptions {
        style: "电影感写实".to_string(),
        prompt_output_language: PromptOutputLanguage::ZhCn,
        prompt_batch_size: 12,
        duration_secs: 4,
        duration_secs_options: vec![4],
        aspect_ratio: "16:9".to_string(),
        reference_image_ids: Vec::new(),
        video_library_image_ids: Vec::new(),
        duration_capability: None,
        regenerate_existing: false,
        workflow_run_id: None,
    }
}

fn video_generation_options_with_batch_size(prompt_batch_size: i32) -> ChapterVideoPromptOptions {
    let mut options = frame_generation_options();
    options.prompt_batch_size = prompt_batch_size;
    options
}

fn english_video_generation_options_with_batch_size(
    prompt_batch_size: i32,
) -> ChapterVideoPromptOptions {
    let mut options = video_generation_options_with_batch_size(prompt_batch_size);
    options.prompt_output_language = PromptOutputLanguage::En;
    options
}

fn regenerate_generation_options() -> ChapterVideoPromptOptions {
    let mut options = frame_generation_options();
    options.regenerate_existing = true;
    options
}

fn regenerate_generation_options_with_duration(duration_secs: i32) -> ChapterVideoPromptOptions {
    let mut options = regenerate_generation_options();
    options.duration_secs = duration_secs;
    options
}

fn single_segment_plan_json(duration_secs: i32) -> String {
    let mut value = segment_plan_fixture();
    value["segments"].as_array_mut().unwrap().truncate(1);
    value["segments"][0]["performance_plan"]["time_blocks"][0]["end_sec"] = json!(duration_secs);
    value.to_string()
}

fn segment_plan_json_for_shot(shot_id: i32, duration_secs: Option<i32>) -> String {
    let value = segment_plan_fixture();
    let source_shot_id = format!("shot-{shot_id}");
    let segments = value["segments"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|segment| segment["source_shot_id"].as_str() == Some(source_shot_id.as_str()))
        .map(|segment| {
            let mut segment = segment.clone();
            if let Some(duration_secs) = duration_secs {
                segment["performance_plan"]["time_blocks"][0]["end_sec"] = json!(duration_secs);
            }
            segment
        })
        .collect::<Vec<_>>();
    json!({ "segments": segments }).to_string()
}

fn frame_prompt_json(frame_number: usize) -> String {
    let value: serde_json::Value = serde_json::from_str(FRAME_PROMPTS_JSON).unwrap();
    let mut frame = value["frames"][frame_number - 1].clone();
    if let Some(frame) = frame.as_object_mut() {
        frame.remove("frame_number");
        frame.remove("duration_secs");
        frame.remove("aspect_ratio");
        frame.remove("segment_numbers");
        frame.remove("reference_image_ids");
    }
    json!({ "frames": [frame] }).to_string()
}

fn boundary_frame_prompt_replies() -> Vec<String> {
    vec![
        frame_prompt_json(1),
        frame_prompt_json(2),
        frame_prompt_json(3),
    ]
}

fn boundary_frame_generation_replies() -> Vec<String> {
    let mut replies = vec![
        segment_plan_json_for_shot(1, None),
        segment_plan_json_for_shot(2, None),
    ];
    replies.extend(boundary_frame_prompt_replies());
    replies
}

fn single_segment_frame_prompt_replies() -> Vec<String> {
    (1..=2).map(frame_prompt_json).collect()
}
