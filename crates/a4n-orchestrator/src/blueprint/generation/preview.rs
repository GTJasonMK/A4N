use std::collections::HashMap;

use super::contracts::BLUEPRINT_GENERATION_STAGES;
use a4n_core::blueprint::build_blueprint_generation_payload;
use a4n_core::error::AppError;
use a4n_prompt::blueprint_generation::{
    BlueprintGenerationPreviewStep, BlueprintStageRequestInput, build_blueprint_stage_request,
};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

const BLUEPRINT_STAGE_PREVIEW_PROMPT_NAME: &str =
    "blueprint + blueprint_stage_system + blueprint_stage_user";

pub(in crate::blueprint) async fn preview_generate_requests(
    project_id: i64,
    novel_title: &str,
    genre: &str,
    target_length: i32,
    prompt_renderer: &dyn PromptRenderer,
    variables: &HashMap<String, Value>,
) -> Result<Vec<BlueprintGenerationPreviewStep>, AppError> {
    let prompt_content = prompt_renderer
        .render_prompt("blueprint", variables, Some(project_id))
        .await?;
    let user_payload =
        build_blueprint_generation_payload(novel_title, genre, target_length, variables)?;
    let user_payload_json = serde_json::to_string_pretty(&user_payload)?;

    let mut steps = Vec::new();
    let mut next_missing_seen = false;
    for spec in &BLUEPRINT_GENERATION_STAGES {
        if next_missing_seen {
            steps.push(blocked_blueprint_preview_step(
                spec.key,
                spec.title,
                spec.blocked_note,
            ));
            continue;
        }
        let request = build_blueprint_stage_request(BlueprintStageRequestInput {
            prompt_renderer,
            project_id,
            base_prompt: &prompt_content,
            user_payload_json: &user_payload_json,
            stage_key: spec.key,
            stage_name: spec.title,
            output_schema: spec.output_schema,
            previous_context: None,
        })
        .await?;
        steps.push(ready_blueprint_preview_step(spec.key, spec.title, request));
        next_missing_seen = true;
    }
    Ok(steps)
}

fn ready_blueprint_preview_step(
    key: &str,
    title: &str,
    request: a4n_llm::provider::ChatRequest,
) -> BlueprintGenerationPreviewStep {
    BlueprintGenerationPreviewStep {
        key: key.to_string(),
        title: title.to_string(),
        prompt_name: BLUEPRINT_STAGE_PREVIEW_PROMPT_NAME.to_string(),
        state: "ready".to_string(),
        note: None,
        request: Some(request),
    }
}

fn blocked_blueprint_preview_step(
    key: &str,
    title: &str,
    note: &str,
) -> BlueprintGenerationPreviewStep {
    BlueprintGenerationPreviewStep {
        key: key.to_string(),
        title: title.to_string(),
        prompt_name: BLUEPRINT_STAGE_PREVIEW_PROMPT_NAME.to_string(),
        state: "blocked".to_string(),
        note: Some(note.to_string()),
        request: None,
    }
}

#[cfg(test)]
mod tests {
    use a4n_llm::provider::{ChatMessage, ChatRequest};

    use super::*;

    #[test]
    fn projects_ready_and_blocked_preview_states() {
        let ready = ready_blueprint_preview_step(
            "story_core",
            "阶段 1/4：故事核心",
            ChatRequest::new(
                vec![ChatMessage::user("生成故事核心")],
                None,
                None,
                Some("blueprint".to_string()),
            ),
        );
        assert_eq!(ready.state, "ready");
        assert!(ready.note.is_none());
        assert!(ready.request.is_some());
        assert_eq!(ready.prompt_name, BLUEPRINT_STAGE_PREVIEW_PROMPT_NAME);

        let blocked = blocked_blueprint_preview_step(
            "world",
            "阶段 2/4：世界观",
            "需要先获得故事核心阶段输出。",
        );
        assert_eq!(blocked.state, "blocked");
        assert_eq!(
            blocked.note.as_deref(),
            Some("需要先获得故事核心阶段输出。")
        );
        assert!(blocked.request.is_none());
    }
}
