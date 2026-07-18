pub use a4n_db::project::{
    BlueprintRestoreError, BlueprintRestoreResult, ChapterGenerationReadiness,
    ChapterGenerationReferenceContextRows, ChapterMangaSourceSummary, CreateDerivedProjectParams,
    CreateFreeformVideoProjectParams, CreateProjectRecordInput, DERIVED_PROJECT_STATUS_DRAFT,
    DerivedChapterScope, DerivedProjectMediaFileCopier, ForeshadowingAnalysisDraft,
    ForeshadowingAnalysisSyncResult, FreeformGeneratedResetTargets, FreeformProjectMediaFileCopier,
    FreeformVideoGeneratedDataResetResult, ImportBlueprintUpsertInput,
    ImportChapterOutlineUpsertInput, ImportPartOutlineUpsertInput, ImportTextProjectBlueprint,
    ImportTextProjectBlueprintVersion, ImportTextProjectChapter, ImportTextProjectChapterOutline,
    ImportTextProjectCharacter, ImportTextProjectError, ImportTextProjectPartOutline,
    ImportTextProjectPayload, ImportTextProjectRecordInput, ImportTextProjectRecordResult,
    ImportTextProjectRelationship, MangaCharacterPortraitRefSummary, OutlineGenerationProgressRows,
    PROJECT_KIND_COMIC, PROJECT_KIND_GALGAME, PROJECT_KIND_TEXT, PROJECT_KIND_VIDEO,
    PROJECT_TYPE_NOVEL_LONG, PROJECT_TYPE_NOVEL_SHORT, PROJECT_TYPE_PROSE, PROJECT_TYPE_SCRIPT,
    ProjectBasicRecordError, ProjectBlueprintCleanupResult, ProjectChapterOutlineUpdateError,
    ProjectChapterOutlineUpdateInput, ProjectChapterSceneRecordError,
    ProjectChapterSceneReorderError, ProjectChapterSceneReorderInput,
    ProjectChapterSceneReorderScope, ProjectChapterSceneUpdateInput,
    ProjectConversationCleanupResult, ProjectFamilyContext, ProjectFamilyExportRows,
    ProjectFamilyExportSelection, ProjectFamilyMetadata, ProjectFamilyMetadataQuery,
    ProjectFamilyMetadataReadError, ProjectFamilyTextCoreRows,
    ProjectGeneratedArtifactLinkCleanupResult, ProjectGeneratedArtifactRowCleanupResult,
    ProjectGeneratedChapterSceneSaveError, ProjectGeneratedChapterSceneSaveInput,
    ProjectGenerationMediaGraphCleanupResult, ProjectGenerationRunCleanupResult,
    ProjectMediaCopyError, ProjectMutationError, ProjectOptimizationCleanupResult,
    ProjectProgressQuery, ProjectProgressReadError, ProjectProgressStatus, ProjectProgressSummary,
    ProjectRollbackImpactQuery, ProjectRollbackImpactReadError, ProjectRollbackImpactSummary,
    ProjectStatusArtifacts, ProjectStatusHistorySummary, ProjectTaskQueueCleanupResult,
    ProjectTextAnchorCleanupResult, ProjectTextAnchorDeleteScope, ProjectWorkflowCleanupResult,
    SOURCE_MODE_DERIVED, SOURCE_MODE_FREEFORM, SOURCE_MODE_SELF, SOURCE_SYNC_CONFLICT,
    SOURCE_SYNC_CURRENT, SOURCE_SYNC_DETACHED, SOURCE_SYNC_ROOT_UPDATED,
    SourceSnapshotChapterSummary, UpdateProjectRecordInput, UpdateProjectStatusRecordInput,
    duplicate_freeform_project_record, is_freeform_generated_media_asset,
    is_freeform_imported_canvas_image, is_freeform_imported_canvas_video,
    is_valid_derived_project_status, is_valid_source_sync_status, normalize_derived_project_kind,
    normalize_project_kind, normalize_project_type,
};
pub use a4n_db::project_dependency_graph::{AssetDependencyGraph, AssetGraphNodeDetail};
pub use a4n_db::project_family_import::*;
pub use a4n_db::project_reset::{
    ProjectGeneratedDataResetOutcome, ProjectGeneratedDataResetResult, ProjectStatusRollbackOutcome,
};

use a4n_core::models::{
    asset::object_profile,
    chapter::{chapter, chapter_outline, chapter_scene, part_outline},
    character::{character_state_index, protagonist_profile},
    project::{novel_blueprint, source_snapshot_chapter},
    scene::scene_profile,
};
use a4n_db::character::{CharacterProfileDeletionSummary, CharacterTimelineDeletionSummary};

#[derive(Debug, Clone, Default)]
pub struct ChapterDerivedCleanupArtifacts {
    pub stale_profile_ids: Vec<i64>,
    pub stale_character_state_ids: Vec<i64>,
    pub stale_attribute_event_ids: Vec<i64>,
    pub stale_foreshadowing_ids: Vec<i64>,
    pub changed_foreshadowing_ids: Vec<i64>,
    pub media_file_paths: Vec<String>,
    pub scene_records_changed: bool,
}

impl ChapterDerivedCleanupArtifacts {
    fn extend_deleted_profile(&mut self, summary: CharacterProfileDeletionSummary) {
        self.stale_profile_ids.push(summary.profile_id);
        self.stale_character_state_ids
            .extend(summary.state_index_ids);
        self.stale_attribute_event_ids
            .extend(summary.attribute_event_ids);
        self.media_file_paths.extend(summary.portrait_paths);
    }

    fn extend_character_timeline(&mut self, artifacts: CharacterTimelineDeletionSummary) {
        self.stale_character_state_ids
            .extend(artifacts.state_index_ids);
        self.stale_attribute_event_ids
            .extend(artifacts.attribute_event_ids);
    }

    fn extend_media_file_paths(&mut self, paths: Vec<String>) {
        self.media_file_paths.extend(paths);
    }
}

#[derive(Debug, Clone)]
pub struct ChapterDerivedCleanupResult {
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub artifacts: ChapterDerivedCleanupArtifacts,
}

pub fn chapter_cleanup_profile_state_should_restore_base(
    profile: &protagonist_profile::Model,
    chapter_number: i32,
) -> bool {
    chapter_number <= 0
        || profile
            .first_seen_chapter()
            .is_some_and(|first_seen| chapter_number < first_seen)
}

pub fn chapter_cleanup_requires_snapshot(
    profile: &protagonist_profile::Model,
    chapter_number: i32,
) -> bool {
    profile.last_synced_chapter >= chapter_number
        && !chapter_cleanup_profile_state_should_restore_base(profile, chapter_number - 1)
}

mod basic;
mod chapters;
mod cleanup;
mod dependency_graph;
mod errors;
mod family_media;
mod status;

pub(crate) use errors::project_chapter_context_error_to_app_error;
pub(crate) use errors::project_mutation_error_to_app_error;
pub(crate) use errors::project_outline_record_error_to_app_error;
pub(crate) use errors::project_status_write_error_to_app_error;
use errors::*;
