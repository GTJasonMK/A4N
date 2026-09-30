//! Pure task-queue contracts.

mod concurrency;
mod kind;
mod persistence;

pub use concurrency::LlmConcurrencyLimit;
pub use kind::{TaskKind, TaskKindParseError};
pub use persistence::PersistTaskQueueRecordInput;

#[cfg(test)]
mod tests {
    use super::TaskKind;

    #[test]
    fn task_kind_storage_round_trip_has_one_domain_owner() {
        for task_kind in [
            TaskKind::LlmCall,
            TaskKind::ImageGeneration,
            TaskKind::VideoGeneration,
            TaskKind::GalgameGeneration,
            TaskKind::RagIngestion,
            TaskKind::ExportGeneration,
            TaskKind::EmbeddingDownload,
            TaskKind::VideoPoll,
            TaskKind::SourceSync,
            TaskKind::Assembly,
            TaskKind::EmbeddingCall,
            TaskKind::ImportAnalysis,
            TaskKind::MangaGeneration,
            TaskKind::TextGeneration,
        ] {
            assert_eq!(TaskKind::parse(task_kind.as_str()), Some(task_kind));
            assert_eq!(task_kind.as_str().parse::<TaskKind>().unwrap(), task_kind);
        }
    }

    #[test]
    fn task_kind_parse_error_keeps_storage_context() {
        let error = TaskKind::parse_storage("unknown_task_kind").unwrap_err();
        assert_eq!(
            error.to_string(),
            "参数无效：未知任务类型: unknown_task_kind"
        );
    }
}
