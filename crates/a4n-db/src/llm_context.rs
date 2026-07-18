use a4n_core::models::generation::llm_context_pack;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, Set,
};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct RecordOrReuseContextPackInput {
    pub project_id: i64,
    pub root_project_id: i64,
    pub scope_kind: String,
    pub workflow: String,
    pub pack_kind: String,
    pub source_refs_json: Option<Value>,
    pub config_hash: String,
    pub content_sha256: String,
    pub canonical_content_json: Value,
    pub canonical_content_text: String,
    pub compression_level: String,
    pub token_estimate: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmContextPackDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("数据库错误: {0}")]
    Database(#[from] DbErr),
}

pub async fn record_or_reuse_context_pack<C>(
    db: &C,
    input: RecordOrReuseContextPackInput,
) -> Result<llm_context_pack::Model, LlmContextPackDbError>
where
    C: ConnectionTrait,
{
    if let Some(existing) = llm_context_pack::Entity::find()
        .filter(llm_context_pack::Column::ProjectId.eq(input.project_id))
        .filter(llm_context_pack::Column::Workflow.eq(input.workflow.as_str()))
        .filter(llm_context_pack::Column::PackKind.eq(input.pack_kind.as_str()))
        .filter(llm_context_pack::Column::CompressionLevel.eq(input.compression_level.as_str()))
        .filter(llm_context_pack::Column::ContentSha256.eq(input.content_sha256.as_str()))
        .filter(llm_context_pack::Column::ConfigHash.eq(input.config_hash.as_str()))
        .one(db)
        .await?
    {
        return Ok(existing);
    }

    let now = Utc::now().naive_utc();
    llm_context_pack::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        scope_kind: Set(input.scope_kind),
        workflow: Set(input.workflow),
        pack_kind: Set(input.pack_kind),
        source_refs_json: Set(input.source_refs_json),
        config_hash: Set(input.config_hash),
        content_sha256: Set(input.content_sha256),
        canonical_content_json: Set(input.canonical_content_json),
        canonical_content_text: Set(input.canonical_content_text),
        compression_level: Set(input.compression_level),
        token_estimate: Set(usize_to_i32(input.token_estimate, "token_estimate")?),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(LlmContextPackDbError::Database)
}

fn usize_to_i32(value: usize, field: &str) -> Result<i32, LlmContextPackDbError> {
    i32::try_from(value).map_err(|_| {
        LlmContextPackDbError::InvalidArgument(format!("LLM 上下文字段 {field} 超出 i32 范围"))
    })
}
