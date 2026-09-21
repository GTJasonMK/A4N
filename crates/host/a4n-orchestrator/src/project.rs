pub(crate) use a4n_db::project::PROJECT_KIND_TEXT;
pub(crate) use a4n_db::project::{
    ChapterMangaSourceSummary, ForeshadowingAnalysisDraft, PROJECT_KIND_VIDEO,
    ProjectFamilyExportRows, ProjectFamilyExportSelection, ProjectFamilyTextCoreRows,
    ProjectGeneratedChapterSceneSaveInput, SOURCE_SYNC_CURRENT, is_valid_source_sync_status,
    normalize_derived_project_kind, normalize_project_kind, normalize_project_type,
};
pub(crate) use a4n_db::project::{
    ProjectFamilyContext, ProjectFamilyMetadata, ProjectFamilyMetadataQuery, ProjectProgressQuery,
};
pub(crate) use a4n_db::project_family_import::{
    ProjectFamilyImportPackage, ProjectFamilyMediaFileImporter, snapshot_profile_key,
};
pub(crate) use a4n_domain::project::{ProjectProgressSummary, ProjectSourceMode};
use a4n_domain::project_reset::{
    FreeformVideoGeneratedDataResetResult, ProjectGeneratedDataResetOutcome,
};

/// application 层项目生成数据重置结果：按项目类型分支，供 Host 做文件/RAG 副作用。
pub enum ProjectGeneratedDataResetApplicationResult {
    /// 文本/派生等标准重置。
    Standard(ProjectGeneratedDataResetOutcome),
    /// 自由画布视频重置。
    Freeform {
        result: FreeformVideoGeneratedDataResetResult,
    },
}

/// 项目 ID 必须为正。
pub(crate) fn ensure_positive_project_id(project_id: i64) -> Result<(), a4n_error::AppError> {
    if project_id <= 0 {
        return Err(a4n_error::AppError::InvalidArgument(
            "项目 ID 必须大于 0".to_string(),
        ));
    }
    Ok(())
}

mod chapters;
mod duplication;
mod errors;
mod family_package;
mod family_transfer;
mod generation_params_rules;
mod json_field_rules;
mod package_rules;
mod package_transfer;
mod project_bundle;
mod video_studio_package;
mod video_studio_transfer;

pub(crate) use a4n_api_contract::project_transfer::{
    ExportDerivedProjectPackageApplicationInput, ExportProjectFamilyPackageApplicationInput,
    ExportVideoStudioProjectApplicationInput, ExportedProjectPackage,
    ImportProjectPackageApplicationInput, MediaFileEntry, ProjectFamilyTransferApplicationPort,
    ProjectPackageTransferError, ProjectPackageTransferHostPort,
    VideoStudioProjectTransferApplicationPort,
};
#[cfg(test)]
pub(crate) use family_package::PROJECT_FAMILY_PACKAGE_VERSION;
pub(crate) use family_package::{
    MANIFEST_PATH as FAMILY_PACKAGE_MANIFEST_PATH, ProjectFamilyPackage,
};
pub(crate) use family_package::{
    assemble_project_family_export_package, family_media_package_path,
    validate_media_manifest as validate_family_media_manifest,
    validate_package_manifest as validate_family_package_manifest,
};
pub(crate) use family_transfer::{
    imported_project_family_media_path, project_family_export_media_files,
};
pub(crate) use generation_params_rules::{
    generation_spec_params_object, optional_params_i64, required_generation_spec_target_id,
    required_params_i64, required_params_i64_array, required_params_object, required_params_string,
    required_params_string_array, validate_generation_spec_canonical_key,
    validate_params_id_equals, validate_required_galgame_cg_importance,
    validate_snapshot_character_profile_id,
};
pub(crate) use json_field_rules::{
    required_json_i32_field, required_json_i64_field, required_json_string_field,
    required_optional_json_string_field,
};
pub(crate) use package_rules::{ImportMediaPayload, media_asset_path, media_file_key};
pub(crate) use package_rules::{
    collect_unique_ids, hash_json, hash_text, is_valid_snapshot_outline_kind,
    is_valid_snapshot_profile_kind, parse_chapter_scope_json, required_package_extension,
    required_path_extension, sha256_bytes, validate_freeform_video_project,
    validate_import_unique_ids, validate_import_unique_key_strings,
    validate_snapshot_payload_identity,
};
pub(crate) use package_transfer::{
    import_result_with_file_rollback, index_import_media_payload, project_package_export_input,
};

pub(crate) use project_bundle::{
    MANIFEST_PATH as PROJECT_BUNDLE_MANIFEST_PATH, ProjectBundlePackage, ProjectBundleProviderSlot,
    add_canvas_provider_slot, assemble_project_bundle,
};

pub(crate) use video_studio_package::{MANIFEST_PATH, VideoStudioProjectPackage};
#[cfg(test)]
pub(crate) use video_studio_package::{PACKAGE_MODE, VIDEO_STUDIO_PACKAGE_VERSION};
pub(crate) use video_studio_package::{
    assemble_video_studio_export_package, assemble_video_studio_project_canvas_package,
    validate_package_manifest,
};
pub(crate) use video_studio_transfer::video_studio_export_media_files;

pub(crate) use chapters::{
    is_character_sync_available_chapter, validate_chapter_generation_number,
};
pub(crate) use errors::blueprint_restore_error_to_app;
pub(crate) use errors::foreshadowing_db_error_to_app_error;
pub(crate) use errors::project_basic_record_error_to_app_error;
pub(crate) use errors::project_chapter_context_error_to_app_error;
pub(crate) use errors::project_chapter_scene_record_error_to_app_error;
pub(crate) use errors::project_chapter_scene_write_error_to_app_error;
pub(crate) use errors::project_duplicate_error_to_app_error;
pub(crate) use errors::project_family_error_to_app_error;
pub(crate) use errors::project_family_export_error_to_app_error;
pub(crate) use errors::project_family_import_error_to_app_error;
pub(crate) use errors::project_family_metadata_read_error_to_app_error;
pub(crate) use errors::project_mutation_error_to_app_error;
pub(crate) use errors::project_outline_record_error_to_app_error;
pub(crate) use errors::project_progress_read_error_to_app_error;
pub(crate) use errors::project_reset_error_to_app_error;
pub(crate) use errors::project_status_read_error_to_app_error;
pub(crate) use errors::project_status_write_error_to_app_error;
