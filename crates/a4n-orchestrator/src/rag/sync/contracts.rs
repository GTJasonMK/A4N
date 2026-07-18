use a4n_core::rag::RagDataType;
use serde_json::Value;

use super::super::write::RagEmbeddingInput;

#[derive(Debug, thiserror::Error)]
pub enum RagSyncError {
    #[error("{message}")]
    InvalidRecord { message: String },
    #[error("读取 RAG 已同步记录失败: {message}")]
    ReadStoredMetadata { message: String },
    #[error("读取当前嵌入模型身份失败: {message}")]
    ReadEmbeddingIdentity { message: String },
    #[error("{message}")]
    Store { message: String },
    #[error("{message}")]
    Delete { message: String },
    #[error("参数无效：{message}")]
    InvalidDeleteInput { message: String },
    #[error("向量检索失败：{message}")]
    DeleteChapter { message: String },
    #[error("向量检索失败：{message}")]
    ClearProject { message: String },
    #[error("清理过期 RAG 记录失败: {message}")]
    DeleteStale { message: String },
    #[error("清理过期 RAG 记录数量超过 u32 上限：{deleted}")]
    DeletedCountOverflow { deleted: String },
    #[error("{message}")]
    EmbeddingIdentityMismatch { message: String },
}

#[derive(Debug, Clone)]
pub struct RagRecord {
    pub chunk_id: String,
    pub text: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RagRecordSyncOutcome {
    Added,
    Updated,
    Skipped,
}

#[derive(Debug, Clone)]
pub struct PreparedRagRecordSync {
    pub(super) record: RagRecord,
    pub(super) outcome: RagRecordSyncOutcome,
}

#[derive(Debug, Clone)]
pub struct DeleteRagRecordsInput {
    pub project_id: i64,
    pub data_type: Option<RagDataType>,
    pub ids: Vec<String>,
    pub id_prefixes: Vec<String>,
}

impl PreparedRagRecordSync {
    pub fn outcome(&self) -> RagRecordSyncOutcome {
        self.outcome
    }

    pub(super) fn embedding_input(&self) -> RagEmbeddingInput {
        RagEmbeddingInput {
            chunk_id: self.record.chunk_id.clone(),
            text: self.record.text.clone(),
            metadata: self.record.metadata.clone(),
        }
    }
}
