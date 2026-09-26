#[test]
fn chat_video_prompt_builder_leaves_input_frames_empty_when_absent() {
    let current_segment = json!({
        "segment_number": 1,
        "duration_secs": 10,
        "source_span_ids": ["ch1:s1:f1"]
    });
    let segment_plan = json!({"segments": [current_segment.clone()]});
    let available_assets = json!([]);
    let reference_ids = json!([]);
    let engine = prompt_engine();

    let chat_prompt =
        super::super::prompts::context::build_chat_video_prompt_user(final_video_context!(
            &engine,
            &segment_plan,
            &json!({"frames": []}),
            &current_segment,
            None,
            &available_assets,
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ))
        .expect("chat video prompt builder should render");

    assert!(!chat_prompt.contains("\"frame_prompt\":"));
    assert!(!chat_prompt.contains("\"blocking_state\":"));
    assert!(!chat_prompt.contains("\"lighting_state\":"));
    let payload: Value =
        serde_json::from_str(a4n_prompt::request::json_user_message_payload(&chat_prompt).unwrap())
            .unwrap();
    assert_eq!(payload["input_frames"], json!({}));
    assert!(chat_prompt.contains("\"input_frame_usage\""));
    assert!(chat_prompt.contains("current_segment_end_state"));
    assert!(chat_prompt.contains("reference_image_ids"));
    assert!(chat_prompt.contains("直接描述本段文字起点、上一段尾态或当前片段开场画面"));
}

#[test]
fn chat_video_prompt_builder_uses_input_frames_when_present() {
    let current_segment = json!({
        "segment_number": 1,
        "duration_secs": 10,
        "source_span_ids": ["ch1:s1:f1"]
    });
    let segment_plan = json!({"segments": [current_segment.clone()]});
    let frame_prompts = json!({
        "frames": [
            {
                "frame_number": 1,
                "scene_summary": "角色站在门外，右手停在门把手旁。",
                "frame_prompt": "门外首帧画面",
                "blocking_state": {"position": "门外偏左"},
                "lighting_state": {"key_light": "左侧月光"}
            },
            {
                "frame_number": 2,
                "scene_summary": "角色停在暗室中央，身体转向门口。",
                "frame_prompt": "室内尾帧画面",
                "blocking_state": {"position": "暗室中央"},
                "lighting_state": {"key_light": "门外冷光"}
            }
        ]
    });
    let available_assets = json!([]);
    let reference_ids = json!([]);
    let engine = prompt_engine();

    let chat_prompt =
        super::super::prompts::context::build_chat_video_prompt_user(final_video_context!(
            &engine,
            &segment_plan,
            &frame_prompts,
            &current_segment,
            None,
            &available_assets,
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ))
        .expect("chat video prompt builder should render");

    assert!(chat_prompt.contains("\"input_frames\""));
    assert!(chat_prompt.contains("\"start_frame\""));
    assert!(chat_prompt.contains("\"end_frame\""));
    assert!(chat_prompt.contains("角色站在门外，右手停在门把手旁"));
    assert!(chat_prompt.contains("角色停在暗室中央，身体转向门口"));
}

#[test]
fn first_frame_video_prompt_builder_uses_only_present_input_frames() {
    let current_segment = json!({
        "segment_number": 1,
        "duration_secs": 10,
        "source_span_ids": ["ch1:s1:f1"]
    });
    let segment_plan = json!({"segments": [current_segment.clone()]});
    let frame_prompts = json!({
        "frames": [
            {
                "frame_number": 1,
                "scene_summary": "角色站在门外，右手停在门把手旁。",
                "frame_prompt": "门外输入首帧画面"
            },
            {
                "frame_number": 2,
                "scene_summary": "角色停在暗室中央，身体转向门口。",
                "frame_prompt": "室内输入尾帧画面"
            }
        ]
    });
    let available_assets = json!([]);
    let reference_ids = json!([]);
    let engine = prompt_engine();

    let first_frame_prompt =
        super::super::prompts::context::build_first_frame_video_prompt_user(final_video_context!(
            &engine,
            &segment_plan,
            &frame_prompts,
            &current_segment,
            None,
            &available_assets,
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ))
        .expect("first-frame video prompt builder should render");

    assert!(first_frame_prompt.contains("\"input_frames\""));
    assert!(first_frame_prompt.contains("\"start_frame\""));
    assert!(first_frame_prompt.contains("\"end_frame\""));
    assert!(first_frame_prompt.contains("角色站在门外，右手停在门把手旁"));
    assert!(first_frame_prompt.contains("角色停在暗室中央，身体转向门口"));

    let second_segment = json!({
        "segment_number": 2,
        "duration_secs": 10,
        "source_span_ids": ["ch1:s2:f1"]
    });
    let two_segment_plan = json!({
        "segments": [current_segment.clone(), second_segment.clone()]
    });
    let first_frame_only = json!({
        "frames": [
            {
                "frame_number": 1,
                "scene_summary": "只有第一段的输入首帧。",
                "frame_prompt": "初始输入首帧画面"
            }
        ]
    });
    let second_prompt =
        super::super::prompts::context::build_first_frame_video_prompt_user(final_video_context!(
            &engine,
            &two_segment_plan,
            &first_frame_only,
            &second_segment,
            None,
            &available_assets,
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ))
        .expect("first-frame second segment prompt builder should render");

    let payload: Value = serde_json::from_str(
        a4n_prompt::request::json_user_message_payload(&second_prompt).unwrap(),
    )
    .unwrap();
    assert_eq!(payload["input_frames"], json!({}));
    assert!(!second_prompt.contains("\"start_frame\""));
    assert!(!second_prompt.contains("\"end_frame\""));
}
