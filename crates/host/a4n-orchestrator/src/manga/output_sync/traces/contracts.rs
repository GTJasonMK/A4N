use a4n_db::generation::GenerationRunCheckpointContext;
use a4n_domain::generation::RecordCompletedGenerationRunInput;

pub(super) struct MangaPageGenerationTraceRecord {
    pub(super) run: RecordCompletedGenerationRunInput,
    pub(super) checkpoint: GenerationRunCheckpointContext,
}
