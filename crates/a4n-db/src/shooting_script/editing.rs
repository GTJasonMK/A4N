use a4n_core::models::project::novel_project;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};

use super::{
    contracts::{
        EditChapterShootingScriptOutcome, EditStoredChapterShootingScriptInput,
        ShootingScriptDbError, UpdateChapterShootingScriptInput,
    },
    mutations::update_chapter_shooting_script,
    queries::chapter_shooting_script_by_id,
};

pub async fn edit_chapter_shooting_script_and_invalidate(
    db: &sea_orm::DatabaseConnection,
    input: EditStoredChapterShootingScriptInput,
) -> Result<EditChapterShootingScriptOutcome, ShootingScriptDbError> {
    let tx = db.begin().await?;
    let current = chapter_shooting_script_by_id(&tx, input.record_id).await?;
    if current.record.revision != input.expected_revision {
        return Err(ShootingScriptDbError::RevisionConflict {
            record_id: input.record_id,
            expected_revision: input.expected_revision,
        });
    }
    let shooting_script = update_chapter_shooting_script(
        &tx,
        UpdateChapterShootingScriptInput {
            record_id: input.record_id,
            expected_revision: input.expected_revision,
            input_fingerprint_sha256: current.record.input_fingerprint_sha256.clone(),
            script: input.script,
            workflow_run_id: None,
        },
    )
    .await?;
    if shooting_script.record.revision == current.record.revision {
        tx.commit().await?;
        return Ok(EditChapterShootingScriptOutcome {
            shooting_script,
            invalidated_project_ids: Vec::new(),
        });
    }
    let invalidated_project_ids = novel_project::Entity::find()
        .filter(novel_project::Column::RootProjectId.eq(current.record.root_project_id))
        .filter(novel_project::Column::ProjectKind.is_in(["comic", "video", "galgame"]))
        .all(&tx)
        .await?
        .into_iter()
        .map(|project| project.id)
        .collect::<Vec<_>>();
    for project_id in &invalidated_project_ids {
        crate::cleanup_outbox::insert_media_workflow_v2_reset_intent(
            &tx,
            format!(
                "shooting-script-edit:{}:revision:{}:project:{}",
                shooting_script.record.id, shooting_script.record.revision, project_id
            ),
            crate::cleanup_outbox::MediaWorkflowV2ResetPayload::new(*project_id),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(EditChapterShootingScriptOutcome {
        shooting_script,
        invalidated_project_ids,
    })
}
