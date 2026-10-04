use a4n_db::source_sync::SourceSyncGalgameAnchorRecord;
use a4n_domain::project::SourceSnapshotChapterSummary;

use crate::galgame::GalgamePipelinePromptSourceSpan;

pub(super) fn galgame_regeneration_source_spans(
    chapter: &SourceSnapshotChapterSummary,
    anchors: &[SourceSyncGalgameAnchorRecord],
) -> Vec<GalgamePipelinePromptSourceSpan> {
    anchors
        .iter()
        .map(|anchor| GalgamePipelinePromptSourceSpan {
            id: format!("ch{}:ta{}", chapter.chapter_number, anchor.id),
            text_anchor_id: anchor.id,
            source_snapshot_chapter_id: chapter.id,
            paragraph_index: anchor.paragraph_index,
            start_offset: anchor.start_offset,
            end_offset: anchor.end_offset,
            text: anchor.quote.clone(),
        })
        .collect()
}
