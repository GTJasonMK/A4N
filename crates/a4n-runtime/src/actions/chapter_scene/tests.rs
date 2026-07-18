use super::access_errors::{
    required_scene_generation_inputs, required_updated_text, trimmed_optional_text,
};
use super::contracts::GenerateChapterSceneInput;
use super::output::normalize_scene_generation_output;
use super::request::build_scene_generation_user_prompt;
use super::workflow::restored_chapter_scene_checkpoint;
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use serde_json::json;

fn chapter_for_scene_generation(title: Option<&str>, summary: Option<&str>) -> chapter::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter::Model {
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

fn scene_generation_input(scene_number: i32, scene_prompt: &str) -> GenerateChapterSceneInput {
    GenerateChapterSceneInput {
        chapter_id: 1,
        scene_number,
        scene_prompt: scene_prompt.to_string(),
    }
}

#[test]
fn required_scene_generation_inputs_accepts_valid_input() {
    let chapter = chapter_for_scene_generation(Some("旧城"), Some("主角进入旧城"));
    let input = scene_generation_input(1, " 写入城门 ");

    let (title, summary, prompt) =
        required_scene_generation_inputs(&chapter, &input).expect("valid scene input");

    assert_eq!(title, "旧城");
    assert_eq!(summary, "主角进入旧城");
    assert_eq!(prompt, "写入城门");
}

#[test]
fn required_scene_generation_inputs_rejects_missing_chapter_title() {
    let chapter = chapter_for_scene_generation(Some("  "), Some("主角进入旧城"));
    let input = scene_generation_input(1, "写入城门");

    let err = required_scene_generation_inputs(&chapter, &input)
        .expect_err("missing chapter title must fail");

    assert!(err.to_string().contains("已有标题"));
}

#[test]
fn required_scene_generation_inputs_rejects_missing_chapter_summary() {
    let chapter = chapter_for_scene_generation(Some("旧城"), None);
    let input = scene_generation_input(1, "写入城门");

    let err = required_scene_generation_inputs(&chapter, &input)
        .expect_err("missing chapter summary must fail");

    assert!(err.to_string().contains("已有摘要"));
}

#[test]
fn required_scene_generation_inputs_rejects_invalid_scene_number() {
    let chapter = chapter_for_scene_generation(Some("旧城"), Some("主角进入旧城"));
    let input = scene_generation_input(0, "写入城门");

    let err = required_scene_generation_inputs(&chapter, &input)
        .expect_err("invalid scene number must fail");

    assert!(err.to_string().contains("场景编号必须大于 0"));
}

#[test]
fn required_scene_generation_inputs_rejects_blank_scene_prompt() {
    let chapter = chapter_for_scene_generation(Some("旧城"), Some("主角进入旧城"));
    let input = scene_generation_input(1, " \n\t ");

    let err = required_scene_generation_inputs(&chapter, &input)
        .expect_err("blank scene prompt must fail");

    assert!(err.to_string().contains("场景引导"));
}

#[test]
fn trimmed_optional_text_normalizes_blank_to_none() {
    assert_eq!(trimmed_optional_text(" \n\t "), None);
    assert_eq!(trimmed_optional_text(" 旧城 "), Some("旧城".to_string()));
}

#[test]
fn required_updated_text_rejects_blank_value() {
    let err = required_updated_text(" \n\t ", "场景正文")
        .expect_err("blank updated scene content must fail");

    assert!(matches!(err, AppError::InvalidArgument(message) if message == "场景正文不能为空"));
    assert_eq!(
        required_updated_text("  场景正文  ", "场景正文").expect("nonblank text"),
        "场景正文"
    );
}

#[test]
fn build_scene_generation_user_prompt_uses_json_payload() {
    let prompt = build_scene_generation_user_prompt(
        2,
        "旧城",
        "主角进入旧城",
        Some(&json!({
            "chapter_outline": {
                "key_scenes": ["城门盘查"]
            },
            "existing_scenes": [{
                "scene_number": 0,
                "content": "主角抵达城外。"
            }]
        })),
        1,
        "写入城门",
    )
    .expect("scene prompt should serialize");

    assert!(prompt.contains(r#""chapter_number": 2"#));
    assert!(prompt.contains(r#""title": "旧城""#));
    assert!(prompt.contains(r#""key_scenes""#));
    assert!(prompt.contains("主角抵达城外"));
    assert!(prompt.contains(r#""scene_number": 1"#));
    assert!(prompt.contains(r#""prompt": "写入城门""#));
    assert!(!prompt.contains("## 场景引导"));
}

#[test]
fn build_scene_generation_user_prompt_rejects_blank_prompt() {
    let err = build_scene_generation_user_prompt(2, "旧城", "摘要", None, 1, " \n\t ")
        .expect_err("blank scene prompt must fail");

    assert!(err.to_string().contains("场景引导"));
}

#[test]
fn normalize_scene_generation_output_rejects_blank() {
    let err =
        normalize_scene_generation_output(" \n\t ").expect_err("blank generated scene must fail");

    assert!(err.to_string().contains("未返回"));
}

#[test]
fn normalize_scene_generation_output_rejects_json() {
    let err = normalize_scene_generation_output(r#"{"title":"城门","content":"主角抵达城门。"}"#)
        .expect_err("scene generation must be plain text");

    assert!(err.to_string().contains("必须是纯文本正文"));
}

#[test]
fn normalize_scene_generation_output_rejects_fenced_markdown() {
    let err = normalize_scene_generation_output("```text\n主角抵达城门。\n```")
        .expect_err("scene generation must not be fenced");

    assert!(err.to_string().contains("Markdown"));
}

#[test]
fn restored_chapter_scene_checkpoint_revalidates_response_content() {
    let restored = restored_chapter_scene_checkpoint(
        json!({
            "output": "主角抵达城门，雨水沿着旧铜钉滑落。",
            "parsed_output": "主角抵达城门，雨水沿着旧铜钉滑落。",
            "response_content": "  主角抵达城门，雨水沿着旧铜钉滑落。  "}),
        "chapter_scene_generation:3:1",
    )
    .expect("valid scene checkpoint should restore");

    assert_eq!(restored, "主角抵达城门，雨水沿着旧铜钉滑落。");
}

#[test]
fn restored_chapter_scene_checkpoint_rejects_mismatched_output() {
    let err = restored_chapter_scene_checkpoint(
        json!({
            "output": "主角抵达城门。",
            "parsed_output": "主角抵达城门。",
            "response_content": "主角已经离开城门。"}),
        "chapter_scene_generation:3:1",
    )
    .expect_err("checkpoint output mismatch must fail");

    assert!(
        err.to_string()
            .contains("checkpoint_parsed_output_mismatch")
    );
}
