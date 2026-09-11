use a4n_domain::prompt_language::PromptOutputLanguage;

fn frame_generation_options() -> ChapterVideoPromptOptions {
    ChapterVideoPromptOptions {
        style: "电影感写实".to_string(),
        prompt_output_language: PromptOutputLanguage::ZhCn,
        prompt_batch_size: 12,
        segment_plan_prompt_id: None,
        target_shot_number: None,
        target_narrative_unit_index: None,
        segment_recipe: None,
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

async fn target_generation_options(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    segment_number: i32,
) -> ChapterVideoPromptOptions {
    let plan = service
        .current_shared_segment_plan(project_id, chapter_id)
        .await
        .expect("test segment plan should exist");
    let metadata = plan
        .metadata_json
        .as_ref()
        .expect("test segment plan should include metadata");
    let segment = serde_json::from_value::<Vec<a4n_video_contract::VideoSegmentPlanV2>>(
        metadata
            .get("segments")
            .cloned()
            .expect("test segment plan should include segments"),
    )
    .unwrap()
    .into_iter()
    .find(|segment| segment.segment_number == segment_number)
    .expect("test segment should exist");
    let recipe = a4n_video_contract::video_segment_recipes_for_plan_from_metadata(
        Some(metadata),
        plan.style.as_deref().expect("test segment plan style"),
    )
    .unwrap()
    .into_iter()
    .find(|recipe| recipe.segment_number == segment_number)
    .expect("test segment recipe should exist");
    let reference_image_ids = recipe.reference_image_ids.clone();
    ChapterVideoPromptOptions {
        style: recipe.style.clone(),
        prompt_output_language: match recipe.prompt_output_language.as_str() {
            "zh_cn" => PromptOutputLanguage::ZhCn,
            "en" => PromptOutputLanguage::En,
            other => panic!("unexpected test prompt language: {other}"),
        },
        prompt_batch_size: 1,
        segment_plan_prompt_id: Some(plan.id),
        target_shot_number: Some(segment_number),
        target_narrative_unit_index: None,
        segment_recipe: Some(recipe),
        duration_secs: segment.duration_secs,
        duration_secs_options: vec![segment.duration_secs],
        aspect_ratio: segment.aspect_ratio,
        reference_image_ids,
        video_library_image_ids: Vec::new(),
        duration_capability: None,
        regenerate_existing: false,
        workflow_run_id: None,
    }
}

pub(in crate::video) fn segment_plan_json_for_shot(
    shot_id: i32,
    duration_secs: Option<i32>,
) -> String {
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

fn single_segment_frame_prompt_replies() -> Vec<String> {
    (1..=2).map(frame_prompt_json).collect()
}
