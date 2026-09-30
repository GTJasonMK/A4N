use a4n_db_models::models::chapter::chapter;
use a4n_domain::chapter::ChapterRecord;
use serde_json::json;

use super::{
    contracts::ChapterSceneGenerationInput,
    execution::required_scene_generation_input,
    output::{normalize_scene_generation_output, restored_chapter_scene_checkpoint},
    request::{
        BuildChapterSceneRequestInput, PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT,
        build_scene_generation_manifest_user_prompt, build_scene_generation_request,
    },
};

fn chapter_for_scene_generation(title: Option<&str>, summary: Option<&str>) -> ChapterRecord {
    let now = chrono::Utc::now().naive_utc();
    ChapterRecord {
        id: 1,
        project_id: 10,
        outline_id: None,
        chapter_number: 3,
        title: title.map(str::to_string),
        content: "章节正文".to_string(),
        word_count: 4,
        status: chapter::STATUS_DRAFT.to_string(),
        summary: summary.map(str::to_string),
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }
}

fn scene_generation_input(scene_number: i32, scene_prompt: &str) -> ChapterSceneGenerationInput {
    ChapterSceneGenerationInput {
        chapter: chapter_for_scene_generation(Some("旧城"), Some("主角进入旧城")),
        scene_number,
        scene_prompt: scene_prompt.to_string(),
    }
}

#[test]
fn required_scene_generation_input_accepts_and_normalizes_valid_input() {
    let required = required_scene_generation_input(&scene_generation_input(1, " 写入城门 "))
        .expect("valid scene input");

    assert_eq!(required.title, "旧城");
    assert_eq!(required.summary, "主角进入旧城");
    assert_eq!(required.scene_prompt, "写入城门");
}

#[test]
fn required_scene_generation_input_rejects_missing_chapter_fields() {
    let mut input = scene_generation_input(1, "写入城门");
    input.chapter.title = Some("  ".to_string());
    assert!(
        required_scene_generation_input(&input)
            .expect_err("missing chapter title must fail")
            .to_string()
            .contains("已有标题")
    );

    input.chapter.title = Some("旧城".to_string());
    input.chapter.summary = None;
    assert!(
        required_scene_generation_input(&input)
            .expect_err("missing chapter summary must fail")
            .to_string()
            .contains("已有摘要")
    );
}

#[test]
fn required_scene_generation_input_rejects_invalid_scene_fields() {
    assert!(
        required_scene_generation_input(&scene_generation_input(0, "写入城门"))
            .expect_err("invalid scene number must fail")
            .to_string()
            .contains("场景编号必须大于 0")
    );
    assert!(
        required_scene_generation_input(&scene_generation_input(1, " \n\t "))
            .expect_err("blank scene prompt must fail")
            .to_string()
            .contains("场景引导")
    );
}

#[test]
fn scene_generation_request_uses_manifest_context_contract() {
    let request = build_scene_generation_request(BuildChapterSceneRequestInput {
        system_prompt: "system".to_string(),
        project_id: 10,
        chapter_number: 2,
        title: "旧城",
        summary: "主角进入旧城",
        generation_context: Some(&json!({
            "chapter_outline": {"key_scenes": ["城门盘查"]},
            "existing_scenes": [{"scene_number": 1, "content": "主角抵达城外。"}],
        })),
        scene_number: 2,
        scene_prompt: "写入城门",
    })
    .expect("scene request should build");

    assert!(request.max_tokens.is_none());
    let manifest = request
        .context_manifest_json
        .expect("context-aware request must carry manifest");
    assert_eq!(
        manifest["packs"][0]["pack_kind"],
        PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT
    );
    let manifest_text = serde_json::to_string(&manifest).expect("serialize manifest");
    assert!(manifest_text.contains("城门盘查"));
    assert!(manifest_text.contains("主角抵达城外"));
    assert!(manifest_text.contains("写入城门"));
}

#[test]
fn manifest_user_prompt_uses_context_refs_and_stays_json() {
    let prompt = build_scene_generation_manifest_user_prompt(1, "写入城门")
        .expect("scene prompt should serialize");
    assert!(prompt.contains(r#""scene_number": 1"#));
    assert!(prompt.contains(r#""prompt": "写入城门""#));
    assert!(prompt.contains(PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT));
    assert!(!prompt.contains("## 场景引导"));
}

#[test]
fn normalize_scene_generation_output_rejects_invalid_shapes() {
    assert!(normalize_scene_generation_output(" \n\t ").is_err());
    assert!(
        normalize_scene_generation_output(r#"{"title":"城门","content":"正文"}"#)
            .expect_err("json must fail")
            .to_string()
            .contains("必须是纯文本正文")
    );
    assert!(
        normalize_scene_generation_output("```text\n正文\n```")
            .expect_err("fenced markdown must fail")
            .to_string()
            .contains("Markdown")
    );
}

#[test]
fn restored_chapter_scene_checkpoint_revalidates_response_content() {
    let restored = restored_chapter_scene_checkpoint(
        &json!("主角抵达城门，雨水沿着旧铜钉滑落。"),
        "  主角抵达城门，雨水沿着旧铜钉滑落。  ",
        "chapter_scene_generation:3:1",
    )
    .expect("valid checkpoint should restore");

    assert_eq!(restored, "主角抵达城门，雨水沿着旧铜钉滑落。");
}

#[test]
fn restored_chapter_scene_checkpoint_rejects_mismatched_parsed_output() {
    let error = restored_chapter_scene_checkpoint(
        &json!("主角抵达城门。"),
        "主角已经离开城门。",
        "chapter_scene_generation:3:1",
    )
    .expect_err("checkpoint parsed output mismatch must fail");

    assert!(
        error
            .to_string()
            .contains("checkpoint_parsed_output_mismatch")
    );
}
