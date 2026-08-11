use std::collections::HashMap;
use std::path::Path;

use a4n_core::models::chapter::part_outline;
use a4n_llm::provider::MessageRole;
use a4n_prompt::tera_engine::TeraPromptEngine;
use chrono::Utc;
use serde_json::json;

use crate::outline::context::ChapterOutlineGenerationContext;

use super::contracts::ChapterOutlineRequestInput;
use super::requests::build_chapter_outline_request;

fn prompts_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts")
}

fn prompt_renderer() -> TeraPromptEngine {
    TeraPromptEngine::from_directory(&prompts_dir()).expect("prompts should load")
}

fn sample_part() -> part_outline::Model {
    let now = Utc::now().naive_utc();
    part_outline::Model {
        id: 8,
        project_id: 42,
        part_number: 1,
        title: "星火初燃".to_string(),
        start_chapter: 1,
        end_chapter: 20,
        summary: Some("分部完整摘要不应进入章节动态 user prompt".to_string()),
        theme: Some("代价".to_string()),
        key_events: Some(json!(["分部关键事件"])),
        character_focus: Some(json!({"林烬": "承担责任"})),
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test]
async fn chapter_outline_request_moves_blueprint_and_parts_into_manifest() {
    let engine = prompt_renderer();
    let part = sample_part();
    let context = ChapterOutlineGenerationContext {
        blueprint_payload: json!({
            "title": "星潮档案",
            "genre": "科幻",
            "one_sentence_summary": "调查星潮真相",
        }),
        parts: vec![part.clone()],
        source_snapshot: json!({}),
    };
    let variables = HashMap::new();
    let request = build_chapter_outline_request(ChapterOutlineRequestInput {
        project_id: 42,
        context: &context,
        focus_part: Some(&part),
        start_chapter: 3,
        count: 1,
        end_chapter: 3,
        prompt_renderer: &engine,
        variables: &variables,
    })
    .await
    .expect("chapter outline request should build");

    let manifest = request
        .context_manifest_json
        .as_ref()
        .expect("context manifest should be attached");
    let pack_kinds = manifest["packs"]
        .as_array()
        .expect("packs should be array")
        .iter()
        .filter_map(|pack| pack["pack_kind"].as_str())
        .collect::<Vec<_>>();
    assert!(pack_kinds.contains(&"outline_blueprint_context"));
    assert!(pack_kinds.contains(&"outline_part_outlines"));

    let user_message = request
        .messages
        .iter()
        .find(|message| matches!(message.role, MessageRole::User))
        .expect("user message should exist");
    assert!(user_message.content.contains("context_refs"));
    assert!(user_message.content.contains("outline_blueprint_context"));
    assert!(user_message.content.contains("outline_part_outlines"));
    assert!(!user_message.content.contains("星潮档案"));
    assert!(
        !user_message
            .content
            .contains("分部完整摘要不应进入章节动态 user prompt")
    );

    let trace_context = request
        .trace_context
        .as_ref()
        .expect("trace context should be attached");
    assert_eq!(trace_context.workflow, "chapter_outline_generation");
    assert_eq!(trace_context.step_key, "chapter_3");
    assert_eq!(trace_context.call_key.as_deref(), Some("chapter_3"));
}
