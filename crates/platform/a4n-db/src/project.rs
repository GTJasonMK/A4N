mod basic_record;
mod blueprint_restore;
mod blueprint_sync;
pub mod bundle;
mod chapter;
pub mod delete;
pub mod dependency_graph;
pub mod family;
pub mod reset;

mod cleanup;
mod duplicate;

mod foreshadowing;
mod freeform_reset;
mod material_refs;
mod mutation;
mod outline_records;
mod progress;
mod records;
mod rollback_impact;
mod shared;
mod source_snapshot;
mod standalone_comic_source;
mod status;

pub(crate) use basic_record::derived_project_count;
pub use basic_record::{
    ProjectBasicRecordError, project_import_analysis_progress, project_kind, project_title,
    save_project_import_analysis_progress,
};
pub(crate) use blueprint_restore::validate_blueprint_version_snapshot;
pub use blueprint_restore::{BlueprintRestoreError, restore_blueprint_version_record};
pub(crate) use blueprint_sync::normalized_blueprint_identity;
pub use blueprint_sync::{
    BlueprintCharacterDraft, BlueprintRelationshipDraft, BlueprintSyncError,
    blueprint_characters_for_project, blueprint_relationships_for_project,
    blueprint_sources_need_review, blueprint_version_by_project_version,
    blueprint_versions_by_project, edit_blueprint_characters, edit_blueprint_relationships,
    merge_blueprint_characters, merge_blueprint_relationships, novel_blueprint_for_project,
};
pub(crate) use chapter::chapter_record;
pub use chapter::context::{
    ChapterGenerationReferenceContextRows, ChapterObjectProfileContext, ChapterProfileEvidence,
    ChapterSceneProfileContext, ProjectChapterContextError, chapter_by_id,
    chapter_generation_reference_context_rows, chapter_project_id, chapters_before_number,
    character_states_for_profiles_before_chapter, character_sync_chapters,
    import_analysis_chapter_candidates, object_profiles_before_chapter,
    protagonist_profiles_for_project, protagonist_profiles_for_project_in,
    refresh_project_current_chapter, scene_profiles_before_chapter,
};
pub use chapter::scene::{
    ProjectChapterSceneRecordError, ProjectChapterSceneUpdateInput, ProjectChapterSceneWriteError,
    chapter_by_project_number, chapter_scene_by_id, chapter_scene_reorder_project_id,
    chapter_scenes_by_chapter, delete_chapter_scene_record, reorder_chapter_scene_records,
    save_generated_chapter_scene_record, update_chapter_scene_record,
};
pub use chapter::source::{
    ResolveChapterSourceError, resolve_chapter_source, resolve_snapshot_sources,
    resolve_video_chapter_source,
};
pub use cleanup::ProjectCleanupDbError;
pub(crate) use cleanup::{
    ProjectChapterAdaptationDeleteScope, ProjectConversationCleanupResult,
    ProjectGenerationMediaGraphCleanupResult, ProjectOptimizationCleanupResult,
    ProjectTaskQueueCleanupResult, ProjectTextAnchorDeleteScope, delete_project_blueprints,
    delete_project_chapter_adaptation_records, delete_project_conversations,
    delete_project_optimization_tree, delete_project_text_anchors,
};
pub(crate) use cleanup::{
    delete_project_generation_media_graph, delete_project_task_queue_records,
    project_generated_media_paths,
};
pub use duplicate::{
    ProjectDuplicateError, duplicate_derived_project_record, duplicate_freeform_project_record,
    duplicate_standalone_comic_project_record, duplicate_text_project_record,
};
pub use family::context::{
    ChapterMangaSourceSummary, MangaCharacterPortraitRefSummary, ProjectFamilyContext,
    ProjectFamilyDbError, chapter_manga_source_by_id, chapter_manga_source_by_number,
    chapter_manga_sources_by_ids, derived_project_family_access_ids,
    derived_project_family_context, manga_character_portrait_refs,
    project_or_derived_family_access_ids, project_source_snapshot_allowed,
};
pub(crate) use family::context::{
    validate_project_family_snapshot_chapter, validate_project_family_source_context,
};
pub use family::export::{
    ProjectFamilyExportError, ProjectFamilyExportRows, ProjectFamilyExportSelection,
    ProjectFamilyTextCoreRows, derived_project_export_selection, project_family_export_rows,
    project_family_export_selection,
};
pub use family::metadata::{
    ProjectFamilyMetadata, ProjectFamilyMetadataQuery, ProjectFamilyMetadataReadError,
    project_family_metadata, project_family_metadata_batch,
};
pub(crate) use foreshadowing::restore_foreshadowing_fact;
pub use foreshadowing::{
    ForeshadowingAnalysisDraft, ForeshadowingAnalysisSyncResult, ForeshadowingDbError,
    ForeshadowingFactUpdate, create_foreshadowing_record, delete_foreshadowing_record,
    foreshadowing_records_by_project, sync_chapter_analysis_foreshadowing,
    update_foreshadowing_record,
};
pub(crate) use freeform_reset::FreeformGeneratedResetTargets;
pub use freeform_reset::{
    FreeformGeneratedResetDbError, reset_freeform_video_generated_data_records,
};
pub use material_refs::{primary_material_image_ids_for_projects, protagonist_profile_project_id};
pub use mutation::{
    CreateFreeformVideoProjectParams, CreateProjectRecordInput,
    CreateStandaloneComicProjectParams, PROJECT_KIND_COMIC, PROJECT_KIND_TEXT,
    PROJECT_KIND_VIDEO, ProjectMutationError, SOURCE_SYNC_CURRENT, UpdateProjectRecordInput,
    create_freeform_video_project_record, create_project_record,
    create_standalone_comic_project_record, is_valid_source_sync_status,
    normalize_derived_project_kind, normalize_project_kind, normalize_project_type,
    project_by_id_record, project_ids, project_summary_by_id_record, update_project_record,
};
pub(crate) use mutation::{
    SOURCE_SYNC_CONFLICT, SOURCE_SYNC_DETACHED, SOURCE_SYNC_ROOT_UPDATED,
    mark_derived_projects_root_updated_in_record, project_by_id_for_update,
    project_write_identity_for_update,
};
pub use outline_records::{
    DeleteLatestOutlineRecordsInput, ManualChapterOutlineSaveInput, ManualPartOutlineSaveInput,
    OutlineDeleteTarget, OutlineGenerationProgressChapter, OutlineGenerationProgressPart,
    OutlineGenerationProgressRows, ProjectChapterOutlineUpdateError,
    ProjectChapterOutlineUpdateInput, ProjectOutlineRecordError, RetrospectiveChapterOutlineAction,
    RetrospectiveChapterOutlineUpsertInput, RetrospectiveChapterOutlineUpsertOutcome,
    chapter_outline_by_id, chapter_outline_by_project_number, chapter_outline_source_kinds_for_ids,
    chapter_outlines_for_project, chapter_outlines_for_project_chapter_number,
    chapter_source_snapshot_from_model, delete_automatic_chapter_outlines_for_source_change,
    delete_latest_chapter_outline_records, delete_latest_part_outline_records,
    formal_chapter_outline_by_project_number, outline_generation_progress_rows, part_outline_by_id,
    part_outlines_by_ids, part_outlines_for_project, save_manual_chapter_outline,
    save_manual_part_outline, update_chapter_outline_record,
    upsert_retrospective_chapter_outline_checked,
};
pub(crate) use outline_records::{
    chapter_source_snapshot_matches, chapter_source_snapshot_matches_fields,
    chapter_source_snapshot_targets, clear_chapter_outline_provenance,
    clear_chapter_outline_provenance_many, formal_chapter_outline_id_if_unchanged,
    link_chapter_to_project_outline_if_needed,
};
pub use progress::{
    ProjectProgressQuery, ProjectProgressReadError, project_progress_summaries,
    project_progress_summary,
};
pub(crate) use records::{project_record_from_model, project_record_into_model};
pub use rollback_impact::{ProjectRollbackImpactReadError, project_rollback_impacts};
pub use source_snapshot::{
    CreateDerivedProjectParams, DerivedChapterScope, create_derived_project_record,
    source_snapshot_chapter_content_by_snapshot_and_source, source_snapshot_chapters,
};
pub use source_snapshot::{authoring_scene_profiles, scene_profiles_for_source};
pub(crate) use source_snapshot::{create_source_snapshot_in_record, source_snapshot_version_hash};
pub use standalone_comic_source::{
    PrepareStandaloneComicSourceDbInput, StandaloneComicSourceDbError,
    prepare_standalone_comic_source_chapter,
};
pub(crate) use status::delete_project_status_history;
pub use status::{
    ProjectStatusReadError, ProjectStatusWriteError, insert_project_status_history,
    project_status_artifacts, project_status_history, update_project_status_record,
    update_project_status_record_in,
};

use shared::{required_text, validate_positive_i64};
