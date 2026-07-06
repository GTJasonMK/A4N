use super::*;
use crate::prompt_language::PromptOutputLanguage;

const PROMPT_PROJECT_ID: i64 = 1;

fn prompt_engine() -> TeraPromptEngine {
    TeraPromptEngine::from_directory(&prompts_dir()).expect("prompts should load")
}

macro_rules! final_video_context {
    (
        $prompt_engine:expr,
        $segment_plan:expr,
        $frame_prompts:expr,
        $current_segment:expr,
        $previous_completed_shot:expr,
        $available_assets:expr,
        $style:expr,
        $prompt_output_language:expr,
        $duration_secs:expr,
        $aspect_ratio:expr,
        $reference_ids:expr $(,)?
    ) => {
        super::super::prompt_context::FinalVideoPromptContext {
            prompt_engine: $prompt_engine,
            project_id: PROMPT_PROJECT_ID,
            chapter_summary: None,
            chapter_analysis: None,
            segment_plan: $segment_plan,
            frame_prompts: $frame_prompts,
            current_segment: $current_segment,
            previous_completed_shot: $previous_completed_shot,
            available_assets: $available_assets,
            style: $style,
            prompt_output_language: $prompt_output_language,
            duration_secs: $duration_secs,
            aspect_ratio: $aspect_ratio,
            reference_ids: $reference_ids,
        }
    };
}

macro_rules! frame_prompt_context {
    (
        $prompt_engine:expr,
        $segment_plan:expr,
        $available_assets:expr,
        $style:expr,
        $prompt_output_language:expr,
        $aspect_ratio:expr,
        $reference_ids:expr $(,)?
    ) => {
        super::super::prompt_context::FramePromptContext {
            prompt_engine: $prompt_engine,
            project_id: PROMPT_PROJECT_ID,
            chapter_summary: None,
            chapter_analysis: None,
            segment_plan: $segment_plan,
            available_assets: $available_assets,
            style: $style,
            prompt_output_language: $prompt_output_language,
            aspect_ratio: $aspect_ratio,
            reference_ids: $reference_ids,
        }
    };
}

#[test]
fn video_context_character_names_collects_analysis_names() {
    let names = video_context_character_names(Some(&json!({
        "metadata": {
            "characters": ["林岚", {"name": "周澈", "aliases": ["周先生"]}]
        },
        "character_states": {
            "沈砚": {"status": "受伤"}
        },
        "key_events": [{
            "involved_characters": ["林岚", "陈栖"]
        }]
    })));

    assert!(names.contains("林岚"));
    assert!(names.contains("周澈"));
    assert!(names.contains("周先生"));
    assert!(names.contains("沈砚"));
    assert!(names.contains("陈栖"));
}

#[test]
fn video_profile_context_is_visual_compact_without_database_metadata() {
    let now = fixed_time(1);
    let profile = protagonist_profile::Model {
        id: 42,
        project_id: 7,
        name: "林岚".to_string(),
        gender: Some("女".to_string()),
        age: Some("二十岁".to_string()),
        base_attributes: json!({}),
        current_state: json!({
            "explicit_attributes": {
                "appearance": "短发黑衣",
                "role": "调查员"
            },
            "implicit_attributes": {},
            "social_attributes": {}
        }),
        character_arc: None,
        voice_fingerprint: None,
        appearance: Some("短发黑衣".to_string()),
        background: Some("旧城调查员".to_string()),
        personality: Some("冷静".to_string()),
        last_synced_chapter: 3,
        created_at: now,
        updated_at: now,
    };

    let value = video_profile_context(&profile);

    assert_eq!(value["appearance"], json!("短发黑衣"));
    assert_eq!(
        value["current_explicit_attributes"]["role"],
        json!("调查员")
    );
    assert!(value.get("id").is_none());
    assert!(value.get("project_id").is_none());
    assert!(value.get("created_at").is_none());
    assert!(value.get("updated_at").is_none());
    assert!(value.get("base_attributes").is_none());
}

#[test]
fn video_prompt_builders_require_sectioned_api_prompt_text() {
    let segment_plan = json!({"segments": []});
    let current_segment = json!({"segment_number": 1, "duration_secs": 10});
    let available_assets = json!([]);
    let reference_ids = json!([]);
    let engine = prompt_engine();

    let boundary_prompt =
        super::super::prompt_context::build_boundary_video_prompt_user(final_video_context!(
            &engine,
            &segment_plan,
            &json!({"frames": []}),
            &current_segment,
            None,
            &available_assets,
            "风格模板：京阿尼动画风格",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ))
        .expect("boundary prompt builder should render");
    assert!(boundary_prompt.contains("[主体定义与素材绑定]"));
    assert!(boundary_prompt.contains("[镜头分镜]"));
    assert!(boundary_prompt.contains("[对白与表演]"));
    assert!(boundary_prompt.contains("风格模板：京阿尼动画风格"));
    assert!(boundary_prompt.contains("不要配背景音乐"));

    let first_frame_prompt =
        super::super::prompt_context::build_first_frame_video_prompt_user(final_video_context!(
            &engine,
            &segment_plan,
            &json!({"frames": []}),
            &current_segment,
            None,
            &available_assets,
            "风格模板：京阿尼动画风格",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ))
        .expect("first-frame prompt builder should render");
    assert!(first_frame_prompt.contains("\"input_frame_rule\""));
    assert!(first_frame_prompt.contains("\"input_frame_usage\""));
    assert!(first_frame_prompt.contains("current_segment_end_state"));
    assert!(first_frame_prompt.contains("[镜头分镜]"));
    assert!(first_frame_prompt.contains("风格模板：京阿尼动画风格"));
    assert!(first_frame_prompt.contains("不要配背景音乐"));
}

#[test]
fn video_prompt_builders_share_final_prompt_contract() {
    let segment_plan = json!({"segments": []});
    let current_segment = json!({
        "segment_number": 1,
        "duration_secs": 10,
        "source_span_ids": ["ch1:s1:f1"]
    });
    let available_assets = json!([]);
    let reference_ids = json!([]);
    let engine = prompt_engine();

    let boundary_prompt =
        super::super::prompt_context::build_boundary_video_prompt_user(final_video_context!(
            &engine,
            &segment_plan,
            &json!({"frames": []}),
            &current_segment,
            None,
            &available_assets,
            "风格模板：京阿尼动画风格",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ))
        .expect("boundary prompt builder should render");

    let first_frame_prompt =
        super::super::prompt_context::build_first_frame_video_prompt_user(final_video_context!(
            &engine,
            &segment_plan,
            &json!({"frames": []}),
            &current_segment,
            None,
            &available_assets,
            "风格模板：京阿尼动画风格",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ))
        .expect("first-frame prompt builder should render");

    let chat_prompt =
        super::super::prompt_context::build_chat_video_prompt_user(final_video_context!(
            &engine,
            &segment_plan,
            &json!({"frames": []}),
            &current_segment,
            None,
            &available_assets,
            "风格模板：京阿尼动画风格",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ))
        .expect("chat prompt builder should render");

    for prompt in [&boundary_prompt, &first_frame_prompt, &chat_prompt] {
        assert!(prompt.contains("\"prompt_text_section_rule\""));
        assert!(prompt.contains("\"prompt_density_rule\""));
        assert!(prompt.contains("\"safe_visual_rewrite_rule\""));
        assert!(prompt.contains("\"reference_display_label_rule\""));
        assert!(prompt.contains("\"natural_language_no_internal_fields_rule\""));
        assert!(prompt.contains("最终自然语言主提示词"));
        assert!(prompt.contains("duration_secs 内自然完成且能被观众读懂的动作"));
        assert!(prompt.contains("不要配背景音乐"));
    }
}

#[test]
fn video_prompt_review_builders_share_review_contract() {
    let segment_plan = json!({"segments": []});
    let current_segment = json!({
        "segment_number": 1,
        "duration_secs": 10,
        "source_span_ids": ["ch1:s1:f1"]
    });
    let available_assets = json!([]);
    let reference_ids = json!([]);
    let draft_prompt = json!({"shots": [{"shot_number": 1}]});
    let engine = prompt_engine();

    let boundary_review = super::super::prompt_context::build_boundary_video_prompt_review_user(
        final_video_context!(
            &engine,
            &segment_plan,
            &json!({"frames": []}),
            &current_segment,
            None,
            &available_assets,
            "风格模板：京阿尼动画风格",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ),
        &draft_prompt,
    )
    .expect("boundary review builder should render");

    let first_frame_review =
        super::super::prompt_context::build_first_frame_video_prompt_review_user(
            final_video_context!(
                &engine,
                &segment_plan,
                &json!({"frames": []}),
                &current_segment,
                None,
                &available_assets,
                "风格模板：京阿尼动画风格",
                PromptOutputLanguage::ZhCn,
                10,
                "16:9",
                &reference_ids,
            ),
            &draft_prompt,
        )
        .expect("first-frame review builder should render");

    let chat_review = super::super::prompt_context::build_chat_video_prompt_review_user(
        final_video_context!(
            &engine,
            &segment_plan,
            &json!({"frames": []}),
            &current_segment,
            None,
            &available_assets,
            "风格模板：京阿尼动画风格",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ),
        &draft_prompt,
    )
    .expect("chat review builder should render");

    for prompt in [&boundary_review, &first_frame_review, &chat_review] {
        assert!(prompt.contains("\"prompt_review_rule\""));
        assert!(prompt.contains("\"safe_visual_rewrite_rule\""));
        assert!(prompt.contains("\"prompt_text_section_rule\""));
        assert!(prompt.contains("最终自然语言主提示词"));
        assert!(prompt.contains("不要解释修订过程"));
    }
}

#[test]
fn chat_video_prompt_builder_leaves_input_frames_empty_when_absent() {
    let segment_plan = json!({"segments": []});
    let current_segment = json!({
        "segment_number": 1,
        "duration_secs": 10,
        "source_span_ids": ["ch1:s1:f1"]
    });
    let available_assets = json!([]);
    let reference_ids = json!([]);
    let engine = prompt_engine();

    let chat_prompt =
        super::super::prompt_context::build_chat_video_prompt_user(final_video_context!(
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
    assert!(chat_prompt.contains("\"input_frames\": {}"));
    assert!(chat_prompt.contains("\"input_frame_usage\""));
    assert!(chat_prompt.contains("current_segment_end_state"));
    assert!(chat_prompt.contains("reference_image_ids"));
    assert!(chat_prompt.contains("直接描述本段文字起点、上一段尾态或当前片段开场画面"));
}

#[test]
fn chat_video_prompt_builder_uses_input_frames_when_present() {
    let segment_plan = json!({"segments": []});
    let current_segment = json!({
        "segment_number": 1,
        "duration_secs": 10,
        "source_span_ids": ["ch1:s1:f1"]
    });
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
        super::super::prompt_context::build_chat_video_prompt_user(final_video_context!(
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
    let segment_plan = json!({"segments": []});
    let current_segment = json!({
        "segment_number": 1,
        "duration_secs": 10,
        "source_span_ids": ["ch1:s1:f1"]
    });
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
        super::super::prompt_context::build_first_frame_video_prompt_user(final_video_context!(
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
        super::super::prompt_context::build_first_frame_video_prompt_user(final_video_context!(
            &engine,
            &segment_plan,
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

    assert!(second_prompt.contains("\"input_frames\": {}"));
    assert!(!second_prompt.contains("\"start_frame\""));
    assert!(!second_prompt.contains("\"end_frame\""));
}

#[test]
fn frame_prompt_builders_require_sectioned_image_prompt_text() {
    let segment_plan = json!({"segments": []});
    let available_assets = json!([]);
    let reference_ids = json!([]);
    let engine = prompt_engine();

    let single_frame_prompt = super::super::prompt_context::build_single_frame_prompt_user(
        frame_prompt_context!(
            &engine,
            &segment_plan,
            &available_assets,
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            "16:9",
            &reference_ids,
        ),
        &json!({
            "frame_number": 2,
            "expected_segment_numbers": [1, 2],
            "required_duration_secs": 5,
            "required_aspect_ratio": "16:9",
            "previous_frame": {
                "scene_summary": "角色在门外",
                "blocking_state": {"position": "门外"},
                "lighting_state": {"key_light": "月光"},
                "frame_prompt": "角色站在门外。"
            }
        }),
    )
    .expect("single boundary frame prompt builder should render");
    assert!(single_frame_prompt.contains("【参考图约束】"));
    assert!(single_frame_prompt.contains("【构图光线风格】"));
    assert!(single_frame_prompt.contains("每个原始片段都有独立首帧和独立尾帧"));
    assert!(single_frame_prompt.contains("constraints.reference_slots"));
    assert!(single_frame_prompt.contains("不得使用紧贴数字的参考图/图片编号写法"));
    assert!(single_frame_prompt.contains("required_frame_count"));
    assert!(single_frame_prompt.contains("focused_chapter_video_segment_plan"));
    assert!(!single_frame_prompt.contains("completed_chapter_video_frame_prompts"));
    assert!(single_frame_prompt.contains("current_frame"));
    assert!(single_frame_prompt.contains("只输出 frames 数组中的 1 条"));
    assert!(single_frame_prompt.contains("previous_frame"));
    assert!(single_frame_prompt.contains("style_template_verbatim"));
    assert!(!single_frame_prompt.contains("frame_number_rule"));

    let initial_frame_prompt = super::super::prompt_context::build_first_frame_prompt_user(
        frame_prompt_context!(
            &engine,
            &segment_plan,
            &available_assets,
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            "16:9",
            &reference_ids,
        ),
        "角色推门进入暗室。",
    )
    .expect("initial first-frame prompt builder should render");
    assert!(initial_frame_prompt.contains("【参考图约束】"));
    assert!(initial_frame_prompt.contains("【一致性与避免项】"));
    assert!(initial_frame_prompt.contains("style_template_verbatim"));
    assert!(initial_frame_prompt.contains("不要自行精简、概括"));
}

#[test]
fn planning_prompt_builders_include_safe_visual_rewrite_rule() {
    let source_spans = Vec::new();
    let engine = prompt_engine();
    let context = super::super::prompt_context::VideoPlanningPromptContext {
        prompt_engine: &engine,
        project_id: PROMPT_PROJECT_ID,
        chapter_summary: None,
        chapter_analysis: None,
        project_context: None,
        style: "电影感写实",
        duration_secs: 5,
        aspect_ratio: "16:9",
    };
    let beat_prompt =
        super::super::prompt_context::build_beat_extraction_user(context, &source_spans, 1, 1)
            .expect("beat extraction prompt builder should render");
    assert!(beat_prompt.contains("safe_visual_rewrite_rule"));
    assert!(beat_prompt.contains("危险物停在安全距离外"));
    assert!(beat_prompt.contains("不得把极端暴力执行瞬间"));

    let current_shot = json!({
        "shot_id": 1,
        "action_beats": [
            {
                "beat_id": 1,
                "beat_type": "movement",
                "natural_seconds": 5.0,
                "required": true,
                "visible_action": "角色发现线索",
                "source_span_ids": ["ch1:s1:f1"]
            }
        ]
    });
    let segment_prompt = super::super::prompt_context::build_segment_plan_for_shot_user(
        context,
        &json!({"cinematic_shots": []}),
        &current_shot,
        &source_spans,
    )
    .expect("segment plan prompt builder should render");
    assert!(segment_prompt.contains("safe_visual_rewrite_rule"));
    assert!(segment_prompt.contains("非血腥痕迹"));
    assert!(segment_prompt.contains("video_intent.hook"));
}
