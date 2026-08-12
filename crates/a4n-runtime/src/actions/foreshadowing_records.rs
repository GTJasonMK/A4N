pub use a4n_api_contract::profile::{
    CreateForeshadowingInput, DeleteForeshadowingInput, ForeshadowingSummary,
    ForeshadowingWriteResult, ListForeshadowingInput, UpdateForeshadowingInput,
};
use a4n_core::models::character::foreshadowing_index;
use a4n_domain::character::ForeshadowingStatus;

use crate::{AppState, actions::access};

pub async fn list_foreshadowing(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ForeshadowingSummary>, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let items = state
        .orchestrator()
        .foreshadowing_records_by_project(project_id)
        .await
        .map_err(|error| error.to_string())?;
    items.iter().map(to_summary).collect()
}

pub async fn create_foreshadowing(
    state: &AppState,
    user_id: i64,
    input: CreateForeshadowingInput,
) -> Result<ForeshadowingWriteResult, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let record = state
        .orchestrator()
        .create_foreshadowing_record(input.project_id, input.record)
        .await
        .map_err(|error| error.to_string())?;
    Ok(ForeshadowingWriteResult {
        warnings: sync_rag_warning(state, &record).await,
    })
}

pub async fn update_foreshadowing(
    state: &AppState,
    user_id: i64,
    input: UpdateForeshadowingInput,
) -> Result<ForeshadowingWriteResult, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let record = state
        .orchestrator()
        .update_foreshadowing_record(
            input.project_id,
            input.foreshadowing_id,
            &input.expected_updated_at,
            input.record,
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(ForeshadowingWriteResult {
        warnings: sync_rag_warning(state, &record).await,
    })
}

pub async fn delete_foreshadowing(
    state: &AppState,
    user_id: i64,
    input: DeleteForeshadowingInput,
) -> Result<ForeshadowingWriteResult, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let deleted = state
        .orchestrator()
        .delete_foreshadowing_record(
            input.project_id,
            input.foreshadowing_id,
            &input.expected_updated_at,
        )
        .await
        .map_err(|error| error.to_string())?;
    let warnings = match state
        .orchestrator()
        .delete_foreshadowing_rag_by_ids(input.project_id, &[deleted.id])
        .await
    {
        Ok(_) => Vec::new(),
        Err(error) => {
            tracing::warn!(
                project_id = input.project_id,
                foreshadowing_id = deleted.id,
                error = %error,
                "伏笔已删除，但 RAG 清理失败"
            );
            vec!["伏笔已删除，但检索索引清理失败，可稍后重试".to_string()]
        }
    };
    Ok(ForeshadowingWriteResult { warnings })
}

fn to_summary(record: &foreshadowing_index::Model) -> Result<ForeshadowingSummary, String> {
    Ok(ForeshadowingSummary {
        id: record.id,
        project_id: record.project_id,
        description: record.description.clone(),
        setup_chapter: record.setup_chapter,
        payoff_chapter: record.payoff_chapter,
        status: ForeshadowingStatus::parse(&record.status)
            .ok_or_else(|| format!("伏笔 #{} 状态非法：{}", record.id, record.status))?,
        related_characters: record
            .related_characters
            .as_ref()
            .map_or_else(
                || Ok(Vec::new()),
                |value| serde_json::from_value::<Vec<String>>(value.clone()),
            )
            .map_err(|error| format!("伏笔 #{} 关联角色格式非法：{error}", record.id))?,
        importance: record.importance,
        created_at: record.created_at.to_string(),
        updated_at: record.updated_at.to_string(),
    })
}

async fn sync_rag_warning(state: &AppState, record: &foreshadowing_index::Model) -> Vec<String> {
    match state
        .orchestrator()
        .sync_foreshadowing_rag(record.project_id, record)
        .await
    {
        Ok(()) => Vec::new(),
        Err(error) => {
            tracing::warn!(
                project_id = record.project_id,
                foreshadowing_id = record.id,
                error = %error,
                "伏笔已保存，但 RAG 索引失败"
            );
            vec!["伏笔已保存，但检索索引更新失败，可稍后重试".to_string()]
        }
    }
}
