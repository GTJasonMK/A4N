use super::*;
use crate::chapter::source_spans::source_spans_for_content;
use a4n_engine::prompt::tera_engine::TeraPromptEngine;
use std::path::Path;

const PROJECT_ID: i64 = 1;

fn planning_context<'a>(
    prompt_engine: &'a dyn PromptEngine,
    style: &'a str,
    duration_secs: i32,
) -> VideoPlanningPromptContext<'a> {
    VideoPlanningPromptContext {
        prompt_engine,
        project_id: PROJECT_ID,
        chapter_summary: None,
        chapter_analysis: None,
        project_context: None,
        style,
        duration_secs,
        aspect_ratio: "16:9",
    }
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
        FinalVideoPromptContext {
            prompt_engine: $prompt_engine,
            project_id: PROJECT_ID,
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
        FramePromptContext {
            prompt_engine: $prompt_engine,
            project_id: PROJECT_ID,
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

fn prompt_engine() -> TeraPromptEngine {
    let prompts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    TeraPromptEngine::from_directory(&prompts_dir).expect("prompts should load")
}

fn video_user_payload(user: &str) -> Value {
    let payload = user
        .split_once("\n\n输入：\n")
        .map(|(_, payload)| payload)
        .expect("video user prompt should include payload delimiter");
    serde_json::from_str(payload.trim()).expect("video user payload should be JSON")
}

fn video_user_cache_prefix(user: &str) -> &str {
    user.split_once("\n\n输入：\n")
        .map(|(prefix, _)| prefix)
        .expect("video user prompt should include payload delimiter")
}

fn contains_json_key(value: &Value, target: &str) -> bool {
    match value {
        Value::Array(items) => items.iter().any(|item| contains_json_key(item, target)),
        Value::Object(object) => {
            object.contains_key(target)
                || object
                    .values()
                    .any(|value| contains_json_key(value, target))
        }
        _ => false,
    }
}

#[test]
fn video_user_prompt_forbids_markdown_fenced_json() {
    let engine = prompt_engine();
    let user = render_strict_json_user_prompt(
        &engine,
        PROJECT_ID,
        "测试视频 JSON 输出",
        &json!({
            "task": "test",
            "output_contract": {"format": "json_object"}
        }),
    )
    .expect("video user prompt should render");

    let prefix = video_user_cache_prefix(&user);
    assert!(prefix.contains("第一个字符必须是 `{`"));
    assert!(prefix.contains("最后一个非空字符必须是 `}`"));
    assert!(prefix.contains("禁止以 Markdown 代码围栏包裹 JSON"));
}

#[test]
fn segment_plan_user_includes_time_budget_rule() {
    let source_spans = source_spans_for_content(1, 1, "他推门进入，停住，低声问话。");
    let engine = prompt_engine();
    let shot_analysis = json!({"cinematic_shots": []});
    let current_shot = json!({
        "shot_id": 1,
        "action_beats": [
            {
                "beat_id": 1,
                "beat_type": "movement",
                "natural_seconds": 3.0,
                "required": true,
                "visible_action": "他推门进入",
                "source_span_ids": ["ch1:s1:f1"]
            }
        ]
    });
    let user = build_segment_plan_for_shot_user(
        planning_context(&engine, "电影感写实", 10),
        &shot_analysis,
        &current_shot,
        &source_spans,
    )
    .expect("segment plan user prompt should render");

    assert!(user.contains("\"time_budget_rule\""));
    assert!(user.contains("不要机械套用固定事件数量"));
    assert!(user.contains("不得改用其他时长"));
    let payload = video_user_payload(&user);
    assert_eq!(payload["constraints"]["selected_duration_secs"], json!(10));
    assert!(
        payload["constraints"]["default_duration_secs"].is_null(),
        "segment planning should not expose a separate default duration"
    );
    assert!(
        !contains_json_key(&payload, "duration_secs_options"),
        "segment planning should not expose selectable duration options to the model"
    );
    assert!(
        !contains_json_key(&payload, "model_duration_capability"),
        "segment planning should not expose provider duration capability to the model"
    );
    assert!(
        payload["constraints"]["duration_selection_rule"]
            .as_str()
            .is_some_and(|rule| rule.contains("必须等于 constraints.selected_duration_secs"))
    );
    let required_fields = payload["output_contract"]["required_segment_fields"]
        .as_array()
        .expect("required_segment_fields should be array");
    assert!(
        required_fields
            .iter()
            .any(|field| field.as_str() == Some("performance_plan")),
        "segment plan contract should require performance_plan"
    );
    let action_script_rule = payload["output_contract"]["performance_action_script_rule"]
        .as_str()
        .expect("segment plan contract should include action_script rule");
    for required in ["character", "body", "movement", "hands", "gaze", "timing"] {
        assert!(
            action_script_rule.contains(required),
            "action_script rule should mention required field {required}"
        );
    }
    assert!(
        action_script_rule.contains("非空字符串")
            && action_script_rule.contains("禁止空字符串")
            && action_script_rule.contains("null")
            && action_script_rule.contains("省略字段"),
        "action_script rule should reject empty or missing field values"
    );
    assert!(
        action_script_rule.contains("手未接触道具") && action_script_rule.contains("双手保持稳定"),
        "hands rule should require visible idle hand state when there is no obvious hand action"
    );
}

#[test]
fn beat_extraction_user_enforces_atomic_action_beats() {
    let source_spans = source_spans_for_content(1, 1, "林寻从黑暗中坐起，手撑地，感受到血迹。");
    let engine = prompt_engine();
    let user = build_beat_extraction_user(
        planning_context(&engine, "电影感写实", 5),
        &source_spans,
        1,
        1,
    )
    .expect("beat extraction user prompt should render");

    assert!(user.contains("\"beat_granularity_rule\""));
    assert!(user.contains("action_beat 应优先保持可观察"));
    assert!(user.contains("如果它们属于同一连续表演动作"));
    assert!(user.contains("角色用手撑地坐起"));
    assert!(user.contains("beats"));
    assert!(user.contains("不要输出 beat_id、beat_type"));
}

#[test]
fn beat_extraction_user_exposes_only_current_window_source_span_ids() {
    let source_spans = source_spans_for_content(1, 1, "林寻从黑暗中坐起。门外传来脚步声。");
    let engine = prompt_engine();
    let user = build_beat_extraction_user(
        planning_context(&engine, "电影感写实", 5),
        &source_spans[..1],
        1,
        2,
    )
    .expect("beat extraction user prompt should render");
    let payload = video_user_payload(&user);
    let allowed = payload["window"]["allowed_source_span_ids"]
        .as_array()
        .expect("allowed source span ids should be present");

    assert_eq!(allowed.as_slice(), [json!("ch1:s1:f1")]);
    assert!(payload["output_contract"]["allowed_source_span_ids"].is_null());
    assert!(
        payload["output_contract"]["beat_number_rule"]
            .as_str()
            .is_some_and(|rule| rule.contains("window.allowed_source_span_ids"))
    );
    assert!(!user.contains("ch1:s2:f1"));
}

#[test]
fn beat_extraction_user_keeps_dynamic_payload_after_cache_prefix() {
    let engine = prompt_engine();
    let first_text = "林寻从黑暗中坐起，手撑地，感受到血迹。";
    let second_text = "门外传来脚步声，林寻屏住呼吸，把匕首藏进袖口。";
    let style = "电影感写实";
    let first_user = build_beat_extraction_user(
        planning_context(&engine, style, 5),
        &source_spans_for_content(1, 1, first_text),
        1,
        1,
    )
    .expect("first beat extraction user prompt should render");
    let second_user = build_beat_extraction_user(
        planning_context(&engine, style, 5),
        &source_spans_for_content(1, 1, second_text),
        1,
        1,
    )
    .expect("second beat extraction user prompt should render");

    let first_prefix = video_user_cache_prefix(&first_user);
    let second_prefix = video_user_cache_prefix(&second_user);
    assert_eq!(
        first_prefix, second_prefix,
        "video user prompt prefix before JSON payload must stay cache-stable"
    );
    for dynamic_value in [
        first_text,
        second_text,
        style,
        "\"duration_secs\": 5",
        "\"16:9\"",
    ] {
        assert!(
            !first_prefix.contains(dynamic_value) && !second_prefix.contains(dynamic_value),
            "dynamic value leaked into cache prefix: {dynamic_value}"
        );
    }
    assert!(first_user.contains("林寻从黑暗中坐起"));
    assert!(second_user.contains("门外传来脚步声"));
}

#[test]
fn video_prompt_user_includes_action_load_and_temporal_capacity_rules() {
    let engine = prompt_engine();
    let user = build_boundary_video_prompt_user(final_video_context!(
        &engine,
        &json!({"segments": []}),
        &json!({"frames": []}),
        &json!({"segment_number": 1}),
        None,
        &json!([]),
        "风格模板：京阿尼动画风格",
        PromptOutputLanguage::ZhCn,
        10,
        "16:9",
        &json!([]),
    ))
    .expect("video prompt user prompt should render");

    assert!(user.contains("\"action_load_rule\""));
    assert!(user.contains("\"style_preservation_rule\""));
    assert!(user.contains("风格模板：京阿尼动画风格"));
    assert!(user.contains("不要配背景音乐"));
    assert!(user.contains("duration_secs 内自然完成且能被观众读懂的动作"));
    assert!(user.contains("中文输出密度：视频 visual_prompt 至少 80 个字符"));
    assert!(user.contains("最后一个镜头必须稳定到本段结尾状态"));
    let payload = video_user_payload(&user);
    assert_eq!(payload["constraints"]["selected_duration_secs"], json!(10));
    assert!(
        !contains_json_key(&payload, "duration_secs_options"),
        "final video prompt should not expose selectable duration options"
    );
    assert!(
        !contains_json_key(&payload, "default_duration_secs"),
        "final video prompt should use selected_duration_secs, not default duration"
    );
    assert_eq!(
        payload["output_contract"]["dialogue_cue_allowed_fields"],
        json!(VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS),
        "final video prompt should expose the dialogue cue field whitelist in the structured contract"
    );
    assert!(
        payload["output_contract"]["dialogue_cue_field_rule"]
            .as_str()
            .is_some_and(|rule| rule.contains("dialogue_cue_allowed_fields")),
        "dialogue cue field rule should point the model at the structured whitelist"
    );
}

#[test]
fn final_video_prompt_payload_hides_internal_source_context_fields() {
    let engine = prompt_engine();
    let user = build_boundary_video_prompt_review_user(
        final_video_context!(
            &engine,
            &json!({
                "segments": [{
                    "segment_number": 1,
                    "source_span_ids": ["ch1:s1:f1"],
                    "reference_image_ids": [59],
                    "scene_summary": "角色在教室窗边回头。"
                }]
            }),
            &json!({
                "frames": [{
                    "frame_number": 1,
                    "source_span_ids": ["ch1:s1:f1"],
                    "reference_image_ids": [59],
                    "image_id": 88,
                    "scene_summary": "窗边起点",
                    "frame_prompt": "角色站在窗边"
                }]
            }),
            &json!({
                "segment_number": 1,
                "source_span_ids": ["ch1:s1:f1"],
                "reference_image_ids": [59],
                "key_source_span_ids": ["ch1:s1:f1"],
                "source_span_range": {"start": "ch1:s1:f1", "end": "ch1:s1:f1"},
                "duration_secs": 5,
                "scene_summary": "角色在教室窗边回头。",
                "emotional_arc": {"end": "角色停在窗边"},
                "material_coverage": {"required_entities": ["角色"], "missing_or_weak_references": [], "risk_note": "已有素材"}
            }),
            Some(&json!({
                "shot_number": 1,
                "title": "上一段",
                "scene_summary": "上一段结尾",
                "dialogue_beats": [{
                    "speaker": "角色",
                    "line": "等等",
                    "delivery": "低声",
                    "visible_action": "嘴唇微动",
                    "listener_reaction": "对方停顿",
                    "timing": "结尾",
                    "source_span_ids": ["ch1:s1:f1"],
                    "dialogue_internal_note": "旧版额外字段"
                }],
                "reference_binding": {"bindings": [{"image_id": 59}]}
            })),
            &json!([]),
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            5,
            "16:9",
            &json!([]),
        ),
        &json!({
            "shots": [{
                "title": "初稿",
                "dialogue_beats": [{
                    "speaker": "角色",
                    "line": "等等",
                    "delivery": "低声",
                    "visible_action": "嘴唇微动",
                    "listener_reaction": "对方停顿",
                    "timing": "中段",
                    "source_span_ids": ["ch1:s1:f1"],
                    "dialogue_internal_note": "初稿额外字段"
                }]
            }]
        }),
    )
    .expect("boundary video review user prompt should render");
    let payload = video_user_payload(&user);

    for key in [
        "source_span_id",
        "source_span_ids",
        "key_source_span_ids",
        "source_span_range",
        "reference_image_ids",
        "reference_binding",
        "image_id",
        "dialogue_beats",
        "dialogue_internal_note",
    ] {
        assert!(
            !contains_json_key(&payload, key),
            "final video prompt payload must not expose internal key {key}"
        );
    }
    assert!(
        !user.contains("source_span_id"),
        "final video prompt text must not expose source_span field names"
    );
    assert!(
        contains_json_key(&payload, "dialogue_cues"),
        "final video prompt payload should expose dialogue_cues as the public model field"
    );
}

#[test]
fn video_prompt_user_rejects_segment_without_number() {
    let engine = prompt_engine();
    let err = build_boundary_video_prompt_user(final_video_context!(
        &engine,
        &json!({"segments": []}),
        &json!({"frames": []}),
        &json!({"title": "缺少片段号"}),
        None,
        &json!([]),
        "电影感写实",
        PromptOutputLanguage::ZhCn,
        5,
        "16:9",
        &json!([]),
    ))
    .expect_err("missing segment_number must fail before LLM request construction");

    assert!(
        format!("{err}").contains("缺少有效 segment_number"),
        "unexpected error: {err}"
    );
}

#[test]
fn first_frame_video_prompt_user_uses_first_frame_specific_instruction() {
    let engine = prompt_engine();
    let user = build_first_frame_video_prompt_user(final_video_context!(
        &engine,
        &json!({"segments": [{"segment_number": 1}]}),
        &json!({"frames": [{"frame_number": 1}]}),
        &json!({"segment_number": 1}),
        None,
        &json!([]),
        "电影感写实",
        PromptOutputLanguage::ZhCn,
        5,
        "16:9",
        &json!([]),
    ))
    .expect("first-frame video user prompt should render");
    let payload = video_user_payload(&user);

    assert_eq!(
        payload["task"]["name"],
        json!("chapter_video_first_frame_video_prompt"),
        "first-frame chain should not reuse generic video task"
    );
    assert_eq!(
        payload["output_contract"]["prompt_name"],
        json!("chapter_video_first_frame_video_prompt")
    );
}

#[test]
fn first_frame_video_prompt_payload_uses_positive_input_frame_usage_for_start_only() {
    let engine = prompt_engine();
    let user = build_first_frame_video_prompt_user(final_video_context!(
        &engine,
        &json!({"segments": [{"segment_number": 1}]}),
        &json!({
            "frames": [{
                "frame_number": 1,
                "scene_summary": "角色坐在窗边，看向窗外。",
                "frame_prompt": "窗边首帧画面"
            }]
        }),
        &json!({
            "segment_number": 1,
            "duration_secs": 5,
            "source_span_ids": ["ch1:s1:f1"],
            "emotional_arc": {"end": "角色看向窗外陷入沉思"},
            "continuity_handoff": "角色保持坐姿，视线停在窗外"
        }),
        None,
        &json!([]),
        "电影感写实",
        PromptOutputLanguage::En,
        5,
        "16:9",
        &json!([]),
    ))
    .expect("first-frame video user prompt should render");
    let payload = video_user_payload(&user);

    assert!(
        payload["input_frames"].get("start_frame").is_some(),
        "start frame should be present"
    );
    assert!(
        payload["input_frames"].get("end_frame").is_none(),
        "first-frame chain start-only request must not expose an end_frame placeholder"
    );
    assert_eq!(
        payload["constraints"]["input_frame_usage"]["opening_source"],
        json!("input_frames.start_frame")
    );
    assert_eq!(
        payload["constraints"]["input_frame_usage"]["ending_source"],
        json!("current_segment_end_state")
    );
    assert!(
        payload["constraints"]["input_frame_usage"]["ending_instruction"]
            .as_str()
            .is_some_and(|text| text.contains("current_segment.emotional_arc.end")),
        "ending instruction should point to segment-derived end state"
    );
    assert!(
        payload["constraints"]["input_frame_usage"]["upload_status_rule"]
            .as_str()
            .is_some_and(
                |text| text.contains("资源是否存在") || text.contains("resource availability")
            ),
        "payload should expose the generic upload-status rule"
    );
    assert!(
        payload["constraints"]["input_frame_usage"]["ending_instruction"]
            .as_str()
            .is_some_and(|text| !text.contains("没有输入尾帧") && !text.contains("无输入尾帧")),
        "ending instruction must describe segment-derived end state, not missing tail-frame status"
    );
    assert!(!user.contains("No input end frame"));
}

#[test]
fn first_frame_video_prompt_payload_uses_reference_slots_without_internal_asset_ids() {
    let engine = prompt_engine();
    let user = build_first_frame_video_prompt_user(final_video_context!(
        &engine,
        &json!({"segments": [{"segment_number": 1}]}),
        &json!({
            "frames": [{
                "frame_number": 1,
                "scene_summary": "角色坐在窗边，看向窗外。",
                "frame_prompt": "窗边首帧画面"
            }]
        }),
        &json!({
            "segment_number": 1,
            "duration_secs": 5,
            "source_span_ids": ["ch1:s1:f1"],
            "emotional_arc": {"end": "角色看向窗外陷入沉思"},
            "continuity_handoff": "角色保持坐姿，视线停在窗外"
        }),
        None,
        &json!([
            {
                "image_id": 59,
                "display_order": 1,
                "display_label": "参考图 1",
                "asset_id": "manual_library_asset_1",
                "source_id": "manual_library_asset_1",
                "candidate_label": "manual_library_asset_1",
                "source_display_label": "参考图 1",
                "project_id": 9,
                "profile_id": 7,
                "role": "chapter_material",
                "label": "manual_library_asset_1",
                "description": "林清辞角色参考 manual_library_asset_1",
                "prompt_excerpt": "清晨教室窗边的角色主图",
                "selection_reason": "manual_library",
                "profile_kind": "asset_id 59",
                "is_primary": true,
                "is_favorite": false
            }
        ]),
        "电影感写实",
        PromptOutputLanguage::ZhCn,
        5,
        "16:9",
        &json!([59]),
    ))
    .expect("first-frame video user prompt should render");
    let payload = video_user_payload(&user);

    assert!(payload.get("available_assets").is_none());
    assert!(payload["constraints"].get("reference_image_ids").is_none());
    assert_eq!(
        payload["constraints"]["reference_slots"][0]["slot_label"],
        json!("参考图 1")
    );
    let asset = payload["reference_asset_summaries"]
        .as_array()
        .and_then(|items| items.first())
        .expect("reference asset summary should exist");
    let serialized = serde_json::to_string(asset).expect("asset summary should serialize");

    assert!(asset.get("image_id").is_none());
    assert!(asset.get("display_label").is_none());
    assert!(asset.get("display_order").is_none());
    assert!(asset.get("project_id").is_none());
    assert!(asset.get("profile_id").is_none());
    assert!(asset.get("asset_id").is_none());
    assert!(asset.get("source_id").is_none());
    assert!(asset.get("candidate_label").is_none());
    assert!(asset.get("source_display_label").is_none());
    assert!(!serialized.contains("manual_library_asset_1"));
    assert!(!serialized.contains("asset_id"));
}

#[test]
fn first_frame_video_prompt_review_payload_hides_backend_dialogue_trace_fields() {
    let engine = prompt_engine();
    let user = build_first_frame_video_prompt_review_user(
        FinalVideoPromptContext {
            prompt_engine: &engine,
            project_id: PROJECT_ID,
            chapter_summary: None,
            chapter_analysis: Some(&json!({
                "scene": "清晨教室窗边对白",
                "source_span_ids": ["ch1:s1:f1"],
                "dialogue_beats": [{
                    "speaker": "林清辞",
                    "line": "等等",
                    "delivery": "低声",
                    "visible_action": "嘴唇微动",
                    "listener_reaction": "顾时年停步",
                    "timing": "中段",
                    "source_span_ids": ["ch1:s1:f1"]
                }]
            })),
            segment_plan: &json!({
                "segments": [{
                    "segment_number": 3,
                    "source_span_ids": ["ch1:s1:f3"],
                    "scene_summary": "顾时年在走廊停步回头。"
                }]
            }),
            frame_prompts: &json!({
                "frames": [{
                    "frame_number": 1,
                    "source_span_ids": ["ch1:s1:f1"],
                    "reference_image_ids": [59],
                    "image_id": 88,
                    "scene_summary": "首帧",
                    "frame_prompt": "窗边起始画面"
                }]
            }),
            current_segment: &json!({
                "segment_number": 3,
                "duration_secs": 5,
                "source_span_ids": ["ch1:s1:f3"],
                "key_source_span_ids": ["ch1:s1:f3"],
                "source_span_range": {"start": "ch1:s1:f3", "end": "ch1:s1:f3"},
                "scene_summary": "顾时年在走廊停步回头。",
                "emotional_arc": {"end": "顾时年停步回头"},
                "material_coverage": {"required_entities": ["顾时年"], "missing_or_weak_references": [], "risk_note": "已有素材"}
            }),
            previous_completed_shot: Some(&json!({
                "shot_number": 2,
                "title": "上一段",
                "scene_summary": "林清辞在窗边低声喊住顾时年。",
                "dialogue_beats": [{
                    "speaker": "林清辞",
                    "line": "等等",
                    "delivery": "低声",
                    "visible_action": "嘴唇微动",
                    "listener_reaction": "顾时年停步",
                    "timing": "结尾",
                    "source_span_ids": ["ch1:s1:f2"],
                    "dialogue_internal_note": "后端保存字段"
                }],
                "reference_binding": {"bindings": [{"image_id": 59}]}
            })),
            available_assets: &json!([]),
            style: "电影感写实",
            prompt_output_language: PromptOutputLanguage::ZhCn,
            duration_secs: 5,
            aspect_ratio: "16:9",
            reference_ids: &json!([]),
        },
        &json!({
            "shots": [{
                "title": "初稿",
                "scene_summary": "顾时年停步回头。",
                "visual_prompt": "顾时年停步回头。",
                "motion_prompt": "镜头轻微推进。",
                "negative_prompt": "",
                "dialogue_beats": [{
                    "speaker": "顾时年",
                    "line": "怎么了？",
                    "delivery": "压低声音",
                    "visible_action": "转头时嘴唇微动",
                    "listener_reaction": "林清辞抬眼",
                    "timing": "中段",
                    "source_span_id": "ch1:s1:f3",
                    "source_span_ids": ["ch1:s1:f3"],
                    "dialogue_internal_note": "旧初稿额外字段"
                }]
            }]
        }),
    )
    .expect("first-frame video review user prompt should render");
    let payload = video_user_payload(&user);

    for key in [
        "source_span_id",
        "source_span_ids",
        "key_source_span_ids",
        "source_span_range",
        "reference_image_ids",
        "reference_binding",
        "image_id",
        "dialogue_internal_note",
    ] {
        assert!(
            !contains_json_key(&payload, key),
            "first-frame review payload must not expose internal key {key}"
        );
    }
    for key in [
        "source_span_id",
        "source_span_ids",
        "key_source_span_ids",
        "source_span_range",
        "dialogue_beats",
        "dialogue_internal_note",
    ] {
        assert!(
            !user.contains(key),
            "first-frame review user prompt text must not expose internal field name {key}"
        );
    }
    assert!(
        contains_json_key(&payload, "dialogue_cues"),
        "first-frame review payload should expose dialogue_cues as the public model field"
    );
}

#[test]
fn first_frame_video_prompt_payload_uses_previous_shot_as_later_segment_opening() {
    let engine = prompt_engine();
    let previous_shot = json!({
        "title": "上一段",
        "scene_summary": "角色停在门口，手握钥匙回头看向走廊。",
        "visual_prompt": "[主体定义与素材绑定]\n角色停在门口。\n\n[片段内容以及前因后果]\n上一段结尾停在门口。\n\n[镜头分镜]\n镜头 1：平视中景保持角色停在门口，手握钥匙并回头看向走廊。\n镜头 2：镜头轻微稳定收束，角色维持门口站位，走廊保持在画面右侧。\n\n[画面风格描述]\n电影感写实。\n\n[连续性与结尾状态]\n角色手握钥匙，走廊在右侧。\n\n[限制与避免项]\n避免字幕。"
    });
    let user = build_first_frame_video_prompt_user(final_video_context!(
        &engine,
        &json!({"segments": [{"segment_number": 1}, {"segment_number": 2}]}),
        &json!({
            "frames": [{
                "frame_number": 1,
                "scene_summary": "第 1 段起始画面",
                "frame_prompt": "第 1 段首帧"
            }]
        }),
        &json!({
            "segment_number": 2,
            "duration_secs": 5,
            "source_span_ids": ["ch1:s1:f2"],
            "emotional_arc": {"end": "角色推门进入"},
            "continuity_handoff": "角色进入门内，钥匙仍在手里"
        }),
        Some(&previous_shot),
        &json!([]),
        "电影感写实",
        PromptOutputLanguage::ZhCn,
        5,
        "16:9",
        &json!([]),
    ))
    .expect("later first-frame video user prompt should render");
    let payload = video_user_payload(&user);

    assert!(
        payload["input_frames"].get("start_frame").is_none(),
        "later first-frame chain prompt should not invent a missing start frame"
    );
    assert_eq!(
        payload["constraints"]["input_frame_usage"]["opening_source"],
        json!("previous_completed_shot")
    );
    assert!(
        payload["constraints"]["input_frame_usage"]["opening_instruction"]
            .as_str()
            .is_some_and(|text| text.contains("不得写资源缺失")),
        "opening instruction should forbid upload-status wording without exposing banned phrases"
    );
    assert!(
        payload["constraints"]["input_frame_usage"]
            .get("forbidden_upload_status_terms")
            .is_none(),
        "payload must not expose exact forbidden terms to the model"
    );
    assert!(!user.contains("previous_completed_shot_or_current_segment"));
}

#[test]
fn first_frame_video_review_uses_dedicated_draft_prompt_field() {
    let engine = prompt_engine();
    let user = build_first_frame_video_prompt_review_user(
        final_video_context!(
            &engine,
            &json!({"segments": [{"segment_number": 1}]}),
            &json!({"frames": [{"frame_number": 1}]}),
            &json!({"segment_number": 1}),
            None,
            &json!([]),
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            5,
            "16:9",
            &json!([]),
        ),
        &json!({"shots": [{"title": "初稿"}]}),
    )
    .expect("first-frame video review user prompt should render");
    let payload = video_user_payload(&user);

    assert_eq!(
        payload["task"]["name"],
        json!("chapter_video_first_frame_video_prompt_review"),
        "first-frame chain review should not reuse generic review task"
    );
    assert!(
        payload["previous_steps"]
            .get("draft_chapter_video_first_frame_video_prompt")
            .is_some(),
        "first-frame review should expose a dedicated draft field"
    );
    assert!(
        payload["previous_steps"]
            .get("draft_chapter_video_prompt")
            .is_none(),
        "first-frame review should not expose the generic draft field"
    );
    assert!(
        payload["constraints"]["shot_number_rule"]
            .as_str()
            .is_some_and(|rule| rule.contains("draft_chapter_video_first_frame_video_prompt")),
        "review rule should reference the same dedicated draft field"
    );
}

#[test]
fn boundary_video_review_uses_generic_draft_prompt_field() {
    let engine = prompt_engine();
    let user = build_boundary_video_prompt_review_user(
        final_video_context!(
            &engine,
            &json!({"segments": [{"segment_number": 1}]}),
            &json!({"frames": [{"frame_number": 1}, {"frame_number": 2}]}),
            &json!({"segment_number": 1}),
            None,
            &json!([]),
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            5,
            "16:9",
            &json!([]),
        ),
        &json!({"shots": [{"title": "初稿"}]}),
    )
    .expect("boundary video review user prompt should render");
    let payload = video_user_payload(&user);

    assert!(
        payload["previous_steps"]
            .get("draft_chapter_video_prompt")
            .is_some(),
        "boundary review should expose the generic draft field"
    );
    assert!(
        payload["previous_steps"]
            .get("draft_chapter_video_first_frame_video_prompt")
            .is_none(),
        "boundary review should not expose the first-frame-chain draft field"
    );
    assert!(
        payload["constraints"]["shot_number_rule"]
            .as_str()
            .is_some_and(|rule| rule.contains("draft_chapter_video_prompt")),
        "boundary review rule should reference the generic draft field"
    );
}

#[test]
fn chat_video_review_uses_chat_draft_prompt_field() {
    let engine = prompt_engine();
    let user = build_chat_video_prompt_review_user(
        final_video_context!(
            &engine,
            &json!({"segments": [{"segment_number": 1}]}),
            &json!({"frames": [{"frame_number": 1}, {"frame_number": 2}]}),
            &json!({"segment_number": 1}),
            None,
            &json!([]),
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            5,
            "16:9",
            &json!([]),
        ),
        &json!({"shots": [{"title": "初稿"}]}),
    )
    .expect("chat video review user prompt should render");
    let payload = video_user_payload(&user);

    assert!(
        payload["previous_steps"]
            .get("draft_chapter_video_chat_prompt")
            .is_some(),
        "chat review should expose the chat draft field"
    );
    assert!(
        payload["previous_steps"]
            .get("draft_chapter_video_prompt")
            .is_none(),
        "chat review should not expose the generic API draft field"
    );
    assert!(
        payload["constraints"]["shot_number_rule"]
            .as_str()
            .is_some_and(|rule| rule.contains("draft_chapter_video_chat_prompt")),
        "chat review rule should reference the chat draft field"
    );
}

#[test]
fn single_frame_prompt_user_exposes_reference_slots_not_internal_ids() {
    let engine = prompt_engine();
    let user = build_single_frame_prompt_user(
        frame_prompt_context!(
            &engine,
            &json!({"segments": [{"segment_number": 1}]}),
            &json!([{
                "description": "林清辞角色参考"
            }]),
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            "16:9",
            &json!([59, 80]),
        ),
        &json!({
            "frame_number": 2,
            "expected_segment_numbers": [1],
            "required_duration_secs": 6,
            "required_aspect_ratio": "16:9",
            "previous_frame_image": {
                "slot_index": 1,
                "slot_label": "参考图 1",
                "usage": "已生成的上一张边界帧画面"
            }
        }),
    )
    .expect("single frame prompt user should render");
    let payload = video_user_payload(&user);
    let constraints = payload["constraints"]
        .as_object()
        .expect("constraints should be an object");

    assert!(constraints.get("reference_image_ids").is_none());
    let slots = constraints
        .get("reference_slots")
        .and_then(Value::as_array)
        .expect("reference slots should be an array");
    assert_eq!(slots[0]["slot_label"], json!("参考图 1"));
    assert_eq!(slots[0]["role"], json!("previous_boundary_frame"));
    assert_eq!(slots[1]["slot_label"], json!("参考图 2"));
    assert!(
        serde_json::to_string(slots)
            .expect("reference slots should serialize")
            .find("59")
            .is_none()
    );
    let available_assets_text = serde_json::to_string(&payload["available_assets"])
        .expect("available assets should serialize");
    assert!(!available_assets_text.contains("候选素材"));
    assert!(!available_assets_text.contains("display_label"));
}

#[test]
fn single_frame_prompt_user_rejects_duplicate_backend_reference_ids() {
    let engine = prompt_engine();
    let err = build_single_frame_prompt_user(
        frame_prompt_context!(
            &engine,
            &json!({"segments": [{"segment_number": 1}]}),
            &json!([]),
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            "16:9",
            &json!([59, 59]),
        ),
        &json!({
            "frame_number": 1,
            "expected_segment_numbers": [1],
            "required_duration_secs": 6,
            "required_aspect_ratio": "16:9"
        }),
    )
    .expect_err("duplicate backend reference ids should fail before LLM request construction");

    assert!(format!("{err}").contains("重复图片 ID：59"));
}

#[test]
fn single_frame_prompt_user_rejects_missing_previous_frame_reference_slot() {
    let engine = prompt_engine();
    let err = build_single_frame_prompt_user(
        frame_prompt_context!(
            &engine,
            &json!({"segments": [{"segment_number": 1}]}),
            &json!([]),
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            "16:9",
            &json!([]),
        ),
        &json!({
            "frame_number": 2,
            "expected_segment_numbers": [1],
            "required_duration_secs": 6,
            "required_aspect_ratio": "16:9",
            "previous_frame_image": {
                "slot_index": 1,
                "slot_label": "参考图 1",
                "usage": "已生成的上一张边界帧画面"
            }
        }),
    )
    .expect_err("previous frame image requires a backend reference slot");

    assert!(format!("{err}").contains("上一帧参考图已存在"));
}

#[test]
fn single_frame_prompt_user_rejects_missing_system_fields() {
    let engine = prompt_engine();
    let err = build_single_frame_prompt_user(
        frame_prompt_context!(
            &engine,
            &json!({"segments": []}),
            &json!([]),
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            "16:9",
            &json!([]),
        ),
        &json!({
            "frame_number": 1,
            "expected_segment_numbers": [1],
            "required_aspect_ratio": "16:9"
        }),
    )
    .expect_err("missing derived duration field must fail before LLM request construction");

    assert!(
        format!("{err}").contains("required_duration_secs"),
        "unexpected error: {err}"
    );
}
