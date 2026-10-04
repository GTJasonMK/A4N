use a4n_domain::project::{ProjectMediaFileCopier, ProjectRecord};
use chrono::Utc;
use sea_orm::{DatabaseConnection, TransactionTrait};

use super::ProjectDuplicateError;
use crate::project::{project_record_from_model, project_record_into_model};

mod assembly;
mod generation;

pub(super) use assembly::{copy_derived_assembly_clips, copy_derived_media_bindings};
pub(super) use generation::{
    copy_derived_generation_spec_media_refs, copy_derived_prompt_artifacts,
};

pub async fn duplicate_derived_project_record<Copier>(
    db: &DatabaseConnection,
    original: ProjectRecord,
    user_id: i64,
    new_title: &str,
    media_file_copier: &mut Copier,
) -> Result<ProjectRecord, ProjectDuplicateError>
where
    Copier: ProjectMediaFileCopier + ?Sized,
{
    let txn = db.begin().await?;
    let now = Utc::now().naive_utc();
    if original.project_kind == crate::project::PROJECT_KIND_TEXT
        || original.root_project_id.is_none()
        || original.source_snapshot_id.is_none()
    {
        return Err(ProjectDuplicateError::InvalidArgument(
            "复制派生项目需要原作和冻结原文".into(),
        ));
    }
    let original = project_record_into_model(original);
    let project = super::shared::insert_project_copy_record(
        &txn,
        &original,
        super::shared::ProjectCopyInsert {
            user_id,
            title: new_title.into(),
            project_kind: original.project_kind.clone(),
            root_project_id: original.root_project_id,
            source_snapshot_id: original.source_snapshot_id,
            status: original.status.clone(),
            current_chapter: original.current_chapter,
            total_chapters: original.total_chapters,
            raw_blueprint: None,
        },
        now,
    )
    .await?;
    crate::project::family::import::copy_derived_media_rows(
        &txn,
        &original,
        &project,
        media_file_copier,
    )
    .await
    .map_err(|error| match error {
        crate::project::family::import::ProjectFamilyImportError::Database(error) => {
            ProjectDuplicateError::Database(error)
        }
        error => ProjectDuplicateError::InvalidState {
            current: error.to_string(),
            operation: "复制派生项目".into(),
        },
    })?;
    txn.commit().await?;
    Ok(project_record_from_model(project))
}
