use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::context::context_aware_system_user_request;
use a4n_llm::provider::{ChatRequest, LlmTraceContext};
use serde_json::{Value, json};

mod builder;
mod context;
mod user_message;

pub use builder::build_blueprint_refine_request;
use context::build_blueprint_refine_context_manifest;
pub use user_message::blueprint_refine_manifest_user_message;

#[cfg(test)]
mod tests {
    use std::path::Path;

    use a4n_llm::provider::MessageRole;
    use serde_json::json;

    use crate::engine::PromptEngine;
    use crate::tera_engine::TeraPromptEngine;

    use super::*;

    fn prompt_engine() -> TeraPromptEngine {
        let prompts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
        TeraPromptEngine::from_directory(&prompts_dir).expect("prompts should load")
    }

    #[tokio::test]
    async fn manifest_user_message_uses_context_refs_and_omits_missing_focus() {
        let message = blueprint_refine_manifest_user_message(&prompt_engine(), None, None)
            .await
            .expect("message should render");

        assert!(message.contains(r#""context_refs""#));
        assert!(message.contains("existing_blueprint_context"));
        assert!(!message.contains("星潮档案"));
        assert!(!message.contains(r#""focus""#));
        assert!(!message.contains("全面优化"));
    }

    #[tokio::test]
    async fn manifest_user_message_omits_blank_focus() {
        let message =
            blueprint_refine_manifest_user_message(&prompt_engine(), None, Some(" \n\t "))
                .await
                .expect("message should render");

        assert!(!message.contains(r#""focus""#));
        assert!(!message.contains("全面优化"));
    }

    #[tokio::test]
    async fn manifest_user_message_includes_explicit_focus() {
        let message =
            blueprint_refine_manifest_user_message(&prompt_engine(), None, Some(" 强化主角动机 "))
                .await
                .expect("message should render");

        assert!(message.contains(r#""focus": "强化主角动机""#));
    }

    #[tokio::test]
    async fn request_moves_existing_blueprint_into_manifest() {
        let engine = prompt_engine();
        let system_prompt = engine
            .render(
                "blueprint_optimization",
                &std::collections::HashMap::new(),
                Some(42),
            )
            .expect("system prompt should render");
        let user_message =
            blueprint_refine_manifest_user_message(&engine, Some(42), Some("强化主角动机"))
                .await
                .expect("user message should render");
        let blueprint_context = json!({
            "blueprint": {
                "title": "星潮档案",
                "genre": "科幻",
                "full_synopsis": "完整主线",
            },
            "characters": [{"name": "林烬", "motivation": "查明真相"}],
            "relationships": [],
        });
        let request =
            build_blueprint_refine_request(42, system_prompt, user_message, &blueprint_context)
                .expect("refine request should build");

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
        assert!(pack_kinds.contains(&"existing_blueprint_context"));

        let user_message = request
            .messages
            .iter()
            .find(|message| matches!(message.role, MessageRole::User))
            .expect("user message should exist");
        assert!(user_message.content.contains("context_refs"));
        assert!(user_message.content.contains("existing_blueprint_context"));
        assert!(!user_message.content.contains("星潮档案"));
        assert!(!user_message.content.contains("完整主线"));

        let trace_context = request
            .trace_context
            .as_ref()
            .expect("trace context should be attached");
        assert_eq!(trace_context.workflow, "blueprint_refine");
        assert_eq!(trace_context.call_key.as_deref(), Some("blueprint_refine"));
    }
}
