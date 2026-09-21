use a4n_db_models::models::project::novel_project;
use a4n_domain::shooting_script::EditStoredChapterShootingScriptInput;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, TransactionTrait,
};

use super::{
    codec::stored_visual_plan_identity,
    contracts::{
        EditChapterShootingScriptOutcome, ShootingScriptDbError, UpdateChapterShootingScriptInput,
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
    let visual_plan = stored_visual_plan_identity(&current.record)?.ok_or_else(|| {
        ShootingScriptDbError::InvalidState(
            "历史拍摄剧本未绑定视觉方案，不能继续编辑；请重新生成绑定当前视觉方案的拍摄剧本"
                .to_string(),
        )
    })?;
    let shooting_script = update_chapter_shooting_script(
        &tx,
        UpdateChapterShootingScriptInput {
            record_id: input.record_id,
            expected_revision: input.expected_revision,
            input_fingerprint_sha256: current.record.input_fingerprint_sha256.clone(),
            visual_plan,
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
    let invalidated_project_ids = invalidate_downstream_projects(
        &tx,
        current.record.root_project_id,
        shooting_script.record.id,
        shooting_script.record.revision,
        "edit",
    )
    .await?;
    tx.commit().await?;
    Ok(EditChapterShootingScriptOutcome {
        shooting_script,
        invalidated_project_ids,
    })
}

pub(super) async fn invalidate_downstream_projects<C: ConnectionTrait>(
    db: &C,
    root_project_id: i64,
    record_id: i64,
    revision: i32,
    operation: &str,
) -> Result<Vec<i64>, ShootingScriptDbError> {
    let invalidated_project_ids = novel_project::Entity::find()
        .select_only()
        .column(novel_project::Column::Id)
        .filter(novel_project::Column::RootProjectId.eq(root_project_id))
        .filter(novel_project::Column::ProjectKind.is_in(["comic", "video", "galgame"]))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    crate::cleanup_outbox::insert_media_workflow_v2_reset_intents(
        db,
        invalidated_project_ids.iter().map(|project_id| {
            (
                format!(
                    "shooting-script-{operation}:{record_id}:revision:{revision}:project:{project_id}"
                ),
                crate::cleanup_outbox::MediaWorkflowV2ResetPayload::new(*project_id),
            )
        }),
    )
    .await?;
    Ok(invalidated_project_ids)
}
