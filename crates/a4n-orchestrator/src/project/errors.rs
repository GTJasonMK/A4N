mod content;
mod family;
mod lifecycle;

pub(crate) use content::{
    blueprint_restore_error_to_app, foreshadowing_db_error_to_app_error,
    project_chapter_context_error_to_app_error, project_chapter_outline_update_error_to_app_error,
    project_chapter_scene_record_error_to_app_error,
    project_chapter_scene_reorder_error_to_app_error,
    project_generated_chapter_scene_save_error_to_app_error,
    project_outline_record_error_to_app_error,
};
pub(crate) use family::{
    import_text_project_error_to_app_error, project_basic_record_error_to_app_error,
    project_family_error_to_app_error, project_family_export_error_to_app_error,
    project_family_import_error_to_app_error, project_family_metadata_read_error_to_app_error,
    project_progress_read_error_to_app_error, project_rollback_impact_read_error_to_app_error,
};
pub(crate) use lifecycle::{
    project_duplicate_error_to_app_error, project_mutation_error_to_app_error,
    project_reset_error_to_app_error, project_status_read_error_to_app_error,
    project_status_write_error_to_app_error,
};

#[cfg(test)]
use a4n_db::project::BlueprintRestoreError;

#[cfg(test)]
mod tests;
