//! 角色行为记录命令。
//!
//! Tauri 层只保留 command 签名；查询逻辑统一在
//! `a4n_runtime::actions::character_records`。

use a4n_runtime::AppState;
use a4n_runtime::actions::character_records as character_records_actions;
use character_records_actions::{BehaviorRecordSummary, ListBehaviorsInput};
use tauri::State;

#[tauri::command]
pub async fn list_behaviors(
    state: State<'_, AppState>,
    user_id: i64,
    input: ListBehaviorsInput,
) -> Result<Vec<BehaviorRecordSummary>, String> {
    character_records_actions::list_behaviors(state.inner(), user_id, input).await
}
