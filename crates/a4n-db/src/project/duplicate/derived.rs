use a4n_core::models::project::{novel_project, project_source_snapshot};
use chrono::{NaiveDateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};
use std::collections::HashMap;

use super::{ProjectDuplicateError, shared::ProjectMediaFileCopier};

mod assembly;
mod generation;
mod media;
mod snapshot;
mod types;
mod workflow;

pub(super) use assembly::{copy_derived_assembly_clips, copy_derived_media_bindings};
pub(super) use generation::{
    copy_derived_generation_spec_media_refs, copy_derived_prompt_artifacts,
};
use types::DuplicatedDerivedSourceSnapshot;

use snapshot::copy_derived_project_graph_record;

pub async fn duplicate_derived_project_record<Copier>(
    db: &DatabaseConnection,
    original: &novel_project::Model,
    user_id: i64,
    new_title: &str,
    media_file_copier: &mut Copier,
) -> Result<novel_project::Model, ProjectDuplicateError>
where
    Copier: ProjectMediaFileCopier,
{
    let txn = db.begin().await?;
    let now = Utc::now().naive_utc();
    let project = copy_derived_project_graph_record(
        &txn,
        original,
        user_id,
        new_title,
        now,
        media_file_copier,
    )
    .await;
    match project {
        Ok(project) => {
            txn.commit().await?;
            Ok(project)
        }
        Err(err) => Err(err),
    }
}
