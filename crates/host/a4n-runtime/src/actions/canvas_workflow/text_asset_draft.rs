use a4n_api_contract::canvas::{CanvasTextAssetDraftInput, CanvasTextAssetDraftSummary};
use a4n_api_contract::request_preview::ModelRequestPreview;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;

use crate::{
    AppState,
    actions::request_preview::{
        ChatRequestStepInput, ChatRequestStepWithBudgetInput, chat_request_step_with_budget,
        model_request_preview,
    },
    dynamic_llm,
};

const SYSTEM_PROMPT: &str =
    "你是 A4N 的媒体文本创作助手。遵守用户指令，参考提供的素材，只返回可直接编辑和使用的正文。";

pub async fn preview_canvas_text_asset_draft_request(
    state: &AppState,
    user_id: i64,
    input: CanvasTextAssetDraftInput,
) -> Result<ModelRequestPreview, AppError> {
    let request = build_request(state, user_id, &input).await?;
    let model = state
        .provider_runtime()
        .resolve_llm_config(input.llm_config_id)
        .await
        .map_err(|error| AppError::LlmError(error.to_string()))?;
    let budget = dynamic_llm::resolve_effective_llm_preview_budget_for_config(model, &request)
        .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
    let step = chat_request_step_with_budget(ChatRequestStepWithBudgetInput {
        step: ChatRequestStepInput {
            key: "text_asset_draft".to_string(),
            title: "项目文本素材草稿请求".to_string(),
            prompt_name: None,
            state: "ready".to_string(),
            note: None,
            request,
        },
        model_context_tokens: budget.max_context_tokens,
        output_token_reserve: budget.output_token_reserve,
        budget_note: Some(format!("预览模型={}；上下文={}", budget.model_name, budget.max_context_tokens)),
    })
    .map_err(AppError::InvalidArgument)?;
    Ok(model_request_preview(
        "canvas_text_asset_draft",
        "项目文本素材 AI 草稿请求预览",
        vec![step],
    ))
}

pub async fn generate_canvas_text_asset_draft(
    state: &AppState,
    user_id: i64,
    input: CanvasTextAssetDraftInput,
) -> Result<CanvasTextAssetDraftSummary, AppError> {
    let request = build_request(state, user_id, &input).await?;
    let response = state
        .dynamic_llm_provider_for_profile()
        .map_err(|error| AppError::LlmError(error.to_string()))?
        .chat_with_config_id(input.llm_config_id, request)
        .await?;
    let content = response.message.content.trim().to_string();
    if content.is_empty() {
        return Err(AppError::LlmError(
            "文本素材草稿模型没有返回正文".to_string(),
        ));
    }
    Ok(CanvasTextAssetDraftSummary { content })
}

async fn build_request(
    state: &AppState,
    user_id: i64,
    input: &CanvasTextAssetDraftInput,
) -> Result<ChatRequest, AppError> {
    let instruction = input.instruction.trim();
    if instruction.is_empty() {
        return Err(AppError::InvalidArgument("AI 草稿指令不能为空".to_string()));
    }
    let source = match (input.source_asset_id, input.source_version) {
        (Some(asset_id), Some(version)) => {
            let versions = state
                .canvas_read_application()
                .list_text_asset_versions(
                    user_id,
                    a4n_api_contract::canvas::ListCanvasTextAssetVersionsInput {
                        project_id: input.project_id,
                        asset_id,
                    },
                )
                .await?;
            let version = versions
                .into_iter()
                .find(|candidate| candidate.version == version)
                .ok_or_else(|| AppError::NotFound(format!("文本素材版本不存在: {asset_id}/{version}")))?;
            Some(version.content)
        }
        (None, None) => None,
        _ => {
            return Err(AppError::InvalidArgument(
                "素材来源必须同时指定素材 ID 和版本".to_string(),
            ));
        }
    };
    let source_message = source.map_or_else(String::new, |text| format!("\n\n参考素材：\n{text}"));
    let user_prompt = format!("指令：\n{instruction}{source_message}\n\n只返回正文。",);
    Ok(ChatRequest::system_user(SYSTEM_PROMPT, user_prompt, None, None))
}
