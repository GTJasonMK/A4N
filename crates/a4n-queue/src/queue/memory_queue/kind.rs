use crate::queue::priority::TaskType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QueueKind {
    Llm,
    Image,
    Video,
    Embedding,
    Background,
}

pub(super) fn queue_kind_for_task_type(task_type: &TaskType) -> QueueKind {
    match task_type {
        TaskType::LlmCall => QueueKind::Llm,
        TaskType::ImageGeneration => QueueKind::Image,
        TaskType::VideoGeneration | TaskType::VideoPoll | TaskType::Assembly => QueueKind::Video,
        TaskType::EmbeddingCall | TaskType::EmbeddingDownload => QueueKind::Embedding,
        TaskType::RagIngestion
        | TaskType::ImportAnalysis
        | TaskType::ExportGeneration
        | TaskType::SourceSync => QueueKind::Background,
    }
}
