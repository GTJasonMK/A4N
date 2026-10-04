use super::*;

#[cfg(test)]
pub(super) fn generation_public_snapshot_for_summary(
    checkpoint: &generation_task_checkpoint::Model,
) -> GenerationPublicSnapshot {
    match super::verified_generation_public_snapshot(checkpoint) {
        Ok(Some(snapshot)) => GenerationPublicSnapshot::Available(snapshot),
        Ok(None) => GenerationPublicSnapshot::Expired,
        Err(_) => GenerationPublicSnapshot::Corrupt,
    }
}

pub fn generation_public_snapshot_for_summary_record(
    checkpoint: &GenerationTaskCheckpointSummaryRecord,
) -> GenerationPublicSnapshot {
    match super::verified_generation_public_snapshot(&super::generation_task_summary_model(
        checkpoint,
    )) {
        Ok(Some(snapshot)) => GenerationPublicSnapshot::Available(snapshot),
        Ok(None) => GenerationPublicSnapshot::Expired,
        Err(_) => GenerationPublicSnapshot::Corrupt,
    }
}
pub async fn list_active_generation_task_checkpoints(
    db: &DatabaseConnection,
    requested_by_user_id: i64,
    project_id: Option<i64>,
) -> Result<GenerationTaskCheckpointPage, GenerationTaskPersistenceError> {
    list_generation_task_checkpoints(
        db,
        requested_by_user_id,
        project_id,
        &ACTIVE_GENERATION_STATUSES,
        None,
        MAX_ACTIVE_GENERATION_TASKS,
        false,
    )
    .await
}

pub async fn list_generation_task_history_checkpoints(
    db: &DatabaseConnection,
    requested_by_user_id: i64,
    project_id: Option<i64>,
    limit: u32,
    cursor: Option<GenerationTaskListCursor>,
) -> Result<GenerationTaskCheckpointPage, GenerationTaskPersistenceError> {
    if !(1..=MAX_GENERATION_HISTORY_LIMIT).contains(&limit) {
        return Err(GenerationTaskPersistenceError::InvalidArgument(format!(
            "history limit 必须在 1..={MAX_GENERATION_HISTORY_LIMIT}"
        )));
    }
    list_generation_task_checkpoints(
        db,
        requested_by_user_id,
        project_id,
        &HISTORY_GENERATION_STATUSES,
        cursor,
        limit,
        true,
    )
    .await
}

async fn list_generation_task_checkpoints(
    db: &DatabaseConnection,
    requested_by_user_id: i64,
    project_id: Option<i64>,
    statuses: &[&str],
    cursor: Option<GenerationTaskListCursor>,
    limit: u32,
    exclude_expired: bool,
) -> Result<GenerationTaskCheckpointPage, GenerationTaskPersistenceError> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "generation task list 仅支持 PostgreSQL".to_string(),
        ));
    }
    validate_safe_id(requested_by_user_id, "requested_by_user_id")?;
    if let Some(project_id) = project_id {
        validate_safe_id(project_id, "project_id")?;
    }
    if let Some(cursor) = &cursor {
        validate_safe_cursor(cursor)?;
    }

    let mut sql = format!(
        "SELECT checkpoint.task_id,\n                checkpoint.attempt,\n                checkpoint.requested_by_user_id,\n                checkpoint.task_kind,\n                checkpoint.delivery_mode,\n                checkpoint.batch_policy,\n                checkpoint.project_id,\n                checkpoint.root_project_id,\n                checkpoint.workflow_run_id,\n                checkpoint.generation_run_id,\n                checkpoint.target_kind,\n                checkpoint.target_id,\n                checkpoint.target_revision,\n                checkpoint.root_task_id,\n                checkpoint.parent_task_id,\n                checkpoint.item_key,\n                checkpoint.retry_of_task_id,\n                checkpoint.status,\n                checkpoint.covers_through_sequence,\n                checkpoint.public_snapshot_json,\n                checkpoint.public_snapshot_sha256,\n                checkpoint.result_kind,\n                checkpoint.result_id,\n                checkpoint.result_revision,\n                checkpoint.expires_at,\n                checkpoint.updated_at\n         FROM generation_task_checkpoint AS checkpoint\n         WHERE checkpoint.requested_by_user_id = $1\n           AND checkpoint.status IN ({})\n           AND EXISTS (\n             SELECT 1\n             FROM \"user\" AS viewer\n             WHERE viewer.id = $1\n               AND viewer.is_active = TRUE\n           )\n           AND (\n             (checkpoint.project_id IS NULL AND checkpoint.root_project_id IS NULL)\n             OR EXISTS (\n                 SELECT 1\n                 FROM novel_project AS accessible_project\n                 WHERE accessible_project.id = COALESCE(checkpoint.project_id, checkpoint.root_project_id)\n                   AND (\n                     accessible_project.user_id = $1\n                     OR accessible_project.id IN (\n                         SELECT member.project_id\n                         FROM project_member AS member\n                         WHERE member.user_id = $1\n                     )\n                     OR accessible_project.root_project_id IN (\n                         SELECT member.project_id\n                         FROM project_member AS member\n                         WHERE member.user_id = $1\n                     )\n                   )\n             )\n           )",
        statuses
            .iter()
            .enumerate()
            .map(|(index, _)| format!("${}", index + 2))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut values = Vec::with_capacity(statuses.len() + 5);
    values.push(requested_by_user_id.into());
    for status in statuses {
        values.push((*status).into());
    }
    let mut next_placeholder = statuses.len() + 2;
    if let Some(project_id) = project_id {
        sql.push_str(&format!(
            "\n           AND (checkpoint.project_id = ${next_placeholder} OR checkpoint.root_project_id = ${next_placeholder})"
        ));
        values.push(project_id.into());
        next_placeholder += 1;
    }
    if exclude_expired {
        sql.push_str(
            "\n           AND (checkpoint.expires_at IS NULL OR checkpoint.expires_at > NOW())",
        );
    }
    if let Some(cursor) = cursor {
        let ms_placeholder = next_placeholder;
        let task_placeholder = next_placeholder + 1;
        sql.push_str(&format!(
            "\n           AND (({GENERATION_UPDATED_AT_MS_SQL} < ${ms_placeholder})\n                OR ({GENERATION_UPDATED_AT_MS_SQL} = ${ms_placeholder} AND checkpoint.task_id > ${task_placeholder}))"
        ));
        values.push(cursor.updated_at_ms.into());
        values.push(cursor.task_id.into());
        next_placeholder += 2;
    }
    let fetch_limit = i64::from(limit) + if exclude_expired { 1 } else { 0 };
    sql.push_str(&format!(
        "\n         ORDER BY {GENERATION_UPDATED_AT_MS_SQL} DESC, checkpoint.task_id ASC\n         LIMIT ${next_placeholder}"
    ));
    values.push(fetch_limit.into());

    let mut rows = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            values,
        ))
        .await?;
    let page_limit = usize::try_from(limit).map_err(|_| {
        GenerationTaskPersistenceError::InvalidArgument(
            "generation task list limit 超出当前平台 usize 范围".to_string(),
        )
    })?;
    let has_more = exclude_expired && rows.len() > page_limit;
    if has_more {
        rows.truncate(page_limit);
    }
    if rows.is_empty() {
        return Ok(GenerationTaskCheckpointPage {
            items: Vec::new(),
            has_more: false,
        });
    }
    let items = rows
        .into_iter()
        .map(|row| GenerationTaskSummaryRow::from_query_result(&row, ""))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(GenerationTaskSummaryRow::into_checkpoint)
        .collect();
    Ok(GenerationTaskCheckpointPage { items, has_more })
}

fn validate_safe_cursor(
    cursor: &GenerationTaskListCursor,
) -> Result<(), GenerationTaskPersistenceError> {
    if !(0..=JS_SAFE_INTEGER_I64).contains(&cursor.updated_at_ms) {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "history cursor updated_at_ms 超出 JavaScript safe integer".to_string(),
        ));
    }
    let canonical = canonical_uuid_v4(&cursor.task_id, "history cursor task_id")?;
    if canonical != cursor.task_id {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "history cursor task_id 必须为小写 UUID v4".to_string(),
        ));
    }
    Ok(())
}
