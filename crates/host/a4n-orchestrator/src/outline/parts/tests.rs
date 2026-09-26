use std::collections::HashMap;
use std::path::Path;

use a4n_db_models::models::chapter::part_outline;
use a4n_llm_provider_contract::provider::MessageRole;
use a4n_prompt::tera_engine::TeraPromptEngine;
use chrono::Utc;
use serde_json::json;

use super::contracts::PartOutlineRequestInput;
use super::requests::build_part_outline_request;

fn prompts_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../prompts")
}

fn prompt_renderer() -> TeraPromptEngine {
    TeraPromptEngine::from_directory(&prompts_dir()).expect("prompts should load")
}

fn sample_previous_part() -> part_outline::Model {
    let now = Utc::now().naive_utc();
    part_outline::Model {
        id: 7,
        project_id: 42,
        part_number: 1,
        title: "星火初燃".to_string(),
        start_chapter: 1,
        end_chapter: 20,
        summary: Some("旧分部完整摘要不应进入动态 user prompt".to_string()),
        theme: Some("代价".to_string()),
        key_events: Some(json!(["旧分部关键事件"])),
        character_focus: Some(json!({"林烬": "承担责任"})),
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test]
async fn part_outline_request_moves_blueprint_and_previous_parts_into_manifest() {
    let engine = prompt_renderer();
    let previous_part = sample_previous_part();
    let request = build_part_outline_request(PartOutlineRequestInput {
        project_id: 42,
        part_num: 2,
        start_ch: 21,
        end_ch: 40,
        chapters_per_part: 20,
        requested_part_count: 1,
        blueprint_context: &json!({
            "blueprint_available": true,
            "title": "星潮档案",
            "genre": "科幻",
            "one_sentence_summary": "调查星潮真相",
            "characters": [{"name": "林烬", "motivation": "查明真相"}],
        }),
        previous_parts: std::slice::from_ref(&previous_part),
        source_snapshot: &json!({"chapters": []}),
        prompt_renderer: &engine,
        variables: &HashMap::new(),
    })
    .await
    .expect("part outline request should build");

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
    assert!(pack_kinds.contains(&"outline_previous_part_outlines"));

    let user_message = request
        .messages
        .iter()
        .find(|message| matches!(message.role, MessageRole::User))
        .expect("user message should exist");
    assert!(user_message.content.contains("context_refs"));
    assert!(user_message.content.contains("outline_blueprint_context"));
    assert!(
        user_message
            .content
            .contains("outline_previous_part_outlines")
    );
    assert!(!user_message.content.contains("星潮档案"));
    assert!(
        !user_message
            .content
            .contains("旧分部完整摘要不应进入动态 user prompt")
    );

    let trace_context = request
        .trace_context
        .as_ref()
        .expect("trace context should be attached");
    assert_eq!(trace_context.workflow, "part_outline_generation");
    assert_eq!(trace_context.step_key, "part_2");
    assert_eq!(trace_context.call_key.as_deref(), Some("part_2"));
}
