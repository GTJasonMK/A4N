use a4n_core::{error::AppError, models::project::novel_project};
use a4n_db::DbHandle;
use a4n_domain::project::ProjectSourceMode;

use super::OrchestratorContext;
use crate::project::{
    DuplicateProjectApplicationInput, ProjectDuplicationHostPort, ProjectMediaFileCopier,
    project_duplicate_error_to_app_error, required_text,
};

#[derive(Clone)]
pub(super) struct ProjectDuplicationApplication {
    database: DbHandle,
}

impl ProjectDuplicationApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn duplicate_text(
        &self,
        original: &novel_project::Model,
        user_id: i64,
        new_title: &str,
    ) -> Result<novel_project::Model, AppError> {
        a4n_db::project::duplicate_text_project_record(
            self.database.connection(),
            original,
            user_id,
            new_title,
        )
        .await
        .map_err(project_duplicate_error_to_app_error)
    }

    async fn duplicate_derived<Copier>(
        &self,
        original: &novel_project::Model,
        user_id: i64,
        new_title: &str,
        media_file_copier: &mut Copier,
    ) -> Result<novel_project::Model, AppError>
    where
        Copier: ProjectMediaFileCopier,
    {
        a4n_db::project::duplicate_derived_project_record(
            self.database.connection(),
            original,
            user_id,
            new_title,
            media_file_copier,
        )
        .await
        .map_err(project_duplicate_error_to_app_error)
    }

    async fn duplicate_freeform<Copier>(
        &self,
        original: &novel_project::Model,
        user_id: i64,
        new_title: &str,
        media_file_copier: &mut Copier,
    ) -> Result<novel_project::Model, AppError>
    where
        Copier: ProjectMediaFileCopier,
    {
        a4n_db::project::duplicate_freeform_project_record(
            self.database.connection(),
            original,
            user_id,
            new_title,
            media_file_copier,
        )
        .await
        .map_err(project_duplicate_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub async fn duplicate_project_validated<Host>(
        &self,
        input: DuplicateProjectApplicationInput,
        host_port: &mut Host,
    ) -> Result<novel_project::Model, AppError>
    where
        Host: ProjectDuplicationHostPort,
    {
        let original = self.project_for_duplication(input.project_id).await?;
        host_port
            .verify_project_editor(input.user_id, original.id)
            .await?;
        let new_title =
            required_text(&input.new_title, "新项目标题").map_err(AppError::InvalidArgument)?;

        match original
            .source_mode()
            .map_err(|error| AppError::InvalidArgument(format!("项目 {}: {error}", original.id)))?
        {
            ProjectSourceMode::Self_ => {
                self.project_duplication
                    .duplicate_text(&original, input.user_id, new_title)
                    .await
            }
            ProjectSourceMode::Derived => {
                let result = self
                    .project_duplication
                    .duplicate_derived(&original, input.user_id, new_title, host_port)
                    .await;
                duplication_result_with_file_rollback(result, host_port).await
            }
            ProjectSourceMode::Freeform => {
                let result = self
                    .project_duplication
                    .duplicate_freeform(&original, input.user_id, new_title, host_port)
                    .await;
                duplication_result_with_file_rollback(result, host_port).await
            }
        }
    }

    async fn project_for_duplication(
        &self,
        project_id: i64,
    ) -> Result<novel_project::Model, AppError> {
        self.project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))
    }
}

async fn duplication_result_with_file_rollback<T>(
    result: Result<T, AppError>,
    host_port: &mut impl ProjectDuplicationHostPort,
) -> Result<T, AppError> {
    match result {
        Ok(value) => Ok(value),
        Err(primary) => match host_port
            .rollback_copied_media_files("复制的项目媒体文件")
            .await
        {
            Ok(()) => Err(primary),
            Err(cleanup_error) => Err(AppError::Internal(anyhow::anyhow!(
                "{primary}；{cleanup_error}"
            ))),
        },
    }
}
