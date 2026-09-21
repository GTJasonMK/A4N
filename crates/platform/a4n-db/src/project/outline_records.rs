mod chapters;
mod cleanup;
mod contracts;
mod mutation;
mod parts;
mod progress;
mod provenance;

pub(crate) use chapters::formal_chapter_outline_id_if_unchanged;
pub use chapters::{
    chapter_outline_by_id, chapter_outline_by_project_number, chapter_outlines_for_project,
    chapter_outlines_for_project_chapter_number, formal_chapter_outline_by_project_number,
};
pub use cleanup::{delete_latest_chapter_outline_records, delete_latest_part_outline_records};
pub use contracts::{
    ChapterSourceSnapshot, DeleteLatestOutlineRecordsInput, ManualChapterOutlineSaveInput,
    ManualPartOutlineSaveInput, OutlineDeleteTarget, OutlineGenerationProgressChapter,
    OutlineGenerationProgressPart, OutlineGenerationProgressRows, ProjectChapterOutlineUpdateError,
    ProjectChapterOutlineUpdateInput, ProjectOutlineRecordError, RetrospectiveChapterOutlineAction,
    RetrospectiveChapterOutlineUpsertInput, RetrospectiveChapterOutlineUpsertOutcome,
};
pub(crate) use contracts::{
    chapter_source_snapshot_matches, chapter_source_snapshot_matches_fields,
    chapter_source_snapshot_targets,
};
pub use mutation::{
    save_manual_chapter_outline, save_manual_part_outline, update_chapter_outline_record,
};
pub use parts::{part_outline_by_id, part_outlines_by_ids, part_outlines_for_project};
pub use progress::outline_generation_progress_rows;
pub use provenance::{
    chapter_outline_source_kinds_for_ids, delete_automatic_chapter_outlines_for_source_change,
    upsert_retrospective_chapter_outline_checked,
};
pub(crate) use provenance::{
    clear_chapter_outline_provenance, clear_chapter_outline_provenance_many,
    link_chapter_to_project_outline_if_needed,
};
