use crate::queue::priority::TaskKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QueueKind {
    Llm,
    Image,
    Video,
    Embedding,
    Background,
}

pub(super) fn queue_kind_for_task_type(task_type: &TaskKind) -> QueueKind {
    match task_type {
        TaskKind::LlmCall | TaskKind::TextGeneration => QueueKind::Llm,
        TaskKind::ImageGeneration => QueueKind::Image,
        TaskKind::VideoGeneration | TaskKind::VideoPoll | TaskKind::Assembly => QueueKind::Video,
        TaskKind::EmbeddingCall | TaskKind::EmbeddingDownload => QueueKind::Embedding,
        TaskKind::RagIngestion
        | TaskKind::ImportAnalysis
        | TaskKind::ExportGeneration
        | TaskKind::SourceSync
        | TaskKind::GalgameGeneration
        | TaskKind::MangaGeneration => QueueKind::Background,
    }
}
