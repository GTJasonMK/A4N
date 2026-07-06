//! 人物三视图拆解命令
//!
//! 对应服务：PortraitService
//! 项目级人物参考图查询、主图设置、提示词生成与图像生成。

pub(crate) mod generation;

use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use a4n_runtime::actions::portrait_generation::{
    GeneratePortraitPromptInput, PortraitSummary, UploadPortraitInput,
};
use a4n_runtime::actions::portrait_records as portrait_records_actions;
use tauri::State;

/// 按项目列出所有人物参考图
#[tauri::command]
pub async fn list_portraits_by_project(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<PortraitSummary>, String> {
    portrait_records_actions::list_portraits_by_project(state.inner(), user_id, project_id).await
}

/// 获取角色的主参考图
#[tauri::command]
pub async fn get_primary_portrait(
    state: State<'_, AppState>,
    user_id: i64,
    profile_id: i64,
) -> Result<Option<PortraitSummary>, String> {
    portrait_records_actions::get_primary_portrait(state.inner(), user_id, profile_id).await
}

/// 设置为主参考图
#[tauri::command]
pub async fn set_primary_portrait(
    state: State<'_, AppState>,
    user_id: i64,
    portrait_id: i64,
) -> Result<(), String> {
    portrait_records_actions::set_primary_portrait(state.inner(), user_id, portrait_id).await
}

/// 上传本地图片作为角色立绘，并同步写入图片资产镜像。
#[tauri::command]
pub async fn upload_portrait(
    state: State<'_, AppState>,
    user_id: i64,
    input: UploadPortraitInput,
) -> Result<PortraitSummary, String> {
    a4n_runtime::actions::portrait_generation::upload_portrait(state.inner(), user_id, input).await
}

/// 生成人物三视图拆解提示词（LLM 生成）
///
/// 需要 AppState 包含 engine 和 llm 字段。
/// 返回可直接用于图片生成的提示词文本。
#[tauri::command]
pub async fn generate_portrait_prompt(
    state: State<'_, AppState>,
    user_id: i64,
    input: GeneratePortraitPromptInput,
) -> Result<String, String> {
    a4n_runtime::actions::portrait_generation::generate_portrait_prompt(
        state.inner(),
        user_id,
        input,
    )
    .await
}

#[tauri::command]
pub async fn preview_generate_portrait_prompt_request(
    state: State<'_, AppState>,
    user_id: i64,
    input: GeneratePortraitPromptInput,
) -> Result<ModelRequestPreview, String> {
    a4n_runtime::actions::portrait_generation::preview_generate_portrait_prompt_request(
        state.inner(),
        user_id,
        input,
    )
    .await
}
