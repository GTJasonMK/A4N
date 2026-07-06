use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use a4n_runtime::actions::character_implicit_review as implicit_review_actions;
use tauri::State;

use implicit_review_actions::{
    ApplyImplicitUpdateDecisionInput, ApplyImplicitUpdateDecisionResultSummary,
    CheckImplicitUpdateInput, GetImplicitReviewInput, ImplicitReviewSummary,
    ImplicitUpdateDecisionSummary,
};

/// 获取隐性属性复审摘要：基于行为记录中的 classification_results 统计符合/不符合证据。
#[tauri::command]
pub async fn get_implicit_review(
    state: State<'_, AppState>,
    user_id: i64,
    input: GetImplicitReviewInput,
) -> Result<ImplicitReviewSummary, String> {
    implicit_review_actions::get_implicit_review(&state, user_id, input).await
}

/// 基于行为证据请求 LLM 判断某个隐性属性是否需要 keep/modify/delete。
#[tauri::command]
pub async fn check_implicit_update(
    state: State<'_, AppState>,
    user_id: i64,
    input: CheckImplicitUpdateInput,
) -> Result<ImplicitUpdateDecisionSummary, String> {
    implicit_review_actions::check_implicit_update(&state, user_id, input).await
}

#[tauri::command]
pub async fn preview_check_implicit_update_request(
    state: State<'_, AppState>,
    user_id: i64,
    input: CheckImplicitUpdateInput,
) -> Result<ModelRequestPreview, String> {
    implicit_review_actions::preview_check_implicit_update_request(&state, user_id, &input).await
}

/// 应用隐性属性复审结果。前端只提交决策，修改/删除保护规则集中在后端执行。
#[tauri::command]
pub async fn apply_implicit_update_decision(
    state: State<'_, AppState>,
    user_id: i64,
    input: ApplyImplicitUpdateDecisionInput,
) -> Result<ApplyImplicitUpdateDecisionResultSummary, String> {
    implicit_review_actions::apply_implicit_update_decision(&state, user_id, input).await
}
