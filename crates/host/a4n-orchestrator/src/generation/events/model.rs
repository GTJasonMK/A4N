use super::*;

#[derive(Debug, thiserror::Error)]
pub enum GenerationSessionError {
    #[error("生成会话配置无效: {0}")]
    InvalidConfiguration(String),
    #[error("生成事件合同无效: {0}")]
    Contract(String),
    #[error("生成会话身份或 replay 无效: {0}")]
    InvalidSession(String),
    #[error("生成会话状态 {status:?} 不允许 {operation}")]
    InvalidState {
        status: GenerationTaskStatus,
        operation: &'static str,
    },
    #[error("生成会话已进入终态")]
    TerminalAlreadySet,
    #[error("文本流 {stream_id} offset 不匹配: expected={expected}, actual={actual}")]
    OffsetMismatch {
        stream_id: String,
        expected: u64,
        actual: u64,
    },
    #[error("usage 累计值发生回退")]
    UsageRegression,
    #[error("sequence lease 扩租失败: {0}")]
    SequenceLeaseUnavailable(String),
    #[error("sequence lease 高水位无效: current={current}, next={next}")]
    InvalidSequenceLease { current: u64, next: u64 },
    #[error("生成会话容量已满")]
    SessionCapacityExceeded,
    #[error("生成订阅容量已满")]
    SubscriptionCapacityExceeded,
    #[error("普通 publisher 已封闭: {0}")]
    PublisherSealed(String),
    #[error("时间戳超出生成合同范围")]
    InvalidTimestamp,
}

impl GenerationSessionError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::OffsetMismatch { .. } => "generation_offset_mismatch",
            Self::SequenceLeaseUnavailable(_) | Self::InvalidSequenceLease { .. } => {
                "generation_checkpoint_unavailable"
            }
            Self::SessionCapacityExceeded => "generation_session_capacity_exceeded",
            Self::SubscriptionCapacityExceeded => "generation_subscription_capacity_exceeded",
            Self::TerminalAlreadySet | Self::InvalidState { .. } => {
                "generation_terminal_already_set"
            }
            Self::PublisherSealed(_) => "generation_checkpoint_unavailable",
            Self::InvalidConfiguration(_)
            | Self::Contract(_)
            | Self::InvalidSession(_)
            | Self::UsageRegression
            | Self::InvalidTimestamp => "generation_event_invalid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GenerationDurableCommit {
    pub(crate) checkpoint_revision: i64,
    pub(crate) covers_through_sequence: u64,
    pub(crate) reserved_through_sequence: u64,
    pub(crate) persisted_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GenerationSequenceLeaseUpdate {
    pub(crate) checkpoint_revision: i64,
    pub(crate) reserved_through_sequence: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct SessionCheckpointCandidate {
    pub(crate) snapshot: GenerationTaskSnapshot,
    pub(crate) checkpoint_revision: i64,
    pub(crate) unpersisted_event_bytes: usize,
    pub(crate) last_persisted_at_ms: i64,
}
