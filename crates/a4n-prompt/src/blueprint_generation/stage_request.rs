use a4n_domain::source_text_hash;
use a4n_error::AppError;
use serde_json::Value;

use crate::rendering::PromptRenderer;

const BLUEPRINT_CONTEXT_LEDGER_VERSION: &str = "blueprint_context_ledger_v1";

mod builder;
mod context;
mod user_message;

pub use builder::build_blueprint_stage_request;
use context::build_blueprint_stage_context_manifest;
use user_message::build_blueprint_stage_manifest_user_message;

pub struct BlueprintStageRequestInput<'a> {
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub project_id: i64,
    pub base_prompt: &'a str,
    pub user_payload_json: &'a str,
    pub stage_key: &'a str,
    pub stage_name: &'a str,
    pub output_schema: &'a str,
    pub previous_context: Option<&'a Value>,
}

fn blueprint_generation_source_hash(value: &Value) -> Result<String, AppError> {
    Ok(source_text_hash(&serde_json::to_string(value)?))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use a4n_llm::provider::MessageRole;
    use serde_json::json;

    use crate::tera_engine::TeraPromptEngine;

    use super::*;

    fn prompt_engine() -> TeraPromptEngine {
        let prompts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
        TeraPromptEngine::from_directory(&prompts_dir).expect("prompts should load")
    }

    #[tokio::test]
    async fn moves_source_context_into_manifest() {
        let previous_context = json!({"story_core":{"title":"星潮档案"}});
        let request = build_blueprint_stage_request(BlueprintStageRequestInput {
            prompt_renderer: &prompt_engine(),
            project_id: 42,
            base_prompt: "基础蓝图规则",
            user_payload_json:
                r#"{"novel_title":"星潮档案","genre":"科幻","requested_chapter_count":80}"#,
            stage_key: "world",
            stage_name: "阶段 2/4：世界观",
            output_schema: r#"{"world_setting":{"premise":"string"}}"#,
            previous_context: Some(&previous_context),
        })
        .await
        .expect("stage request should build");

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
        assert!(pack_kinds.contains(&"blueprint_project_seed_context"));
        assert!(pack_kinds.contains(&"blueprint_previous_stage_outputs"));

        let user_message = request
            .messages
            .iter()
            .find(|message| matches!(message.role, MessageRole::User))
            .expect("user message should exist");
        assert!(user_message.content.contains("context_refs"));
        assert!(
            user_message
                .content
                .contains("blueprint_project_seed_context")
        );
        assert!(user_message.content.contains("每个 factions[] 必须包含"));
        assert!(user_message.content.contains("representative_characters"));
        assert!(!user_message.content.contains("星潮档案"));
        assert!(!user_message.content.contains(r#""story_core""#));
    }

    #[tokio::test]
    async fn rejects_invalid_output_schema() {
        let error = build_blueprint_stage_request(BlueprintStageRequestInput {
            prompt_renderer: &prompt_engine(),
            project_id: 42,
            base_prompt: "基础蓝图规则",
            user_payload_json: r#"{"novel_title":"星潮档案"}"#,
            stage_key: "story_core",
            stage_name: "阶段测试",
            output_schema: "not-json",
            previous_context: None,
        })
        .await
        .expect_err("stage schema must be JSON");

        assert!(error.to_string().contains("输出 schema 不是合法 JSON"));
    }

    #[tokio::test]
    async fn rejects_non_object_output_schema() {
        let error = build_blueprint_stage_request(BlueprintStageRequestInput {
            prompt_renderer: &prompt_engine(),
            project_id: 42,
            base_prompt: "基础蓝图规则",
            user_payload_json: r#"{"novel_title":"星潮档案"}"#,
            stage_key: "story_core",
            stage_name: "阶段测试",
            output_schema: "[]",
            previous_context: None,
        })
        .await
        .expect_err("stage schema must be an object");

        assert!(error.to_string().contains("输出 schema 必须是 JSON 对象"));
    }

    #[test]
    fn source_hash_uses_serialized_json_text() {
        let value = json!({"title": "测试 & 蓝图", "chapters": [1, 2]});
        let serialized = serde_json::to_string(&value).unwrap();

        assert_eq!(
            blueprint_generation_source_hash(&value).unwrap(),
            source_text_hash(&serialized)
        );
    }
}
