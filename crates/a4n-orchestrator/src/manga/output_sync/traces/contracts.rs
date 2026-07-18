use a4n_db::generation::{GenerationRunCheckpointContext, RecordCompletedGenerationRunInput};

pub(super) struct MangaGenerationTraceRecord {
    pub(super) run: RecordCompletedGenerationRunInput,
    pub(super) checkpoint: GenerationRunCheckpointContext,
}
