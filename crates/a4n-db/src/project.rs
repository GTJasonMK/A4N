mod basic_record;
mod blueprint_restore;
mod blueprint_sync;
mod chapter_context;
#[path = "project/chapter_scene.rs"]
mod chapter_scene_records;
mod chapter_source;
mod cleanup;
mod duplicate;
mod family_context;
mod family_export;
mod family_metadata;
mod foreshadowing;
mod freeform_reset;
mod material_refs;
mod mutation;
mod outline_records;
mod progress;
mod rollback_impact;
mod shared;
mod source_snapshot;
mod status;

pub use basic_record::{
    ProjectBasicRecordError, derived_project_count, project_import_analysis_progress, project_kind,
    project_title, save_project_import_analysis_progress,
};
pub(crate) use blueprint_restore::validate_blueprint_version_snapshot;
pub use blueprint_restore::{
    BlueprintRestoreError, BlueprintRestoreResult, BlueprintRestoreStatusChange,
    restore_blueprint_version_record,
};
pub use blueprint_sync::*;
pub use chapter_context::*;
pub use chapter_scene_records::*;
pub use chapter_source::*;
pub use cleanup::*;
pub use duplicate::*;
pub use family_context::*;
pub use family_export::*;
pub use family_metadata::*;
pub use foreshadowing::*;
pub use freeform_reset::*;
pub use material_refs::*;
pub use mutation::*;
pub use outline_records::{
    ChapterSourceSnapshot, DeleteLatestOutlineRecordsInput, ManualChapterOutlineSaveInput,
    ManualPartOutlineSaveInput, OutlineDeleteTarget, OutlineGenerationProgressRows,
    ProjectChapterOutlineUpdateError, ProjectChapterOutlineUpdateInput, ProjectOutlineRecordError,
    RetrospectiveChapterOutlineAction, RetrospectiveChapterOutlineUpsertInput,
    RetrospectiveChapterOutlineUpsertOutcome, chapter_outline_by_id,
    chapter_outline_by_project_number, chapter_outline_source_kinds_for_ids,
    chapter_outlines_for_project, chapter_outlines_for_project_chapter_number,
    delete_automatic_chapter_outlines_for_source_change, delete_latest_chapter_outline_records,
    delete_latest_part_outline_records, formal_chapter_outline_by_project_number,
    outline_generation_progress_rows, part_outline_by_id, part_outlines_for_project,
    save_manual_chapter_outline, save_manual_part_outline, update_chapter_outline_record,
    upsert_retrospective_chapter_outline_checked,
};
pub(crate) use outline_records::{
    chapter_source_snapshot_matches, chapter_source_snapshot_targets,
    clear_chapter_outline_provenance, formal_chapter_outline_id_if_unchanged,
    link_chapter_to_project_outline_if_needed,
};
pub use progress::*;
pub use rollback_impact::*;
pub use source_snapshot::*;
pub(crate) use status::delete_project_status_history;
pub use status::{
    ProjectStatusHistoryInput, ProjectStatusReadError, ProjectStatusWriteError,
    insert_project_status_history, project_status_artifacts, project_status_history,
    update_project_status_record, update_project_status_record_in,
};

use shared::{required_text, validate_positive_i64};
