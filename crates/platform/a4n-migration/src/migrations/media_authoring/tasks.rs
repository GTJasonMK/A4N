use a4n_db_models::models::{
    generation::{generation_task_checkpoint, workflow_run},
    project::novel_project,
    task::task_queue,
};
use a4n_service_protocol::{
    BatchSummary, DraftDisposition, GenerationEvent, GenerationTaskEvent, GenerationTaskSnapshot,
    GenerationTaskStatus, InterruptedEvent, InterruptionReason,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, QueryFilter,
    Set, Statement,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

use super::{ownership::FilmOwnership, record_receipt};

const NOTICE: &str = "媒体创作已迁入派生项目；原始请求与草稿保留，请从工作区已保存的阶段继续。";

pub(super) async fn migrate<C: ConnectionTrait>(
    db: &C,
    owners: &BTreeMap<i64, i64>,
    films: &FilmOwnership,
    now: sea_orm::prelude::DateTime,
) -> Result<(), DbErr> {
    if owners.is_empty() {
        return Ok(());
    }
    let runs = workflow_run::Entity::find()
        .filter(workflow_run::Column::Id.is_in(owners.keys().copied()))
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect::<BTreeMap<_, _>>();
    let projects = novel_project::Entity::find()
        .filter(novel_project::Column::Id.is_in(owners.values().copied()))
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect::<BTreeMap<_, _>>();
    let tasks = task_queue::Entity::find()
        .filter(task_queue::Column::WorkflowRunId.is_in(owners.keys().copied()))
        .all(db)
        .await?;
    for task in tasks {
        let run_id = task
            .workflow_run_id
            .ok_or_else(|| failure("迁移任务的工作流身份丢失"))?;
        let project_id = owners[&run_id];
        let project = &projects[&project_id];
        let root_id = project.root_project_id.unwrap_or(project.id);
        let before = json!({"task_id":task.id, "project_id":task.project_id, "root_project_id":task.root_project_id,
            "source_snapshot_id":task.source_snapshot_id, "status":task.status, "error_message":task.error_message, "completed_at":task.completed_at});
        let checkpoint = generation_task_checkpoint::Entity::find_by_id(task.id.clone())
            .one(db)
            .await?;
        let mut queue_status = task.status.clone();
        if matches!(queue_status.as_str(), "pending" | "running") {
            queue_status = "interrupted".into();
        }
        if let Some(checkpoint) = checkpoint {
            let original_snapshot = checkpoint.public_snapshot_json.clone();
            let mut snapshot = original_snapshot
                .clone()
                .map(serde_json::from_value::<GenerationTaskSnapshot>)
                .transpose()
                .map_err(json_error)?;
            if let Some(snapshot) = &mut snapshot {
                snapshot
                    .validate()
                    .map_err(|error| failure(error.to_string()))?;
                if snapshot.task_id != checkpoint.task_id
                    || snapshot.attempt
                        != u32::try_from(checkpoint.attempt)
                            .map_err(|error| failure(error.to_string()))?
                    || original_snapshot.as_ref().map(canonical_hash).transpose()?
                        != checkpoint.public_snapshot_sha256
                {
                    return Err(failure("历史生成任务的快照身份或哈希不一致"));
                }
                snapshot.started.project_id = Some(project_id);
                snapshot.started.root_project_id = Some(root_id);
                if let Some(latest) = &mut snapshot.latest_checkpoint {
                    latest.resumable = false;
                    latest.summary = Some(NOTICE.into());
                }
                if !snapshot.status.is_terminal() {
                    let sequence = u64::try_from(checkpoint.reserved_through_sequence)
                        .map_err(|error| failure(error.to_string()))?
                        .checked_add(1)
                        .ok_or_else(|| failure("历史任务事件序列溢出"))?;
                    snapshot.status = GenerationTaskStatus::Interrupted;
                    snapshot.covers_through_sequence = sequence;
                    snapshot.updated_at_ms = now.and_utc().timestamp_millis();
                    snapshot.terminal_event = Some(GenerationTaskEvent {
                        schema_version: snapshot.schema_version,
                        task_id: snapshot.task_id.clone(),
                        attempt: snapshot.attempt,
                        sequence,
                        emitted_at_ms: snapshot.updated_at_ms,
                        event: GenerationEvent::Interrupted(InterruptedEvent {
                            reason: InterruptionReason::HostRestartRecovery,
                            retryable: false,
                            draft_disposition: DraftDisposition::RetainedUntilExpiry,
                            summary: Some(NOTICE.into()),
                            batch_summary: batch_summary(db, snapshot).await?,
                        }),
                    });
                }
                if let Some(terminal) = &mut snapshot.terminal_event
                    && let GenerationEvent::Completed(completed) = &mut terminal.event
                    && let Some(result) = &mut completed.result_ref
                    && result.kind == "chapter_shooting_script"
                    && let Ok(old) = result.id.parse::<i64>()
                    && let Some(mapped) = films.script_copies.get(&(old, project_id))
                {
                    result.id = mapped.to_string();
                }
                snapshot
                    .validate()
                    .map_err(|error| failure(error.to_string()))?;
                queue_status = serde_json::to_value(snapshot.status)
                    .map_err(json_error)?
                    .as_str()
                    .ok_or_else(|| failure("任务状态序列化无效"))?
                    .into();
            }
            let after_snapshot = snapshot
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .map_err(json_error)?;
            let mut active: generation_task_checkpoint::ActiveModel = checkpoint.clone().into();
            active.project_id = Set(Some(project_id));
            active.root_project_id = Set(Some(root_id));
            if let Some(snapshot) = &snapshot {
                active.status = Set(queue_status.clone());
                active.covers_through_sequence =
                    Set(i64::try_from(snapshot.covers_through_sequence)
                        .map_err(|error| failure(error.to_string()))?);
                active.reserved_through_sequence = Set(checkpoint.reserved_through_sequence.max(
                    i64::try_from(snapshot.covers_through_sequence)
                        .map_err(|error| failure(error.to_string()))?,
                ));
                active.public_snapshot_sha256 =
                    Set(after_snapshot.as_ref().map(canonical_hash).transpose()?);
                active.public_snapshot_json = Set(after_snapshot.clone());
                if !matches!(
                    checkpoint.status.as_str(),
                    "completed" | "failed" | "cancelled" | "interrupted"
                ) {
                    active.completed_at = Set(Some(now));
                }
                if let Some(GenerationTaskEvent {
                    event: GenerationEvent::Completed(completed),
                    ..
                }) = &snapshot.terminal_event
                    && let Some(result) = &completed.result_ref
                {
                    active.result_id = Set(Some(result.id.clone()));
                }
            }
            active.checkpoint_revision = Set(checkpoint
                .checkpoint_revision
                .checked_add(1)
                .ok_or_else(|| failure("任务 revision 溢出"))?);
            active.updated_at = Set(now);
            active.update(db).await?;
            // Execution checkpoints and provider jobs are retained verbatim for
            // audit and cleanup. Only the public wrapper becomes archival.
            record_receipt(db, &format!("generation_task:{}", task.id), run_id, run_id, project_id,
                &json!({"project_id":checkpoint.project_id, "root_project_id":checkpoint.root_project_id, "status":checkpoint.status,
                    "checkpoint_revision":checkpoint.checkpoint_revision, "public_snapshot_json":original_snapshot,
                    "public_snapshot_sha256":checkpoint.public_snapshot_sha256}), &canonical_hash(&json!({"snapshot":after_snapshot,"project_id":project_id}))?).await?;
        }
        let mut active: task_queue::ActiveModel = task.clone().into();
        active.project_id = Set(Some(project_id));
        active.root_project_id = Set(Some(root_id));
        active.source_snapshot_id = Set(runs[&run_id].source_snapshot_id);
        active.status = Set(queue_status.clone());
        if queue_status == "interrupted" {
            active.error_message = Set(Some(NOTICE.into()));
            active.completed_at = Set(Some(now));
        }
        active.update(db).await?;
        record_receipt(
            db,
            &format!("media_task:{}", task.id),
            run_id,
            run_id,
            project_id,
            &before,
            &canonical_hash(&json!({"project_id":project_id,"status":queue_status}))?,
        )
        .await?;
    }
    Ok(())
}

async fn batch_summary<C: ConnectionTrait>(
    db: &C,
    snapshot: &GenerationTaskSnapshot,
) -> Result<Option<BatchSummary>, DbErr> {
    let Some(policy) = snapshot.started.batch_policy else {
        return Ok(None);
    };
    let rows = db.query_all(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT status,item_key FROM generation_task_checkpoint WHERE parent_task_id=$1 ORDER BY item_key", [snapshot.task_id.clone().into()])).await?;
    let mut summary = BatchSummary {
        policy,
        total: u32::try_from(rows.len()).map_err(|error| failure(error.to_string()))?,
        succeeded: 0,
        failed: 0,
        cancelled: 0,
        skipped: 0,
        failed_item_keys: Vec::new(),
        failed_item_keys_truncated: false,
    };
    if summary.total == 0 {
        return Err(failure("历史批处理任务缺少子项，无法证明完成统计"));
    }
    for row in rows {
        match row.try_get::<String>("", "status")?.as_str() {
            "completed" => summary.succeeded += 1,
            "failed" => {
                summary.failed += 1;
                summary.failed_item_keys.push(
                    row.try_get::<Option<String>>("", "item_key")?
                        .ok_or_else(|| failure("失败的历史子任务缺少 item_key"))?,
                );
            }
            "cancelled" => summary.cancelled += 1,
            _ => summary.skipped += 1,
        }
    }
    summary.failed_item_keys.sort();
    Ok(Some(summary))
}

fn canonical_hash(value: &Value) -> Result<String, DbErr> {
    a4n_domain::generation::canonical_json_sha256(value).map_err(json_error)
}
fn failure(error: impl Into<String>) -> DbErr {
    DbErr::Custom(error.into())
}
fn json_error(error: serde_json::Error) -> DbErr {
    failure(error.to_string())
}
