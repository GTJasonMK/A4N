use super::{ChapterVisualPlanDbError, ParsedChapterVisualPlan, SaveChapterVisualPlanDraftInput};
use a4n_domain::chapter_visual_plan::{
    ChapterVisualPlanV3, VisualPlanChange, VisualPlanEditReceipt, VisualPlanEditVersion,
};
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};

pub async fn save_chapter_visual_plan_draft(
    db: &sea_orm::DatabaseConnection,
    input: SaveChapterVisualPlanDraftInput,
) -> Result<ParsedChapterVisualPlan, ChapterVisualPlanDbError> {
    save_chapter_visual_plan_draft_with_operation(db, input, &uuid::Uuid::new_v4().to_string())
        .await
}

/// The journal and the business record commit together. A lost HTTP reply can
/// be retried with the same operation ID without repeating the mutation.
pub async fn save_chapter_visual_plan_draft_with_operation(
    db: &sea_orm::DatabaseConnection,
    input: SaveChapterVisualPlanDraftInput,
    operation_id: &str,
) -> Result<ParsedChapterVisualPlan, ChapterVisualPlanDbError> {
    let payload_hash = a4n_domain::generation::canonical_json_sha256(&(
        input.author_user_id,
        input.source.requested_project_id,
        input.source.source_chapter_id,
        &input.source.source_revision_key,
        input.authoring_mode,
        input.expected,
        &input.expected_source_hash,
        &input.plan,
    ))
    .map_err(|error| ChapterVisualPlanDbError::InvalidArgument(error.to_string()))?;
    save_chapter_visual_plan_edit(db, input, operation_id, &payload_hash).await
}

pub async fn replay_visual_plan_edit<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    operation_id: &str,
    payload_hash: &str,
) -> Result<
    Option<(Option<ParsedChapterVisualPlan>, ParsedChapterVisualPlan)>,
    ChapterVisualPlanDbError,
> {
    let Some(row) = db.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT id, payload_hash, before_record, after_record FROM visual_plan_edit_receipt WHERE project_id = $1 AND operation_id = $2",
        [project_id.into(), operation_id.into()])).await? else { return Ok(None); };
    let stored_hash: String = row.try_get("", "payload_hash")?;
    if stored_hash != payload_hash {
        return Err(ChapterVisualPlanDbError::InvalidArgument(
            "同一 operation_id 不能用于不同编辑请求".into(),
        ));
    }
    let before: serde_json::Value = row.try_get("", "before_record")?;
    let after: serde_json::Value = row.try_get("", "after_record")?;
    let before: Option<ParsedChapterVisualPlan> = serde_json::from_value(before)
        .map_err(|error| ChapterVisualPlanDbError::InvalidState(error.to_string()))?;
    let mut after: ParsedChapterVisualPlan = serde_json::from_value(after)
        .map_err(|error| ChapterVisualPlanDbError::InvalidState(error.to_string()))?;
    after.edit_receipt = Some(receipt(
        row.try_get("", "id")?,
        operation_id,
        before.as_ref(),
        &after,
    ));
    Ok(Some((before, after)))
}

pub async fn save_chapter_visual_plan_edit(
    db: &sea_orm::DatabaseConnection,
    mut input: SaveChapterVisualPlanDraftInput,
    operation_id: &str,
    payload_hash: &str,
) -> Result<ParsedChapterVisualPlan, ChapterVisualPlanDbError> {
    if operation_id.is_empty()
        || operation_id.len() > 128
        || operation_id.chars().any(char::is_control)
    {
        return Err(ChapterVisualPlanDbError::InvalidArgument(
            "operation_id 无效".into(),
        ));
    }
    let project_id = input.source.requested_project_id;
    let tx = db.begin().await?;
    // Same project lock orders receipt cursors by commit and also protects draft creation.
    // ponytail: per-project serialization; split by document if contention is measured.
    lock_project(&tx, project_id).await?;
    if let Some((_, saved)) =
        replay_visual_plan_edit(&tx, project_id, operation_id, payload_hash).await?
    {
        tx.commit().await?;
        return Ok(saved);
    }
    let before = match &input.expected {
        Some(expected) => Some(super::chapter_visual_plan_by_id(&tx, expected.record_id).await?),
        None => {
            super::chapter_visual_plans_for_source(&tx, &input.source)
                .await?
                .confirmed
        }
    };
    if let (Some(expected), Some(current)) = (&mut input.expected, &before)
        && expected.revision != current.record.revision
    {
        let base = visual_plan_revision(&tx, current.record.id, expected.revision).await?;
        if base.record.source_revision_key != current.record.source_revision_key
            || base.status != current.status
        {
            return Err(ChapterVisualPlanDbError::InvalidState(
                "编排来源或状态已变化，请重新规划".into(),
            ));
        }
        input.plan = current
            .plan
            .merge_edit(&base.plan, &input.plan)
            .map_err(|error| ChapterVisualPlanDbError::InvalidState(error.to_string()))?;
        expected.revision = current.record.revision;
    }
    let mut saved = super::mutations::save_chapter_visual_plan_draft_in(&tx, input).await?;
    record_edit(&tx, before.as_ref(), &mut saved, operation_id, payload_hash).await?;
    tx.commit().await?;
    Ok(saved)
}

pub(super) async fn lock_project<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
) -> Result<(), ChapterVisualPlanDbError> {
    db.query_one(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT id FROM novel_project WHERE id = $1 FOR UPDATE",
        [project_id.into()],
    ))
    .await?
    .ok_or_else(|| ChapterVisualPlanDbError::NotFound("视觉方案项目不存在".into()))?;
    Ok(())
}

pub async fn record_visual_plan_noop(
    db: &sea_orm::DatabaseConnection,
    base: &ParsedChapterVisualPlan,
    operation_id: &str,
    payload_hash: &str,
) -> Result<ParsedChapterVisualPlan, ChapterVisualPlanDbError> {
    if operation_id.is_empty()
        || operation_id.len() > 128
        || operation_id.chars().any(char::is_control)
    {
        return Err(ChapterVisualPlanDbError::InvalidArgument(
            "operation_id 无效".into(),
        ));
    }
    let tx = db.begin().await?;
    lock_project(&tx, base.record.project_id).await?;
    if let Some((_, saved)) =
        replay_visual_plan_edit(&tx, base.record.project_id, operation_id, payload_hash).await?
    {
        tx.commit().await?;
        return Ok(saved);
    }
    let current = super::chapter_visual_plan_by_id(&tx, base.record.id).await?;
    super::queries::ensure_current_plan_source(&tx, &current).await?;
    if current.status != base.status
        || current.record.source_revision_key != base.record.source_revision_key
    {
        return Err(ChapterVisualPlanDbError::InvalidState(
            "编排来源或状态已变化，请重新读取".into(),
        ));
    }
    let mut saved = current.clone();
    record_edit(&tx, Some(&current), &mut saved, operation_id, payload_hash).await?;
    tx.commit().await?;
    Ok(saved)
}

pub(super) async fn record_edit<C: ConnectionTrait>(
    db: &C,
    before: Option<&ParsedChapterVisualPlan>,
    saved: &mut ParsedChapterVisualPlan,
    operation_id: &str,
    payload_hash: &str,
) -> Result<(), ChapterVisualPlanDbError> {
    let changes = edit_changes(before, saved);
    let before_json = serde_json::to_value(before)
        .map_err(|error| ChapterVisualPlanDbError::InvalidState(error.to_string()))?;
    let after_json = serde_json::to_value(&*saved)
        .map_err(|error| ChapterVisualPlanDbError::InvalidState(error.to_string()))?;
    let changes_json = serde_json::to_value(&changes)
        .map_err(|error| ChapterVisualPlanDbError::InvalidState(error.to_string()))?;
    let row = db.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO visual_plan_edit_receipt (project_id, source_chapter_id, operation_id, payload_hash, record_id, before_record, after_record, changes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
        [saved.record.project_id.into(), saved.record.source_chapter_id.into(), operation_id.into(), payload_hash.into(), saved.record.id.into(), before_json.into(), after_json.into(), changes_json.into()])).await?
        .ok_or_else(|| ChapterVisualPlanDbError::InvalidState("编辑回执未写入".into()))?;
    saved.edit_receipt = Some(receipt(row.try_get("", "id")?, operation_id, before, saved));
    Ok(())
}

pub async fn visual_plan_revision<C: ConnectionTrait>(
    db: &C,
    record_id: i64,
    revision: i32,
) -> Result<ParsedChapterVisualPlan, ChapterVisualPlanDbError> {
    let current = super::chapter_visual_plan_by_id(db, record_id).await?;
    if current.record.revision == revision {
        return Ok(current);
    }
    let row = db.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT snapshot FROM (
            SELECT id, after_record AS snapshot FROM visual_plan_edit_receipt WHERE record_id = $1 AND (after_record->'record'->>'revision')::integer = $2
            UNION ALL
            SELECT id, before_record AS snapshot FROM visual_plan_edit_receipt WHERE record_id = $1 AND (before_record->'record'->>'revision')::integer = $2
         ) versions ORDER BY id DESC LIMIT 1", [record_id.into(), revision.into()])).await?
        .ok_or(ChapterVisualPlanDbError::RevisionConflict { record_id, expected_revision: revision })?;
    let value: serde_json::Value = row.try_get("", "snapshot")?;
    serde_json::from_value(value)
        .map_err(|error| ChapterVisualPlanDbError::InvalidState(error.to_string()))
}

fn receipt(
    sequence: i64,
    operation_id: &str,
    before: Option<&ParsedChapterVisualPlan>,
    after: &ParsedChapterVisualPlan,
) -> VisualPlanEditReceipt {
    let version = |plan: &ParsedChapterVisualPlan| VisualPlanEditVersion {
        record_id: plan.record.id,
        revision: plan.record.revision,
        plan_sha256: plan.record.plan_sha256.clone(),
    };
    VisualPlanEditReceipt {
        sequence,
        operation_id: operation_id.into(),
        project_id: after.record.project_id,
        source_chapter_id: after.record.source_chapter_id,
        source_revision_key: after.record.source_revision_key.clone(),
        author_user_id: after.record.author_user_id,
        before: before.map(version),
        after: version(after),
        changes: edit_changes(before, after),
    }
}

fn edit_changes(
    before: Option<&ParsedChapterVisualPlan>,
    after: &ParsedChapterVisualPlan,
) -> Vec<VisualPlanChange> {
    match before {
        Some(before) => before.plan.changes_to(&after.plan),
        None => ChapterVisualPlanV3::starter().changes_to(&after.plan),
    }
}

pub async fn visual_plan_edits<C: ConnectionTrait>(
    db: &C,
    source: &a4n_domain::chapter::ResolvedChapterSource,
    after_sequence: i64,
) -> Result<Vec<ParsedChapterVisualPlan>, ChapterVisualPlanDbError> {
    if after_sequence < 0 {
        return Err(ChapterVisualPlanDbError::InvalidArgument(
            "after_sequence 不能为负数".into(),
        ));
    }
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id, operation_id, before_record, after_record FROM visual_plan_edit_receipt
         WHERE project_id = $1 AND source_chapter_id = $2 AND id > $3
           AND after_record->'record'->>'source_revision_key' = $4 ORDER BY id LIMIT 100",
            [
                source.requested_project_id.into(),
                source.source_chapter_id.into(),
                after_sequence.into(),
                source.source_revision_key.clone().into(),
            ],
        ))
        .await?;
    rows.into_iter()
        .map(|row| {
            let before: serde_json::Value = row.try_get("", "before_record")?;
            let after: serde_json::Value = row.try_get("", "after_record")?;
            let before: Option<ParsedChapterVisualPlan> = serde_json::from_value(before)
                .map_err(|error| ChapterVisualPlanDbError::InvalidState(error.to_string()))?;
            let mut after: ParsedChapterVisualPlan = serde_json::from_value(after)
                .map_err(|error| ChapterVisualPlanDbError::InvalidState(error.to_string()))?;
            after.edit_receipt = Some(receipt(
                row.try_get("", "id")?,
                &row.try_get::<String>("", "operation_id")?,
                before.as_ref(),
                &after,
            ));
            Ok(after)
        })
        .collect()
}

/// Pin the replay cursor to the returned snapshot, never to a newer commit
/// racing its read. This avoids replaying the project's entire editing history.
pub async fn visual_plan_edit_cursor<C: ConnectionTrait>(
    db: &C,
    plan: Option<&ParsedChapterVisualPlan>,
) -> Result<i64, ChapterVisualPlanDbError> {
    let Some(plan) = plan else {
        return Ok(0);
    };
    let row = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT COALESCE(MAX(id), 0) AS cursor FROM visual_plan_edit_receipt
         WHERE record_id = $1 AND (after_record->'record'->>'revision')::integer <= $2",
            [plan.record.id.into(), plan.record.revision.into()],
        ))
        .await?
        .ok_or_else(|| ChapterVisualPlanDbError::InvalidState("无法读取编辑游标".into()))?;
    Ok(row.try_get("", "cursor")?)
}
