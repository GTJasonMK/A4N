use super::*;
use a4n_db_models::models::{
    generation::{generation_spec_media_ref, generation_spec_text_anchor},
    media::media_asset,
    project::{project_source_snapshot, text_anchor},
};
use a4n_domain::llm::LlmCallUsageTraceInput;
use sea_orm::{ActiveModelTrait, Set};
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

#[tokio::test]
async fn validate_required_prompt_refs_reads_required_roles_in_one_batch() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "生成意图媒体角色批量校验".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project");
    let now = Utc::now().naive_utc();
    let snapshot = project_source_snapshot::ActiveModel {
        id: Set(1),
        root_project_id: Set(project.id),
        source_version_hash: Set("snapshot-hash".to_string()),
        blueprint_version_id: Set(None),
        chapter_scope_json: Set(json!({})),
        chapter_hashes_json: Set(json!({})),
        profile_hashes_json: Set(json!({})),
        created_at: Set(now),
    }
    .insert(&db)
    .await
    .expect("test snapshot");
    let anchor = text_anchor::ActiveModel {
        id: Set(1),
        project_id: Set(project.id),
        source_chapter_id: Set(None),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_snapshot_chapter_id: Set(None),
        source_kind: Set("chapter".to_string()),
        source_id: Set(1),
        paragraph_index: Set(0),
        start_offset: Set(0),
        end_offset: Set(1),
        quote: Set("锚点".to_string()),
        quote_hash: Set("quote-hash".to_string()),
        source_version_hash: Set("snapshot-hash".to_string()),
        created_at: Set(now),
    }
    .insert(&db)
    .await
    .expect("test anchor");
    let spec = generation_spec::ActiveModel {
        id: Set(1),
        project_id: Set(project.id),
        root_project_id: Set(project.id),
        source_context_kind: Set(Some("snapshot_backed".to_string())),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_chapter_id: Set(None),
        source_snapshot_chapter_id: Set(None),
        canvas_text_asset_id: Set(None),
        target_kind: Set("video_segment".to_string()),
        target_entity_kind: Set("segment".to_string()),
        target_entity_id: Set(Some(1)),
        status: Set("ready".to_string()),
        source_version_hash: Set("snapshot-hash".to_string()),
        params_json: Set(json!({
            "required_media_roles": [" first_frame ", "end_frame"]
        })),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&db)
    .await
    .expect("test generation spec");
    generation_spec_text_anchor::ActiveModel {
        generation_spec_id: Set(spec.id),
        text_anchor_id: Set(anchor.id),
        role: Set("context".to_string()),
    }
    .insert(&db)
    .await
    .expect("test generation anchor ref");
    let asset = media_asset::ActiveModel {
        id: Set(1),
        project_id: Set(project.id),
        root_project_id: Set(project.id),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_chapter_id: Set(None),
        source_snapshot_chapter_id: Set(None),
        media_kind: Set("image".to_string()),
        origin: Set("generated".to_string()),
        file_path: Set(Some("test/first-frame.png".to_string())),
        thumbnail_path: Set(None),
        width: Set(Some(1)),
        height: Set(Some(1)),
        duration_secs: Set(None),
        fps: Set(None),
        file_size: Set(Some(1)),
        checksum: Set(None),
        status: Set("completed".to_string()),
        error_message: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&db)
    .await
    .expect("test media asset");
    generation_spec_media_ref::ActiveModel {
        generation_spec_id: Set(spec.id),
        media_asset_id: Set(asset.id),
        role: Set("first_frame".to_string()),
    }
    .insert(&db)
    .await
    .expect("test first-frame ref");

    let missing = validate_required_prompt_refs(&db, &spec)
        .await
        .expect_err("the second required role must be reported");
    assert!(missing.to_string().contains("缺少必需媒体引用: end_frame"));

    generation_spec_media_ref::ActiveModel {
        generation_spec_id: Set(spec.id),
        media_asset_id: Set(asset.id),
        role: Set("end_frame".to_string()),
    }
    .insert(&db)
    .await
    .expect("test end-frame ref");
    validate_required_prompt_refs(&db, &spec)
        .await
        .expect("all required roles should pass");
}

#[test]
fn usage_trace_fields_calculates_cache_hit_ratio() {
    let fields = usage_trace_fields(Some(&LlmCallUsageTraceInput {
        prompt_tokens: 10,
        completion_tokens: 5,
        total_tokens: 15,
        prompt_cache_hit_tokens: Some(8),
        prompt_cache_miss_tokens: Some(2),
        prompt_cache_write_tokens: Some(1),
    }))
    .expect("usage trace fields should be valid");

    assert_eq!(fields.prompt_tokens, Some(10));
    assert_eq!(fields.prompt_cache_write_tokens, Some(1));
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
        prompt_cache_write_tokens: None,
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

    let prompt = render_prompt_text(&spec, GENERATION_PROMPT_KIND_VIDEO_PROMPT)
        .expect("video prompt should render from current schema");

    for section in VIDEO_PROMPT_REQUIRED_SECTIONS {
        assert!(prompt.contains(section), "missing section {section}");
    }
    assert!(!prompt.contains("[声音与对白]"));
    validate_prompt_artifact_text(GENERATION_PROMPT_KIND_VIDEO_PROMPT, &prompt)
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

    let err = render_prompt_text(&spec, GENERATION_PROMPT_KIND_VIDEO_PROMPT)
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
    validate_prompt_artifact_text(
        GENERATION_PROMPT_KIND_IMAGE_PROMPT,
        "清晰写实的单张角色立绘。",
    )
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
