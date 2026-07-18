use super::prelude::*;

pub(super) async fn sync_chapter_analysis_outputs(
    state: &AppState,
    ch: &chapter::Model,
) -> Result<(), String> {
    state
        .orchestrator
        .sync_chapter_analysis_foreshadowing_outputs(ch)
        .await
        .map_err(|error| error.to_string())?;
    state
        .orchestrator
        .sync_chapter_rag_records(ch, crate::rag_ingest::chapter_rag_chunking_mode())
        .await?;
    Ok(())
}
