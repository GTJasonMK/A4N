use super::*;
use sea_orm::ConnectionTrait;
pub(super) struct PreparedReservation {
    input: ReserveGenerationTaskInput,
    start_request_id: Uuid,
    business_key: String,
    input_fingerprint_sha256: String,
}

pub async fn reserve_generation_task(
    db: &impl crate::DbExecutor,
    input: ReserveGenerationTaskInput,
) -> Result<GenerationTaskReservation, GenerationTaskPersistenceError> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "生成任务 reservation 仅支持 PostgreSQL".to_string(),
        ));
    }
    let mut prepared = prepare_reservation(input)?;
    if let Some(project_id) = prepared.input.project_id {
        let canonical_root_project_id = resolve_generation_root_project_id(db, project_id).await?;
        if let Some(provided_root_project_id) = prepared.input.root_project_id
            && provided_root_project_id != canonical_root_project_id
        {
            return Err(GenerationTaskPersistenceError::InvalidArgument(format!(
                "root_project_id {provided_root_project_id} 与项目 {project_id} 的规范 root {canonical_root_project_id} 不一致"
            )));
        }
        prepared.input.root_project_id = Some(canonical_root_project_id);
    }
    let tx = db.begin().await?;

    if let Some((package_id, reason_code)) = crate::package::package_admission_block(
        &tx,
        prepared.input.task_kind.as_str(),
        prepared.input.task_scope.as_deref(),
        prepared
            .input
            .target
            .as_ref()
            .map(|target| target.kind.as_str()),
    )
    .await?
    {
        tx.rollback().await?;
        return Err(GenerationTaskPersistenceError::Rejected {
            code: "package_admission_blocked",
            message: format!(
                "功能包 {package_id} 当前不允许创建生成任务: reason_code={reason_code}"
            ),
        });
    }

    require_generation_owner_gates_active(
        &tx,
        prepared.input.requested_by_user_id,
        prepared.input.project_id,
        prepared.input.root_project_id,
    )
    .await?;
    if prepared.input.task_scope.as_deref() == Some("chapter_generation") {
        let project_id = prepared.input.project_id.ok_or_else(|| {
            GenerationTaskPersistenceError::InvalidArgument(
                "章节生成任务缺少 project_id".to_string(),
            )
        })?;
        let scope = crate::generation::chapter_generation_project_lock_scope(project_id);
        tx.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            [scope.into()],
        ))
        .await?;
    }
    acquire_generation_advisory_lock(
        &tx,
        START_REQUEST_LOCK_NAMESPACE,
        start_request_lock_hash(
            prepared.input.requested_by_user_id,
            prepared.start_request_id,
        ),
    )
    .await?;

    if let Some(mapping) = generation_start_request::Entity::find_by_id((
        prepared.input.requested_by_user_id,
        prepared.start_request_id,
    ))
    .one(&tx)
    .await?
    {
        ensure_start_request_fingerprint(&mapping, &prepared.input_fingerprint_sha256)?;
        touch_start_request_mapping(&tx, mapping.clone()).await?;
        let task = super::checkpoint::load_persisted_generation_task(&tx, &mapping.task_id).await?;
        let queue_task =
            super::checkpoint::load_generation_queue_task(&tx, &mapping.task_id).await?;
        tx.commit().await?;
        return Ok(GenerationTaskReservation {
            created: false,
            task,
            queue_task,
        });
    }

    acquire_generation_advisory_lock(
        &tx,
        BUSINESS_ATTEMPT_LOCK_NAMESPACE,
        business_attempt_lock_hash(&prepared.business_key),
    )
    .await?;

    if let Some(active) = active_generation_task(&tx, &prepared.business_key).await? {
        ensure_active_task_matches(&active, &prepared)?;
        bind_start_request_alias(&tx, &prepared, &active.task_id).await?;
        let queue_task =
            super::checkpoint::load_generation_queue_task(&tx, &active.task_id).await?;
        let task = persisted_generation_task_from_checkpoint(active)?;
        tx.commit().await?;
        return Ok(GenerationTaskReservation {
            created: false,
            task,
            queue_task,
        });
    }

    let attempt = next_generation_attempt(&tx, &prepared.business_key).await?;
    validate_retry_lineage(&tx, &prepared, attempt).await?;
    validate_task_tree_lineage(&tx, &prepared).await?;
    validate_related_generation_records(&tx, &prepared.input).await?;

    let task_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let (checkpoint, public_snapshot) =
        build_reserved_checkpoint(&prepared, task_id.clone(), attempt, now)?;
    insert_generation_queue_record(&tx, &prepared.input, &task_id, now).await?;
    generation_task_checkpoint::ActiveModel::from(checkpoint.clone())
        .insert(&tx)
        .await?;
    insert_start_request_mapping(&tx, &prepared, &task_id, now).await?;
    let queue_task = super::checkpoint::load_generation_queue_task(&tx, &task_id).await?;
    tx.commit().await?;

    Ok(GenerationTaskReservation {
        created: true,
        task: PersistedGenerationTask {
            checkpoint: generation_task_checkpoint_record(checkpoint),
            public_snapshot: Some(public_snapshot),
        },
        queue_task,
    })
}
pub(super) fn prepare_reservation(
    mut input: ReserveGenerationTaskInput,
) -> Result<PreparedReservation, GenerationTaskPersistenceError> {
    validate_safe_id(input.requested_by_user_id, "requested_by_user_id")?;
    let start_request_id = parse_uuid_v4(&input.start_request_id, "start_request_id")?;
    let business_key_object = input.business_key_json.as_object().ok_or_else(|| {
        GenerationTaskPersistenceError::InvalidArgument(
            "business_key_json 必须是含 version 的对象".to_string(),
        )
    })?;
    if business_key_object
        .get("version")
        .is_none_or(serde_json::Value::is_null)
    {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "business_key_json 缺少非空 version".to_string(),
        ));
    }
    if !input.input_fingerprint_json.is_object() {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "input_fingerprint_json 必须是对象".to_string(),
        ));
    }
    let business_key = canonical_json_text(&input.business_key_json)?;
    if !(2..=2048).contains(&business_key.len()) {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "business_key canonical JSON 必须在 2..=2048 字节内".to_string(),
        ));
    }
    let input_fingerprint_sha256 = canonical_json_sha256(&input.input_fingerprint_json)?;

    validate_optional_safe_id(input.project_id, "project_id")?;
    validate_optional_safe_id(input.root_project_id, "root_project_id")?;
    validate_optional_safe_id(input.source_snapshot_id, "source_snapshot_id")?;
    validate_optional_safe_id(input.workflow_run_id, "workflow_run_id")?;
    validate_optional_safe_id(input.generation_run_id, "generation_run_id")?;
    if input.project_id.is_none() && input.root_project_id.is_some() {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "无 project_id 时不能设置 root_project_id".to_string(),
        ));
    }
    input.task_scope = normalize_optional_text(input.task_scope, "task_scope", 128)?;
    input.root_task_id = normalize_optional_uuid(input.root_task_id, "root_task_id")?;
    input.parent_task_id = normalize_optional_uuid(input.parent_task_id, "parent_task_id")?;
    input.retry_of_task_id = normalize_optional_uuid(input.retry_of_task_id, "retry_of_task_id")?;
    input.item_key = normalize_optional_text(input.item_key, "item_key", 256)?;
    match (
        input.root_task_id.is_some(),
        input.parent_task_id.is_some(),
        input.item_key.is_some(),
    ) {
        (false, false, false) | (true, true, true) => {}
        _ => {
            return Err(GenerationTaskPersistenceError::InvalidArgument(
                "child task 必须同时提供 root_task_id、parent_task_id 和 item_key".to_string(),
            ));
        }
    }
    input.target = normalize_target(input.target)?;

    Ok(PreparedReservation {
        input,
        start_request_id,
        business_key,
        input_fingerprint_sha256,
    })
}

fn normalize_target(
    target: Option<GenerationTargetRef>,
) -> Result<Option<GenerationTargetRef>, GenerationTaskPersistenceError> {
    target
        .map(|target| {
            Ok(GenerationTargetRef {
                kind: required_text(target.kind, "target.kind", 128)?,
                id: required_text(target.id, "target.id", 256)?,
                revision: normalize_optional_text(target.revision, "target.revision", 256)?,
            })
        })
        .transpose()
}

async fn acquire_generation_advisory_lock(
    tx: &DatabaseTransaction,
    namespace: i32,
    key: i32,
) -> Result<(), GenerationTaskPersistenceError> {
    tx.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock($1, $2)",
        [namespace.into(), key.into()],
    ))
    .await?;
    Ok(())
}

pub(super) fn start_request_lock_hash(requested_by_user_id: i64, start_request_id: Uuid) -> i32 {
    let mut hasher = Sha256::new();
    hasher.update(requested_by_user_id.to_be_bytes());
    hasher.update([0]);
    hasher.update(start_request_id.as_bytes());
    digest_prefix_i32(hasher.finalize())
}

pub(super) fn business_attempt_lock_hash(business_key: &str) -> i32 {
    digest_prefix_i32(Sha256::digest(business_key.as_bytes()))
}

fn digest_prefix_i32(digest: impl AsRef<[u8]>) -> i32 {
    let digest = digest.as_ref();
    i32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]])
}

fn ensure_start_request_fingerprint(
    mapping: &generation_start_request::Model,
    expected_sha256: &str,
) -> Result<(), GenerationTaskPersistenceError> {
    if mapping.input_fingerprint_sha256 == expected_sha256 {
        Ok(())
    } else {
        Err(GenerationTaskPersistenceError::Rejected {
            code: "generation_start_request_conflict",
            message: "相同 start_request_id 对应了不同输入".to_string(),
        })
    }
}

async fn touch_start_request_mapping(
    tx: &DatabaseTransaction,
    mapping: generation_start_request::Model,
) -> Result<(), GenerationTaskPersistenceError> {
    let mut active: generation_start_request::ActiveModel = mapping.into();
    active.last_seen_at = Set(Utc::now().naive_utc());
    active.update(tx).await?;
    Ok(())
}

async fn active_generation_task(
    tx: &DatabaseTransaction,
    business_key: &str,
) -> Result<Option<generation_task_checkpoint::Model>, GenerationTaskPersistenceError> {
    Ok(generation_task_checkpoint::Entity::find()
        .filter(generation_task_checkpoint::Column::BusinessKey.eq(business_key))
        .filter(generation_task_checkpoint::Column::Status.is_in(ACTIVE_GENERATION_STATUSES))
        .lock_exclusive()
        .one(tx)
        .await?)
}

fn ensure_active_task_matches(
    active: &generation_task_checkpoint::Model,
    prepared: &PreparedReservation,
) -> Result<(), GenerationTaskPersistenceError> {
    let active_is_root = active.root_task_id == active.task_id
        && active.parent_task_id.is_none()
        && active.item_key.is_none();
    let target_matches = match (
        &prepared.input.target,
        &active.target_kind,
        &active.target_id,
    ) {
        (None, None, None) => active.target_revision.is_none(),
        (Some(target), Some(kind), Some(id)) => {
            target.kind.as_str() == kind.as_str()
                && target.id.as_str() == id.as_str()
                && target.revision.as_deref() == active.target_revision.as_deref()
        }
        _ => false,
    };
    let batch_policy = prepared
        .input
        .batch_policy
        .map(|policy| enum_storage(policy, "batch_policy"))
        .transpose()?;
    let delivery_mode = enum_storage(prepared.input.delivery_mode, "delivery_mode")?;
    let lineage_matches = match prepared.input.root_task_id.as_deref() {
        None => active_is_root,
        Some(root_task_id) => {
            active.root_task_id == root_task_id
                && active.parent_task_id.as_deref() == prepared.input.parent_task_id.as_deref()
                && active.item_key.as_deref() == prepared.input.item_key.as_deref()
        }
    };
    if active.requested_by_user_id == prepared.input.requested_by_user_id
        && active.input_fingerprint_sha256 == prepared.input_fingerprint_sha256
        && active.task_kind == prepared.input.task_kind.as_str()
        && active.delivery_mode == delivery_mode
        && active.batch_policy.as_deref() == batch_policy.as_deref()
        && active.project_id == prepared.input.project_id
        && active.root_project_id == prepared.input.root_project_id
        && active.workflow_run_id == prepared.input.workflow_run_id
        && active.generation_run_id == prepared.input.generation_run_id
        && active.retry_of_task_id.as_deref() == prepared.input.retry_of_task_id.as_deref()
        && target_matches
        && lineage_matches
    {
        Ok(())
    } else {
        Err(GenerationTaskPersistenceError::Rejected {
            code: "generation_attempt_conflict",
            message: "相同 business key 的 active task 与本次请求不一致".to_string(),
        })
    }
}

async fn bind_start_request_alias(
    tx: &DatabaseTransaction,
    prepared: &PreparedReservation,
    task_id: &str,
) -> Result<(), GenerationTaskPersistenceError> {
    let mapping_count = generation_start_request::Entity::find()
        .filter(generation_start_request::Column::TaskId.eq(task_id))
        .count(tx)
        .await?;
    if mapping_count >= MAX_GENERATION_START_REQUEST_MAPPINGS {
        return Err(GenerationTaskPersistenceError::Rejected {
            code: "generation_start_request_capacity_exceeded",
            message: format!(
                "任务 {task_id} 已达到 {MAX_GENERATION_START_REQUEST_MAPPINGS} 个 start request 映射上限"
            ),
        });
    }
    insert_start_request_mapping(tx, prepared, task_id, Utc::now()).await
}

async fn next_generation_attempt(
    tx: &DatabaseTransaction,
    business_key: &str,
) -> Result<i32, GenerationTaskPersistenceError> {
    let previous = generation_task_checkpoint::Entity::find()
        .select_only()
        .column(generation_task_checkpoint::Column::Attempt)
        .filter(generation_task_checkpoint::Column::BusinessKey.eq(business_key))
        .order_by_desc(generation_task_checkpoint::Column::Attempt)
        .into_tuple::<i32>()
        .one(tx)
        .await?;
    previous
        .unwrap_or(0)
        .checked_add(1)
        .filter(|attempt| *attempt > 0)
        .ok_or_else(|| {
            GenerationTaskPersistenceError::InvalidArgument(
                "generation attempt 已耗尽 i32 范围".to_string(),
            )
        })
}

async fn generation_task_lineage_row(
    tx: &DatabaseTransaction,
    task_id: &str,
) -> Result<Option<GenerationTaskLineageRow>, GenerationTaskPersistenceError> {
    Ok(
        generation_task_checkpoint::Entity::find_by_id(task_id.to_string())
            .select_only()
            .column(generation_task_checkpoint::Column::TaskId)
            .column(generation_task_checkpoint::Column::BusinessKey)
            .column(generation_task_checkpoint::Column::Attempt)
            .column(generation_task_checkpoint::Column::RequestedByUserId)
            .column(generation_task_checkpoint::Column::ProjectId)
            .column(generation_task_checkpoint::Column::RootProjectId)
            .column(generation_task_checkpoint::Column::RootTaskId)
            .column(generation_task_checkpoint::Column::ParentTaskId)
            .column(generation_task_checkpoint::Column::ItemKey)
            .lock_shared()
            .into_model::<GenerationTaskLineageRow>()
            .one(tx)
            .await?,
    )
}

async fn validate_retry_lineage(
    tx: &DatabaseTransaction,
    prepared: &PreparedReservation,
    new_attempt: i32,
) -> Result<(), GenerationTaskPersistenceError> {
    let Some(retry_of_task_id) = prepared.input.retry_of_task_id.as_deref() else {
        return Ok(());
    };
    let retry_of = generation_task_lineage_row(tx, retry_of_task_id)
        .await?
        .ok_or_else(|| {
            GenerationTaskPersistenceError::NotFound(format!(
                "retry_of_task_id 不存在: {retry_of_task_id}"
            ))
        })?;
    let retry_is_root = retry_of.root_task_id == retry_of.task_id
        && retry_of.parent_task_id.is_none()
        && retry_of.item_key.is_none();
    let requested_tree_matches = if retry_is_root {
        prepared.input.root_task_id.is_none()
            && prepared.input.parent_task_id.is_none()
            && prepared.input.item_key.is_none()
    } else {
        prepared.input.root_task_id.as_deref() == Some(retry_of.root_task_id.as_str())
            && prepared.input.parent_task_id.as_deref() == retry_of.parent_task_id.as_deref()
            && prepared.input.item_key.as_deref() == retry_of.item_key.as_deref()
    };
    if retry_of.business_key != prepared.business_key
        || retry_of.requested_by_user_id != prepared.input.requested_by_user_id
        || retry_of.project_id != prepared.input.project_id
        || retry_of.root_project_id != prepared.input.root_project_id
        || retry_of.attempt >= new_attempt
        || !requested_tree_matches
    {
        return Err(GenerationTaskPersistenceError::Rejected {
            code: "generation_attempt_conflict",
            message: "retry_of_task_id 与当前 business/user/project/task tree 不一致".to_string(),
        });
    }
    Ok(())
}

async fn validate_task_tree_lineage(
    tx: &DatabaseTransaction,
    prepared: &PreparedReservation,
) -> Result<(), GenerationTaskPersistenceError> {
    let (Some(root_task_id), Some(parent_task_id)) = (
        prepared.input.root_task_id.as_deref(),
        prepared.input.parent_task_id.as_deref(),
    ) else {
        return Ok(());
    };
    let root = generation_task_lineage_row(tx, root_task_id)
        .await?
        .ok_or_else(|| {
            GenerationTaskPersistenceError::NotFound(format!("root_task_id 不存在: {root_task_id}"))
        })?;
    let parent = generation_task_lineage_row(tx, parent_task_id)
        .await?
        .ok_or_else(|| {
            GenerationTaskPersistenceError::NotFound(format!(
                "parent_task_id 不存在: {parent_task_id}"
            ))
        })?;
    if root.root_task_id != root.task_id
        || parent.root_task_id != root.task_id
        || root.requested_by_user_id != prepared.input.requested_by_user_id
        || parent.requested_by_user_id != prepared.input.requested_by_user_id
        || root.project_id != prepared.input.project_id
        || parent.project_id != prepared.input.project_id
        || root.root_project_id != prepared.input.root_project_id
        || parent.root_project_id != prepared.input.root_project_id
    {
        return Err(GenerationTaskPersistenceError::Rejected {
            code: "generation_attempt_conflict",
            message: "root/parent task 与当前 owner/project task tree 不一致".to_string(),
        });
    }
    Ok(())
}

async fn validate_related_generation_records(
    tx: &DatabaseTransaction,
    input: &ReserveGenerationTaskInput,
) -> Result<(), GenerationTaskPersistenceError> {
    if let Some(workflow_run_id) = input.workflow_run_id {
        let (run_project_id, run_root_project_id) =
            workflow_run::Entity::find_by_id(workflow_run_id)
                .select_only()
                .column(workflow_run::Column::ProjectId)
                .column(workflow_run::Column::RootProjectId)
                .lock_shared()
                .into_tuple::<(Option<i64>, Option<i64>)>()
                .one(tx)
                .await?
                .ok_or_else(|| {
                    GenerationTaskPersistenceError::NotFound(format!(
                        "workflow_run 不存在: {workflow_run_id}"
                    ))
                })?;
        if run_project_id != input.project_id
            || run_root_project_id.or(run_project_id) != input.root_project_id
        {
            return Err(GenerationTaskPersistenceError::InvalidArgument(
                "workflow_run 与 generation task project 归属不一致".to_string(),
            ));
        }
    }
    if let Some(generation_run_id) = input.generation_run_id {
        let (run_project_id, run_root_project_id) =
            generation_run::Entity::find_by_id(generation_run_id)
                .select_only()
                .column(generation_run::Column::ProjectId)
                .column(generation_run::Column::RootProjectId)
                .lock_shared()
                .into_tuple::<(i64, i64)>()
                .one(tx)
                .await?
                .ok_or_else(|| {
                    GenerationTaskPersistenceError::NotFound(format!(
                        "generation_run 不存在: {generation_run_id}"
                    ))
                })?;
        if input.project_id != Some(run_project_id)
            || input.root_project_id != Some(run_root_project_id)
        {
            return Err(GenerationTaskPersistenceError::InvalidArgument(
                "generation_run 与 generation task project 归属不一致".to_string(),
            ));
        }
    }
    Ok(())
}

pub(super) fn build_reserved_checkpoint(
    prepared: &PreparedReservation,
    task_id: String,
    attempt: i32,
    now: DateTime<Utc>,
) -> Result<
    (generation_task_checkpoint::Model, GenerationTaskSnapshot),
    GenerationTaskPersistenceError,
> {
    let attempt_u32 = u32::try_from(attempt).map_err(|_| {
        GenerationTaskPersistenceError::InvalidArgument("attempt 超出 u32 范围".to_string())
    })?;
    let root_task_id = prepared
        .input
        .root_task_id
        .clone()
        .unwrap_or_else(|| task_id.clone());
    let started = StartedEvent {
        task_kind: prepared.input.task_kind.as_str().to_string(),
        delivery_mode: prepared.input.delivery_mode,
        project_id: prepared.input.project_id,
        root_project_id: prepared.input.root_project_id,
        workflow_run_id: prepared.input.workflow_run_id,
        generation_run_id: prepared.input.generation_run_id,
        target: prepared.input.target.clone(),
        batch_policy: prepared.input.batch_policy,
        root_task_id: root_task_id.clone(),
        parent_task_id: prepared.input.parent_task_id.clone(),
        item_key: prepared.input.item_key.clone(),
        retry_of_task_id: prepared.input.retry_of_task_id.clone(),
    };
    let public_snapshot = GenerationTaskSnapshot {
        schema_version: GENERATION_EVENT_SCHEMA_VERSION,
        task_id: task_id.clone(),
        attempt: attempt_u32,
        covers_through_sequence: 1,
        status: GenerationTaskStatus::Reserved,
        started,
        cancel_reason: None,
        progress: None,
        streams: Vec::new(),
        artifacts: Vec::new(),
        usage: None,
        latest_checkpoint: None,
        terminal_event: None,
        updated_at_ms: now.timestamp_millis(),
    };
    public_snapshot
        .validate()
        .map_err(|error| GenerationTaskPersistenceError::InvalidArgument(error.to_string()))?;
    let public_snapshot_json = serde_json::to_value(&public_snapshot)?;
    let public_snapshot_sha256 = canonical_json_sha256(&public_snapshot_json)?;
    let target = prepared.input.target.as_ref();
    let now_naive = now.naive_utc();
    let checkpoint = generation_task_checkpoint::Model {
        task_id,
        business_key: prepared.business_key.clone(),
        attempt,
        requested_by_user_id: prepared.input.requested_by_user_id,
        task_kind: prepared.input.task_kind.as_str().to_string(),
        delivery_mode: enum_storage(prepared.input.delivery_mode, "delivery_mode")?,
        batch_policy: prepared
            .input
            .batch_policy
            .map(|policy| enum_storage(policy, "batch_policy"))
            .transpose()?,
        project_id: prepared.input.project_id,
        root_project_id: prepared.input.root_project_id,
        workflow_run_id: prepared.input.workflow_run_id,
        generation_run_id: prepared.input.generation_run_id,
        target_kind: target.map(|target| target.kind.clone()),
        target_id: target.map(|target| target.id.clone()),
        target_revision: target.and_then(|target| target.revision.clone()),
        root_task_id,
        parent_task_id: prepared.input.parent_task_id.clone(),
        item_key: prepared.input.item_key.clone(),
        retry_of_task_id: prepared.input.retry_of_task_id.clone(),
        status: enum_storage(GenerationTaskStatus::Reserved, "status")?,
        checkpoint_revision: 1,
        covers_through_sequence: 1,
        reserved_through_sequence: INITIAL_RESERVED_THROUGH_SEQUENCE,
        public_snapshot_json: Some(public_snapshot_json),
        public_snapshot_sha256: Some(public_snapshot_sha256),
        step_key: None,
        call_key: None,
        provider_kind: None,
        provider_model: None,
        provider_job_id: None,
        provider_idempotency_key_sha256: None,
        resume_metadata_version: None,
        execution_checkpoint_json: None,
        execution_checkpoint_sha256: None,
        input_fingerprint_sha256: prepared.input_fingerprint_sha256.clone(),
        result_kind: None,
        result_id: None,
        result_revision: None,
        expires_at: None,
        created_at: now_naive,
        updated_at: now_naive,
        completed_at: None,
    };
    Ok((checkpoint, public_snapshot))
}

async fn insert_generation_queue_record(
    tx: &DatabaseTransaction,
    input: &ReserveGenerationTaskInput,
    task_id: &str,
    now: DateTime<Utc>,
) -> Result<(), GenerationTaskPersistenceError> {
    task_queue::ActiveModel {
        id: Set(task_id.to_string()),
        task_type: Set(input.task_kind.as_str().to_string()),
        priority: Set(input.priority.as_str().to_string()),
        status: Set("pending".to_string()),
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        source_snapshot_id: Set(input.source_snapshot_id),
        task_scope: Set(input.task_scope.clone()),
        workflow_run_id: Set(input.workflow_run_id),
        payload: Set(None),
        progress: Set(None),
        error_message: Set(None),
        dag_id: Set(None),
        created_at: Set(now.naive_utc()),
        started_at: Set(None),
        completed_at: Set(None),
    }
    .insert(tx)
    .await?;
    Ok(())
}

async fn insert_start_request_mapping(
    tx: &DatabaseTransaction,
    prepared: &PreparedReservation,
    task_id: &str,
    now: DateTime<Utc>,
) -> Result<(), GenerationTaskPersistenceError> {
    let now = now.naive_utc();
    generation_start_request::ActiveModel {
        requested_by_user_id: Set(prepared.input.requested_by_user_id),
        start_request_id: Set(prepared.start_request_id),
        task_id: Set(task_id.to_string()),
        input_fingerprint_sha256: Set(prepared.input_fingerprint_sha256.clone()),
        created_at: Set(now),
        last_seen_at: Set(now),
    }
    .insert(tx)
    .await?;
    Ok(())
}
