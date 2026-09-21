use a4n_api_contract::project::DuplicateProjectInput;
use a4n_db::DbHandle;
use a4n_db_models::models::project::novel_project;
use a4n_domain::project::{
    ProjectMediaCopyError, ProjectMediaFileCopier, ProjectSourceMode, required_text,
};
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::project::project_duplicate_error_to_app_error;
use a4n_api_contract::project::{ProjectDuplicationApplicationPort, ProjectDuplicationHostPort};

struct ProjectDuplicationHostPortRef<'a> {
    inner: &'a mut (dyn ProjectDuplicationHostPort + Sync + 'a),
}

impl ProjectMediaFileCopier for ProjectDuplicationHostPortRef<'_> {
    fn copy_media_file(
        &mut self,
        new_project_id: i64,
        source_media_asset_id: i64,
        field: &str,
        relative_path: Option<&str>,
    ) -> Result<Option<String>, ProjectMediaCopyError> {
        self.inner
            .copy_media_file(new_project_id, source_media_asset_id, field, relative_path)
    }
}

#[async_trait::async_trait]
impl ProjectDuplicationHostPort for ProjectDuplicationHostPortRef<'_> {
    async fn verify_project_editor(&self, user_id: i64, project_id: i64) -> Result<(), AppError> {
        self.inner.verify_project_editor(user_id, project_id).await
    }

    async fn rollback_copied_media_files(&mut self, label: &str) -> Result<(), String> {
        self.inner.rollback_copied_media_files(label).await
    }
}

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
    pub(crate) async fn duplicate_project_validated<Host>(
        &self,
        user_id: i64,
        input: DuplicateProjectInput,
        host_port: &mut Host,
    ) -> Result<novel_project::Model, AppError>
    where
        Host: ProjectDuplicationHostPort,
    {
        let original = self.project_for_duplication(input.project_id).await?;
        host_port
            .verify_project_editor(user_id, original.id)
            .await?;
        let new_title =
            required_text(&input.new_title, "新项目标题").map_err(AppError::InvalidArgument)?;

        match original
            .source_mode()
            .map_err(|error| AppError::InvalidArgument(format!("项目 {}: {error}", original.id)))?
        {
            ProjectSourceMode::Self_ => {
                self.project_duplication
                    .duplicate_text(&original, user_id, new_title)
                    .await
            }
            ProjectSourceMode::Derived => {
                let result = self
                    .project_duplication
                    .duplicate_derived(&original, user_id, new_title, host_port)
                    .await;
                duplication_result_with_file_rollback(result, host_port).await
            }
            ProjectSourceMode::Freeform => {
                let result = self
                    .project_duplication
                    .duplicate_freeform(&original, user_id, new_title, host_port)
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

#[async_trait::async_trait]
impl ProjectDuplicationApplicationPort for OrchestratorContext {
    async fn duplicate_project(
        &self,
        user_id: i64,
        input: DuplicateProjectInput,
        host_port: &mut dyn ProjectDuplicationHostPort,
    ) -> Result<a4n_api_contract::project::ProjectSummary, AppError> {
        let mut host_port_ref = ProjectDuplicationHostPortRef { inner: host_port };
        let project = self
            .duplicate_project_validated(user_id, input, &mut host_port_ref)
            .await?;
        self.project_summary_for_user(user_id, project.id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("复制后的项目不存在: {}", project.id)))
    }
}

async fn duplication_result_with_file_rollback<T, Host>(
    result: Result<T, AppError>,
    host_port: &mut Host,
) -> Result<T, AppError>
where
    Host: ProjectDuplicationHostPort,
{
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
