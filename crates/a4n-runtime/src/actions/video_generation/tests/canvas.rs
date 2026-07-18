use super::*;

#[test]
fn canvas_cliproxy_reference_mode_maps_frame_inputs_to_reference_ids() {
    let mut cfg = video_config(CLIPROXY_OPENAI_VIDEO_BACKEND);
    cfg.provider_options_json = Some(json!({
        "grok_reference_mode": "reference",
    }));
    let input = GenerateCanvasVideoInput {
        user_id: 1,
        project_id: 2,
        graph_id: 3,
        node_id: 4,
        workflow_run_id: None,
        prompt: "画布提示词".to_string(),
        negative_prompt: None,
        config_id: Some(cfg.id),
        width: None,
        height: None,
        duration_secs: None,
        fps: None,
        first_frame_image_id: Some(21),
        last_frame_image_id: Some(22),
        reference_image_ids: vec![31, 21, 32],
        source_node_ids: vec![9],
    };
    let mut note = None;

    let binding = canvas_video_provider_image_binding(&cfg, &input, &mut note)
        .expect("cliproxy reference mode should normalize canvas image inputs");

    assert_eq!(binding.first_frame_image_id, None);
    assert_eq!(binding.last_frame_image_id, None);
    assert_eq!(binding.reference_image_ids, vec![21, 22, 31, 32]);
    let note = note.expect("normalization should explain frame-to-reference mapping");
    assert!(note.contains("参考模式"));
    assert!(note.contains("21, 22, 31, 32"));
}

#[test]
fn canvas_cliproxy_first_frame_mode_keeps_first_frame_slot() {
    let mut cfg = video_config(CLIPROXY_OPENAI_VIDEO_BACKEND);
    cfg.provider_options_json = Some(json!({
        "grok_reference_mode": "first_frame",
    }));
    let input = GenerateCanvasVideoInput {
        user_id: 1,
        project_id: 2,
        graph_id: 3,
        node_id: 4,
        workflow_run_id: None,
        prompt: "画布提示词".to_string(),
        negative_prompt: None,
        config_id: Some(cfg.id),
        width: None,
        height: None,
        duration_secs: None,
        fps: None,
        first_frame_image_id: Some(21),
        last_frame_image_id: None,
        reference_image_ids: vec![],
        source_node_ids: vec![],
    };
    let mut note = None;

    let binding = canvas_video_provider_image_binding(&cfg, &input, &mut note)
        .expect("cliproxy first-frame mode should preserve the first-frame input slot");

    assert_eq!(binding.first_frame_image_id, Some(21));
    assert_eq!(binding.last_frame_image_id, None);
    assert!(binding.reference_image_ids.is_empty());
    assert!(note.is_none());
}
