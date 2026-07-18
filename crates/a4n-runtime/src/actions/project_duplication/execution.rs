use super::DuplicateProjectInput;
use super::media::RuntimeProjectMediaFileCopier;
use super::rules::{ensure_text_project_for_copy, required_project_status, required_text};
use crate::AppState;
use crate::actions::{access, app_error_message};
use a4n_core::models::project::novel_project;
use a4n_orchestrator::project::{PROJECT_KIND_TEXT, SOURCE_MODE_FREEFORM};

pub async fn duplicate_project(
    state: &AppState,
    user_id: i64,
    input: DuplicateProjectInput,
) -> Result<novel_project::Model, String> {
    let original = project_record_by_id(state, input.project_id).await?;
    access::verify_project_editor_string(state, user_id, original.id).await?;
    let new_title = required_text(&input.new_title, "新项目标题")?;

    if original.source_mode != SOURCE_MODE_FREEFORM && original.project_kind == PROJECT_KIND_TEXT {
        return state
            .orchestrator
            .duplicate_text_project_record(&original, user_id, new_title)
            .await
            .map_err(app_error_message);
    }

    if original.source_mode != SOURCE_MODE_FREEFORM {
        let mut copied_media_files = Vec::new();
        let mut media_file_copier = RuntimeProjectMediaFileCopier {
            data_dir: &state.config.data_dir,
            copied_media_files: &mut copied_media_files,
        };
        return match state
            .orchestrator
            .duplicate_derived_project_record(&original, user_id, new_title, &mut media_file_copier)
            .await
            .map_err(app_error_message)
        {
            Ok(project) => Ok(project),
            Err(error) => Err(crate::file_utils::error_with_rollback_file_cleanup(
                error,
                &copied_media_files,
                "复制的项目媒体文件",
            )),
        };
    }

    let mut copied_media_files = Vec::new();
    let mut media_file_copier = RuntimeProjectMediaFileCopier {
        data_dir: &state.config.data_dir,
        copied_media_files: &mut copied_media_files,
    };
    match state
        .orchestrator
        .duplicate_freeform_project_record(&original, user_id, new_title, &mut media_file_copier)
        .await
        .map_err(app_error_message)
    {
        Ok(project) => Ok(project),
        Err(error) => Err(crate::file_utils::error_with_rollback_file_cleanup(
            error,
            &copied_media_files,
            "复制的项目媒体文件",
        )),
    }
}

pub async fn branch_project(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    branch_title: String,
    at_status: String,
) -> Result<novel_project::Model, String> {
    let branch_title = required_text(&branch_title, "分支标题")?.to_string();
    let branch_status = required_project_status(&at_status, "分支状态")?;
    let original = project_record_by_id(state, project_id).await?;
    access::verify_project_editor_string(state, user_id, original.id).await?;
    ensure_text_project_for_copy(&original, "创建项目分支")?;

    state
        .orchestrator
        .branch_text_project_record(&original, &branch_title, &branch_status)
        .await
        .map_err(app_error_message)
}

async fn project_record_by_id(
    state: &AppState,
    project_id: i64,
) -> Result<novel_project::Model, String> {
    state
        .orchestrator
        .project_by_id(project_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .ok_or_else(|| format!("项目不存在: {project_id}"))
}
