use a4n_core::error::AppError;
use a4n_core::models::galgame::{galgame_line, galgame_scene};
use a4n_db::project::{ProjectFamilyContext, ProjectFamilyDbError};

use super::{
    OrchestratorContext, PROJECT_KIND_GALGAME, galgame_context_record_error,
    galgame_project_record_error, validate_positive_id,
};

impl OrchestratorContext {
    pub(in crate::galgame) async fn load_galgame_context(
        &self,
        project_id: i64,
    ) -> Result<ProjectFamilyContext, AppError> {
        validate_positive_id(project_id, "project_id")?;
        let context =
            a4n_db::project::derived_project_family_context(self.database.connection(), project_id)
                .await
                .map_err(project_family_app_error)?;
        if context.project.project_kind != PROJECT_KIND_GALGAME {
            return Err(AppError::InvalidArgument(format!(
                "项目不是 Galgame 派生项目: {project_id}"
            )));
        }
        Ok(context)
    }

    pub(in crate::galgame) async fn load_scene_in_project(
        &self,
        project_id: i64,
        scene_id: i64,
    ) -> Result<galgame_scene::Model, AppError> {
        validate_positive_id(project_id, "project_id")?;
        validate_positive_id(scene_id, "scene_id")?;
        a4n_db::galgame::galgame_scene_in_project_record(
            self.database.connection(),
            project_id,
            scene_id,
        )
        .await
        .map_err(galgame_project_record_error)
    }

    pub(in crate::galgame) async fn load_line_in_project(
        &self,
        project_id: i64,
        line_id: i64,
    ) -> Result<galgame_line::Model, AppError> {
        validate_positive_id(project_id, "project_id")?;
        validate_positive_id(line_id, "line_id")?;
        a4n_db::galgame::galgame_line_in_project_record(
            self.database.connection(),
            project_id,
            line_id,
        )
        .await
        .map_err(galgame_project_record_error)
    }

    pub(in crate::galgame) async fn validate_speaker_profile(
        &self,
        context: &ProjectFamilyContext,
        speaker_profile_id: i64,
    ) -> Result<(), AppError> {
        validate_positive_id(speaker_profile_id, "speaker_profile_id")?;
        a4n_db::galgame::galgame_speaker_profile_in_root_project_record(
            self.database.connection(),
            context.root_project.id,
            speaker_profile_id,
        )
        .await
        .map(|_| ())
        .map_err(galgame_context_record_error)
    }

    pub(in crate::galgame) async fn validate_text_anchor(
        &self,
        context: &ProjectFamilyContext,
        text_anchor_id: i64,
        expected_source_snapshot_chapter_id: Option<i64>,
    ) -> Result<(), AppError> {
        validate_positive_id(text_anchor_id, "text_anchor_id")?;
        a4n_db::galgame::galgame_text_anchor_in_source_snapshot_record(
            self.database.connection(),
            context.root_project.id,
            context.source_snapshot.id,
            text_anchor_id,
            expected_source_snapshot_chapter_id,
        )
        .await
        .map(|_| ())
        .map_err(galgame_context_record_error)
    }
}

fn project_family_app_error(error: ProjectFamilyDbError) -> AppError {
    match error {
        ProjectFamilyDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ProjectFamilyDbError::NotFound(message) => AppError::NotFound(message),
        ProjectFamilyDbError::Database(error) => AppError::Database(error),
    }
}
