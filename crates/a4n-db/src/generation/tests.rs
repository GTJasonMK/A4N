use super::*;
use serde_json::json;

fn spec_with_params(params_json: Value) -> generation_spec::Model {
    let now = Utc::now().naive_utc();
    generation_spec::Model {
        id: 1,
        project_id: 1,
        root_project_id: 1,
        source_context_kind: Some("snapshot_backed".to_string()),
        source_snapshot_id: Some(1),
        source_chapter_id: Some(1),
        source_snapshot_chapter_id: Some(1),
        canvas_text_asset_id: None,
        target_kind: "video_segment".to_string(),
        target_entity_kind: "segment".to_string(),
        target_entity_id: Some(1),
        status: "ready".to_string(),
        source_version_hash: "source-hash".to_string(),
        params_json,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn usage_trace_fields_calculates_cache_hit_ratio() {
    let fields = usage_trace_fields(Some(&LlmCallUsageTraceInput {
        prompt_tokens: 10,
        completion_tokens: 5,
        total_tokens: 15,
        prompt_cache_hit_tokens: Some(8),
        prompt_cache_miss_tokens: Some(2),
    }))
    .expect("usage trace fields should be valid");

    assert_eq!(fields.prompt_tokens, Some(10));
    assert_eq!(fields.completion_tokens, Some(5));
    assert_eq!(fields.total_tokens, Some(15));
    assert_eq!(fields.prompt_cache_hit_tokens, Some(8));
    assert_eq!(fields.prompt_cache_miss_tokens, Some(2));
    assert_eq!(fields.prompt_cache_hit_ratio, Some(0.8));
}

#[test]
fn usage_trace_fields_rejects_i32_overflow() {
    let err = usage_trace_fields(Some(&LlmCallUsageTraceInput {
        prompt_tokens: i32::MAX as usize + 1,
        completion_tokens: 0,
        total_tokens: i32::MAX as usize + 1,
        prompt_cache_hit_tokens: None,
        prompt_cache_miss_tokens: None,
    }))
    .expect_err("overflowing usage must fail");

    assert!(err.to_string().contains("prompt_tokens"));
}

#[test]
fn generation_ref_normalizers_reject_unknown_values() {
    assert_eq!(
        normalize_media_ref_role(" first_frame ").expect("known media role"),
        "first_frame"
    );
    assert_eq!(
        normalize_entity_kind("character").expect("known entity kind"),
        "character"
    );

    let media_err =
        normalize_media_ref_role("local_path").expect_err("unknown media role must fail");
    assert!(media_err.to_string().contains("media_ref_role 无效"));

    let entity_err =
        normalize_entity_kind("chapter").expect_err("unknown generation entity kind must fail");
    assert!(entity_err.to_string().contains("entity_kind 无效"));
}

#[test]
fn generation_checkpoint_fingerprint_is_stable_sha256_hex() {
    let value = json!({
        "workflow": "image_generation",
        "step": "provider_request",
        "input": {"prompt": "角色站在窗边", "seed": 42}
    });

    let first = generation_checkpoint_fingerprint_sha256(&value).expect("fingerprint");
    let second = generation_checkpoint_fingerprint_sha256(&value).expect("fingerprint");

    assert_eq!(first, second);
    assert_eq!(first.len(), 64);
    assert!(first.chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[test]
fn generation_checkpoint_fingerprint_uses_rfc8785_canonical_json() {
    let first: Value = serde_json::from_str(
        r#"{
            "numbers": [333333333.33333329, 1E30, 4.50, 2e-3, 0.000000000000000000000000001],
            "string": "\u20ac$\u000F\u000aA'\u0042\u0022\u005c\\\"\/",
            "literals": [null, true, false]
        }"#,
    )
    .expect("RFC 8785 example JSON");
    let second: Value = serde_json::from_str(
        r#"{
            "literals": [null, true, false],
            "string": "\u20ac$\u000F\u000aA'\u0042\u0022\u005c\\\"\/",
            "numbers": [333333333.33333329, 1E30, 4.50, 2e-3, 0.000000000000000000000000001]
        }"#,
    )
    .expect("reordered RFC 8785 example JSON");

    assert_eq!(
        canonical_json_text(&first).expect("canonical JSON"),
        r#"{"literals":[null,true,false],"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27],"string":"€$\u000f\nA'B\"\\\\\"/"}"#
    );
    assert_eq!(
        generation_checkpoint_fingerprint_sha256(&first).unwrap(),
        generation_checkpoint_fingerprint_sha256(&second).unwrap()
    );
}

#[test]
fn render_video_prompt_text_uses_current_main_sections() {
    let spec = spec_with_params(json!({
        "input_image_introduction": "从已有首帧自然开始。",
        "segment_context": "角色承接上一段尾态完成一次转身。",
        "time_distribution": ["0-2秒：角色停顿。", "2-5秒：角色转身并稳定。"],
        "visual_style": "清透动画风格，侧逆光。",
        "continuity_and_end_state": "结尾保持同一服装、站位和光源方向。",
        "restrictions_and_avoidance": "不要配背景音乐，不要字幕、水印、空间跳变。"
    }));

    let prompt = render_prompt_text(&spec, "video_prompt")
        .expect("video prompt should render from current schema");

    for section in VIDEO_PROMPT_REQUIRED_SECTIONS {
        assert!(prompt.contains(section), "missing section {section}");
    }
    assert!(!prompt.contains("[声音与对白]"));
    validate_prompt_artifact_text("video_prompt", &prompt)
        .expect("rendered video prompt should pass artifact validation");
}

#[test]
fn render_video_prompt_text_rejects_old_sound_only_schema() {
    let spec = spec_with_params(json!({
        "input_image_introduction": "从已有首帧自然开始。",
        "segment_context": "角色承接上一段尾态完成一次转身。",
        "time_distribution": ["0-2秒：角色停顿。", "2-5秒：角色转身并稳定。"],
        "visual_style": "清透动画风格，侧逆光。",
        "sound_and_dialogue": "无对白。"
    }));

    let err = render_prompt_text(&spec, "video_prompt")
        .expect_err("old video prompt schema must fail explicitly");

    assert!(err.to_string().contains("continuity_and_end_state"));
}

#[test]
fn validate_prompt_artifact_text_rejects_video_without_required_sections() {
    let err = validate_prompt_artifact_text(
        "provider_video_prompt",
        "[主体定义与素材绑定]\n只有图片介绍，没有完整视频主提示词章节。",
    )
    .expect_err("video artifact text must include the current main sections");

    assert!(err.to_string().contains("[片段内容以及前因后果]"));
}

#[test]
fn validate_prompt_artifact_text_allows_image_prompt_without_video_sections() {
    validate_prompt_artifact_text("image_prompt", "清晰写实的单张角色立绘。")
        .expect("image prompt should not require video prompt sections");
}

#[test]
fn append_generation_artifact_target_requires_current_targets_shape() {
    let target = GenerationRunTraceTarget {
        target_kind: "generated_image".to_string(),
        target_table: "generated_image".to_string(),
        target_id: 9,
    };

    let refs = append_generation_artifact_target(None, target.clone())
        .expect("empty artifact refs should create current targets shape");
    assert_eq!(refs["targets"].as_array().map(Vec::len), Some(1));

    let refs = append_generation_artifact_target(
        Some(json!({
            "source": "checkpoint",
            "targets": [{
                "target_kind": "chapter_video_prompt",
                "target_table": "chapter_video_prompt",
                "target_id": 7,
            }],
        })),
        target.clone(),
    )
    .expect("current artifact refs shape should append target");
    assert_eq!(refs["source"], "checkpoint");
    assert_eq!(refs["targets"].as_array().map(Vec::len), Some(2));

    let err = append_generation_artifact_target(Some(json!(["legacy-target"])), target.clone())
        .expect_err("non-object artifact refs must fail explicitly");
    assert!(err.to_string().contains("artifact_refs_json 必须是对象"));

    let err = append_generation_artifact_target(
        Some(json!({
            "targets": {
                "target_kind": "legacy-shape",
            },
        })),
        target,
    )
    .expect_err("non-array targets must fail explicitly");
    assert!(
        err.to_string()
            .contains("artifact_refs_json.targets 必须是数组")
    );
}
