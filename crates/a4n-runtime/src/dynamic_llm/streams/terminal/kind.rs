use a4n_error::AppError;

#[derive(Clone, Copy)]
pub(in crate::dynamic_llm::streams) enum StreamKind {
    #[cfg(feature = "embedded-llm-provider")]
    Embedded,
    Sidecar,
}

impl StreamKind {
    pub(super) fn failed_trace_code(self) -> &'static str {
        match self {
            #[cfg(feature = "embedded-llm-provider")]
            Self::Embedded => "stream_error",
            Self::Sidecar => "sidecar_stream_error",
        }
    }

    pub(super) fn completed_queue_error(self, error: impl std::fmt::Display) -> AppError {
        let context = match self {
            #[cfg(feature = "embedded-llm-provider")]
            Self::Embedded => "LLM stream 已完成，但同步队列完成状态失败",
            Self::Sidecar => "LLM sidecar stream 已完成，但同步队列完成状态失败",
        };
        AppError::Internal(anyhow::anyhow!("{context}: {error}"))
    }

    pub(super) fn failed_queue_error(
        self,
        stream_error: impl std::fmt::Display,
        status_error: impl std::fmt::Display,
    ) -> AppError {
        let context = match self {
            #[cfg(feature = "embedded-llm-provider")]
            Self::Embedded => "同步 LLM stream 队列失败状态失败",
            Self::Sidecar => "同步 LLM sidecar stream 队列失败状态失败",
        };
        AppError::Internal(anyhow::anyhow!("{stream_error}；{context}: {status_error}"))
    }

    pub(super) fn terminal_task_error(self, error: impl std::fmt::Display) -> AppError {
        let context = match self {
            #[cfg(feature = "embedded-llm-provider")]
            Self::Embedded => "LLM stream 队列终态同步任务异常结束",
            Self::Sidecar => "LLM sidecar stream 队列终态同步任务异常结束",
        };
        AppError::Internal(anyhow::anyhow!("{context}: {error}"))
    }
}
