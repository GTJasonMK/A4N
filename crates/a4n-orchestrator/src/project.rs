pub use a4n_db::project::{
    BlueprintRestoreError, BlueprintRestoreResult, ChapterGenerationReferenceContextRows,
    ChapterMangaSourceSummary, CreateDerivedProjectParams, CreateFreeformVideoProjectParams,
    CreateProjectRecordInput, DeleteLatestOutlineRecordsInput, DerivedChapterScope,
    ForeshadowingAnalysisDraft, ForeshadowingAnalysisSyncResult, FreeformGeneratedResetTargets,
    FreeformVideoGeneratedDataResetResult, MangaCharacterPortraitRefSummary,
    ManualChapterOutlineSaveInput, ManualPartOutlineSaveInput, OutlineDeleteTarget,
    OutlineGenerationProgressRows, PROJECT_KIND_COMIC, PROJECT_KIND_GALGAME, PROJECT_KIND_TEXT,
    PROJECT_KIND_VIDEO, PROJECT_TYPE_NOVEL_LONG, PROJECT_TYPE_NOVEL_SHORT, PROJECT_TYPE_PROSE,
    PROJECT_TYPE_SCRIPT, ProjectBasicRecordError, ProjectBlueprintCleanupResult,
    ProjectChapterOutlineUpdateError, ProjectChapterOutlineUpdateInput,
    ProjectChapterSceneRecordError, ProjectChapterSceneReorderError,
    ProjectChapterSceneReorderInput, ProjectChapterSceneReorderScope,
    ProjectConversationCleanupResult, ProjectFamilyContext, ProjectFamilyExportRows,
    ProjectFamilyExportSelection, ProjectFamilyMetadata, ProjectFamilyMetadataQuery,
    ProjectFamilyMetadataReadError, ProjectFamilyTextCoreRows,
    ProjectGeneratedArtifactLinkCleanupResult, ProjectGeneratedArtifactRowCleanupResult,
    ProjectGeneratedChapterSceneSaveError, ProjectGeneratedChapterSceneSaveInput,
    ProjectGenerationMediaGraphCleanupResult, ProjectGenerationRunCleanupResult,
    ProjectMediaCopyError, ProjectMediaFileCopier, ProjectMutationError,
    ProjectOptimizationCleanupResult, ProjectProgressQuery, ProjectProgressReadError,
    ProjectTaskQueueCleanupResult, ProjectTextAnchorCleanupResult, ProjectTextAnchorDeleteScope,
    ProjectWorkflowCleanupResult, SOURCE_SYNC_CONFLICT, SOURCE_SYNC_CURRENT, SOURCE_SYNC_DETACHED,
    SOURCE_SYNC_ROOT_UPDATED, UpdateProjectRecordInput, duplicate_freeform_project_record,
    is_freeform_generated_media_asset, is_freeform_imported_canvas_image,
    is_freeform_imported_canvas_video, is_valid_source_sync_status, normalize_derived_project_kind,
    normalize_project_kind, normalize_project_type,
};
pub use a4n_db::project_family_import::{
    ImportIdMaps, ProjectFamilyImportPackage, ProjectFamilyMediaFileImporter,
    media_asset_status_for_import_insert, remap_chapter_hashes_json, remap_conflict_affected_refs,
    remap_generation_spec_params_json, remap_generation_spec_source_version_hash,
    remap_media_request_snapshot, remap_profile_hashes_json, remap_source_snapshot_outline_payload,
    remap_source_snapshot_profile_payload, remap_source_sync_decision_payload,
    remap_source_sync_target_ref, snapshot_profile_key,
};
pub use a4n_db::project_reset::{
    ProjectGeneratedDataResetOutcome, ProjectGeneratedDataResetResult,
};
pub use a4n_domain::project::{
    ProjectProgressSummary, ProjectSourceMode, SourceSnapshotChapterSummary,
};

use a4n_core::models::character::protagonist_profile;
use a4n_db::character::{CharacterProfileDeletionSummary, CharacterTimelineDeletionSummary};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
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

    pub(crate) fn merge(&mut self, other: Self) {
        self.stale_profile_ids.extend(other.stale_profile_ids);
        self.stale_character_state_ids
            .extend(other.stale_character_state_ids);
        self.stale_attribute_event_ids
            .extend(other.stale_attribute_event_ids);
        self.stale_foreshadowing_ids
            .extend(other.stale_foreshadowing_ids);
        self.changed_foreshadowing_ids
            .extend(other.changed_foreshadowing_ids);
        self.media_file_paths.extend(other.media_file_paths);
        self.scene_records_changed |= other.scene_records_changed;

        self.stale_profile_ids.sort_unstable();
        self.stale_profile_ids.dedup();
        self.stale_character_state_ids.sort_unstable();
        self.stale_character_state_ids.dedup();
        self.stale_attribute_event_ids.sort_unstable();
        self.stale_attribute_event_ids.dedup();
        self.stale_foreshadowing_ids.sort_unstable();
        self.stale_foreshadowing_ids.dedup();
        self.changed_foreshadowing_ids.sort_unstable();
        self.changed_foreshadowing_ids.dedup();
        self.media_file_paths.sort();
        self.media_file_paths.dedup();
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

/// application 层项目生成数据重置结果：按项目类型分支，供 Host 做文件/RAG 副作用。
#[derive(Debug, Clone)]
pub enum ProjectGeneratedDataResetApplicationResult {
    /// 文本/派生等标准重置。
    Standard(ProjectGeneratedDataResetOutcome),
    /// 自由画布视频重置。
    Freeform {
        project: a4n_core::models::project::novel_project::Model,
        result: FreeformVideoGeneratedDataResetResult,
    },
}

/// 项目 ID 必须为正。
pub fn ensure_positive_project_id(project_id: i64) -> Result<(), a4n_core::error::AppError> {
    if project_id <= 0 {
        return Err(a4n_core::error::AppError::InvalidArgument(
            "项目 ID 必须大于 0".to_string(),
        ));
    }
    Ok(())
}

mod chapter_scene_rules;
mod chapters;
mod cleanup;
mod duplication;
mod errors;
mod family_package;
mod family_transfer;
mod generation_params_rules;
mod input_rules;
mod json_field_rules;
mod package_rules;
mod package_transfer;
mod video_studio_package;
mod video_studio_transfer;

pub use duplication::{DuplicateProjectApplicationInput, ProjectDuplicationHostPort};
pub use generation_params_rules::{
    generation_spec_params_object, optional_params_i64, required_generation_spec_target_id,
    required_params_i64, required_params_i64_array, required_params_object, required_params_string,
    required_params_string_array, validate_generation_spec_canonical_key,
    validate_params_id_equals, validate_required_galgame_cg_importance,
    validate_snapshot_character_profile_id,
};
pub use input_rules::{
    ProjectDuplicatePath, is_freeform_video_generated_data_reset_project, optional_text,
    project_list_offset, required_text, resolve_project_duplicate_path,
};
pub use json_field_rules::{
    required_json_i32_field, required_json_i64_field, required_json_string_field,
    required_optional_json_string_field,
};
pub(crate) use package_rules::collect_unique_ids;
pub use package_rules::{
    hash_json, hash_text, is_valid_snapshot_outline_kind, is_valid_snapshot_profile_kind,
    media_asset_path, media_file_key, parse_chapter_scope_json, required_package_extension,
    required_path_extension, sha256_bytes, validate_freeform_video_project,
    validate_import_unique_ids, validate_import_unique_key_strings,
    validate_snapshot_payload_identity,
};
pub(crate) use package_transfer::import_result_with_file_rollback;
pub use package_transfer::{
    ExportedProjectPackage, ImportProjectPackageApplicationInput, ProjectPackageFileHostPort,
    ProjectPackageTransferError,
};

pub use family_package::{
    FamilyImportMediaPayload, MANIFEST_PATH as FAMILY_PACKAGE_MANIFEST_PATH, MEDIA_FILE_FIELDS,
    MediaFileEntry, PROJECT_FAMILY_PACKAGE_VERSION, ProjectFamilyPackage, TextCorePackage,
    assemble_project_family_export_package, family_media_package_path,
    snapshot_profile_key as family_snapshot_profile_key, validate_export_same_user,
    validate_export_snapshot_coverage, validate_media_bytes as validate_family_media_bytes,
    validate_media_manifest as validate_family_media_manifest,
    validate_package_manifest as validate_family_package_manifest,
};
pub use family_transfer::{
    ExportDerivedProjectPackageApplicationInput, ExportProjectFamilyPackageApplicationInput,
    ProjectFamilyTransferHostPort, ReadProjectFamilyPackage,
};
pub(crate) use family_transfer::{
    imported_project_family_media_path, project_family_export_media_files,
};

pub use video_studio_package::{
    ImportMediaPayload, MANIFEST_PATH, PACKAGE_MODE, VIDEO_STUDIO_PACKAGE_VERSION,
    VideoStudioMediaFileEntry, VideoStudioProjectPackage, assemble_video_studio_export_package,
    media_file_key as video_studio_media_file_key, validate_media_entry_bytes,
    validate_media_entry_owner, validate_package_manifest,
};
pub(crate) use video_studio_transfer::video_studio_export_media_files;
pub use video_studio_transfer::{
    ExportVideoStudioProjectApplicationInput, ReadVideoStudioProjectPackage,
    VideoStudioProjectTransferHostPort,
};

pub(crate) use chapter_scene_rules::chapter_scene_update_record_input;
pub(crate) use chapters::{
    is_character_sync_available_chapter, validate_chapter_generation_number,
};
pub(crate) use cleanup::cleanup_chapter_derived_data_in;
pub(crate) use errors::blueprint_restore_error_to_app;
pub(crate) use errors::foreshadowing_db_error_to_app_error;
pub(crate) use errors::project_basic_record_error_to_app_error;
pub(crate) use errors::project_chapter_context_error_to_app_error;
pub(crate) use errors::project_chapter_outline_update_error_to_app_error;
pub(crate) use errors::project_chapter_scene_record_error_to_app_error;
pub(crate) use errors::project_chapter_scene_reorder_error_to_app_error;
pub(crate) use errors::project_duplicate_error_to_app_error;
pub(crate) use errors::project_family_error_to_app_error;
pub(crate) use errors::project_family_export_error_to_app_error;
pub(crate) use errors::project_family_import_error_to_app_error;
pub(crate) use errors::project_family_metadata_read_error_to_app_error;
pub(crate) use errors::project_generated_chapter_scene_save_error_to_app_error;
pub(crate) use errors::project_mutation_error_to_app_error;
pub(crate) use errors::project_outline_record_error_to_app_error;
pub(crate) use errors::project_progress_read_error_to_app_error;
pub(crate) use errors::project_reset_error_to_app_error;
pub(crate) use errors::project_rollback_impact_read_error_to_app_error;
pub(crate) use errors::project_status_read_error_to_app_error;
pub(crate) use errors::project_status_write_error_to_app_error;
