mod builder;
mod context;
mod user_message;

pub use builder::build_blueprint_refine_request;

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
    async fn manifest_user_message_uses_context_refs_and_omits_missing_focus() {
        let message =
            user_message::blueprint_refine_manifest_user_message(&prompt_engine(), None, None)
                .await
                .expect("message should render");

        assert!(message.contains(r#""context_refs""#));
        assert!(message.contains("existing_blueprint_context"));
        assert!(!message.contains("星潮档案"));
        assert!(!message.contains(r#""focus""#));
        assert!(!message.contains("全面优化"));
        assert!(!message.contains(r#""title""#));
        assert!(message.contains(r#""style""#));
        assert!(message.contains(r#""field_schemas""#));
        assert!(message.contains(r#""relationship_type""#));
        for relationship_type in a4n_core::blueprint::BLUEPRINT_RELATIONSHIP_TYPES {
            assert!(message.contains(&format!(r#""{relationship_type}""#)));
        }
    }

    #[tokio::test]
    async fn manifest_user_message_omits_blank_focus() {
        let message = user_message::blueprint_refine_manifest_user_message(
            &prompt_engine(),
            None,
            Some(" \n\t "),
        )
        .await
        .expect("message should render");

        assert!(!message.contains(r#""focus""#));
        assert!(!message.contains("全面优化"));
    }

    #[tokio::test]
    async fn manifest_user_message_includes_explicit_focus() {
        let message = user_message::blueprint_refine_manifest_user_message(
            &prompt_engine(),
            None,
            Some(" 强化主角动机 "),
        )
        .await
        .expect("message should render");

        assert!(message.contains(r#""focus": "强化主角动机""#));
    }

    #[tokio::test]
    async fn request_moves_existing_blueprint_into_manifest() {
        let engine = prompt_engine();
        let blueprint_context = json!({
            "blueprint": {
                "title": "星潮档案",
                "genre": "科幻",
                "full_synopsis": "完整主线",
            },
            "characters": [{"name": "林烬", "motivation": "查明真相"}],
            "relationships": [],
        });
        let request = build_blueprint_refine_request(
            &engine,
            42,
            Some("强化主角动机"),
            &std::collections::HashMap::new(),
            &blueprint_context,
        )
        .await
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
        assert_eq!(
            trace_context.workflow,
            a4n_core::blueprint::BLUEPRINT_REFINEMENT_WORKFLOW_KIND
        );
        assert_eq!(
            trace_context.call_key.as_deref(),
            Some("blueprint_refine:project:42")
        );
    }
}
