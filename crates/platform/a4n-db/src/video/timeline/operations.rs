/// 在同一事务中提交插入片段提示词和时间线项。
///
/// 草稿阶段不会触碰 canonical 表；只有这里成功后，提示词、时间线顺序和
/// `timeline_item_id` 才会一起可见。
pub async fn commit_inserted_timeline_item(
    db: &DatabaseConnection,
    input: CommitInsertedTimelineItemInput,
) -> Result<(), VideoDbError> {
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let timeline = lock_active_timeline_for_update(&txn, input.timeline_id).await?;
    let existing = load_items(&txn, timeline.id).await?;
    if input.workflow_run_id <= 0 {
        return Err(VideoDbError::InvalidArgument(
            "workflow_run_id 必须大于 0".to_string(),
        ));
    }
    if let Some(item) = inserted_timeline_item_by_workflow_run(&existing, input.workflow_run_id) {
        if item.status == TIMELINE_DELETED_STATUS {
            return Err(VideoDbError::InvalidState {
                current: "inserted_segment_already_deleted".to_string(),
                operation: format!("恢复插入草稿 workflow_run_id={}", input.workflow_run_id),
            });
        }
        if item.insert_plan_prompt_id.is_none() {
            return Err(VideoDbError::InvalidState {
                current: "inserted_timeline_item_missing_prompt".to_string(),
                operation: format!("恢复插入草稿 workflow_run_id={}", input.workflow_run_id),
            });
        }
        txn.commit().await.map_err(VideoDbError::Database)?;
        return Ok(());
    }
    ensure_revision(&timeline, input.expected_revision)?;
    if input.prompt.project_id <= 0 || input.prompt.chapter_id <= 0 {
        return Err(VideoDbError::InvalidArgument(
            "插入片段提示词必须包含有效项目和章节".to_string(),
        ));
    }
    if input.prompt.project_id != timeline.project_id
        || input.prompt.chapter_id != timeline.chapter_id
    {
        return Err(VideoDbError::InvalidArgument(
            "插入片段提示词与时间线范围不一致".to_string(),
        ));
    }
    if input.prompt.prompt_type.trim().is_empty()
        || input.prompt.workflow_mode.trim().is_empty()
        || input.prompt.visual_prompt.trim().is_empty()
    {
        return Err(VideoDbError::InvalidArgument(
            "插入片段提示词缺少 prompt_type、workflow_mode 或 visual_prompt".to_string(),
        ));
    }

    let order = positioned_order(&existing)?;
    let insertion_index = timeline_gap_index(&order, input.gap)
        .map_err(|error| VideoDbError::InvalidArgument(error.to_string()))?;
    let temporary_position = i32::try_from(order.len() + 1)
        .map_err(|_| VideoDbError::InvalidArgument("时间线片段数量超出范围".to_string()))?;
    let now = Utc::now().naive_utc();
    let prompt = chapter_video_prompt::ActiveModel {
        project_id: Set(input.prompt.project_id),
        chapter_id: Set(input.prompt.chapter_id),
        style: Set(input.prompt.style),
        prompt_type: Set(input.prompt.prompt_type),
        workflow_mode: Set(input.prompt.workflow_mode),
        shot_number: Set(input.prompt.shot_number),
        title: Set(input.prompt.title),
        scene_summary: Set(input.prompt.scene_summary),
        visual_prompt: Set(input.prompt.visual_prompt),
        motion_prompt: Set(input.prompt.motion_prompt),
        negative_prompt: Set(input.prompt.negative_prompt),
        duration_secs: Set(input.prompt.duration_secs),
        aspect_ratio: Set(input.prompt.aspect_ratio),
        reference_image_ids_json: Set(input.prompt.reference_image_ids_json),
        status: Set(chapter_video_prompt::STATUS_COMPLETED.to_string()),
        generated_video_id: Set(None),
        timeline_item_id: Set(None),
        metadata_json: Set(input.prompt.metadata_json),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(VideoDbError::Database)?;

    let item = chapter_video_timeline_item::ActiveModel {
        timeline_id: Set(timeline.id),
        position: Set(Some(temporary_position)),
        item_kind: Set(TIMELINE_INSERTED_KIND.to_string()),
        status: Set(TIMELINE_ACTIVE_STATUS.to_string()),
        native_source_shot_id: Set(None),
        native_segment_index_in_shot: Set(None),
        insert_plan_prompt_id: Set(Some(prompt.id)),
        workflow_run_id: Set(Some(input.workflow_run_id)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(VideoDbError::Database)?;
    let mut new_order = order;
    new_order.insert(insertion_index, item.id);
    rewrite_positions(&txn, &timeline, &new_order).await?;

    let mut prompt_active: chapter_video_prompt::ActiveModel = prompt.into();
    prompt_active.timeline_item_id = Set(Some(item.id));
    prompt_active.updated_at = Set(now);
    prompt_active
        .update(&txn)
        .await
        .map_err(VideoDbError::Database)?;
    sync_inserted_prompt_shot_numbers(&txn, &new_order).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(())
}

/// 获取或创建章节时间线。创建时为空，原生项由调用方随后同步。
pub async fn ensure_chapter_video_timeline(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
) -> Result<chapter_video_timeline::Model, VideoDbError> {
    validate_scope(project_id, chapter_id, workflow_mode)?;
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    lock_generated_media_project(&txn, project_id).await?;
    let now = Utc::now().naive_utc();
    chapter_video_timeline::Entity::insert(chapter_video_timeline::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        workflow_mode: Set(workflow_mode.to_string()),
        revision: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::columns([
            chapter_video_timeline::Column::ProjectId,
            chapter_video_timeline::Column::ChapterId,
            chapter_video_timeline::Column::WorkflowMode,
        ])
        .do_nothing()
        .to_owned(),
    )
    .do_nothing()
    .exec_without_returning(&txn)
    .await
    .map_err(VideoDbError::Database)?;
    let timeline = chapter_video_timeline_by_scope(&txn, project_id, chapter_id, workflow_mode)
        .await?
        .ok_or_else(|| VideoDbError::InvalidState {
            current: "timeline_create_conflict_without_row".to_string(),
            operation: format!(
                "读取章节视频时间线 project_id={project_id}, chapter_id={chapter_id}, workflow_mode={workflow_mode}"
            ),
        })?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(timeline)
}

pub async fn chapter_video_timeline_by_scope<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
) -> Result<Option<chapter_video_timeline::Model>, VideoDbError>
where
    C: ConnectionTrait,
{
    validate_scope(project_id, chapter_id, workflow_mode)?;
    chapter_video_timeline::Entity::find()
        .filter(chapter_video_timeline::Column::ProjectId.eq(project_id))
        .filter(chapter_video_timeline::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_timeline::Column::WorkflowMode.eq(workflow_mode))
        .one(db)
        .await
        .map_err(VideoDbError::Database)
}

/// 读取并验证活动位置顺序；软删除项保留身份记录，但没有位置。
pub async fn chapter_video_timeline_snapshot(
    db: &DatabaseConnection,
    timeline_id: i64,
) -> Result<VideoTimelineSnapshot, VideoDbError> {
    if timeline_id <= 0 {
        return Err(VideoDbError::InvalidArgument(
            "timeline_id 必须大于 0".to_string(),
        ));
    }
    let timeline_result = chapter_video_timeline::Entity::find_by_id(timeline_id).one(db);
    let items_result = chapter_video_timeline_item::Entity::find()
        .filter(chapter_video_timeline_item::Column::TimelineId.eq(timeline_id))
        .order_by_asc(chapter_video_timeline_item::Column::Position)
        .order_by_asc(chapter_video_timeline_item::Column::Id)
        .all(db);
    let (timeline_result, items_result) = futures::join!(timeline_result, items_result);
    let timeline = timeline_result
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("时间线不存在: {timeline_id}")))?;
    let items = items_result.map_err(VideoDbError::Database)?;
    let _ = positioned_order(&items)?;
    Ok(VideoTimelineSnapshot { timeline, items })
}

pub fn active_timeline_order(snapshot: &VideoTimelineSnapshot) -> Result<Vec<i64>, VideoDbError> {
    positioned_order(&snapshot.items)
}

pub fn inserted_timeline_item_by_workflow_run(
    items: &[chapter_video_timeline_item::Model],
    workflow_run_id: i64,
) -> Option<&chapter_video_timeline_item::Model> {
    (workflow_run_id > 0).then_some(())?;
    items.iter().find(|item| {
        item.item_kind == TIMELINE_INSERTED_KIND && item.workflow_run_id == Some(workflow_run_id)
    })
}

/// 按共享规划的稳定来源键同步 native 项。
pub async fn reconcile_native_timeline(
    db: &DatabaseConnection,
    timeline_id: i64,
    keys: &[NativeVideoTimelineKey],
) -> Result<VideoTimelineSnapshot, VideoDbError> {
    if timeline_id <= 0 {
        return Err(VideoDbError::InvalidArgument(
            "timeline_id 必须大于 0".to_string(),
        ));
    }
    let mut unique_keys = std::collections::HashSet::with_capacity(keys.len());
    if keys.iter().any(|key| !unique_keys.insert(key.clone())) {
        return Err(VideoDbError::InvalidArgument(
            "原生时间线稳定身份不能重复".to_string(),
        ));
    }
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let timeline = lock_active_timeline_for_update(&txn, timeline_id).await?;
    if keys.is_empty() {
        return Err(VideoDbError::InvalidArgument(
            "原生时间线不能同步为空规划".to_string(),
        ));
    }
    let existing = chapter_video_timeline_item::Entity::find()
        .filter(chapter_video_timeline_item::Column::TimelineId.eq(timeline_id))
        .all(&txn)
        .await
        .map_err(VideoDbError::Database)?;
    let current_order = positioned_order(&existing)?;
    let mut next_temporary_position = existing
        .iter()
        .filter_map(|item| item.position)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| VideoDbError::InvalidArgument("时间线片段数量超出范围".to_string()))?;
    let mut by_key = std::collections::HashMap::new();
    for item in existing
        .iter()
        .filter(|item| item.item_kind == TIMELINE_NATIVE_KIND)
    {
        let source_shot_id =
            item.native_source_shot_id
                .clone()
                .ok_or_else(|| VideoDbError::InvalidState {
                    current: "native_timeline_item_missing_source_shot_id".to_string(),
                    operation: format!("同步时间线项 {}", item.id),
                })?;
        let segment_index =
            item.native_segment_index_in_shot
                .ok_or_else(|| VideoDbError::InvalidState {
                    current: "native_timeline_item_missing_segment_index".to_string(),
                    operation: format!("同步时间线项 {}", item.id),
                })?;
        let key = NativeVideoTimelineKey::new(source_shot_id, segment_index).map_err(|error| {
            VideoDbError::InvalidState {
                current: error,
                operation: format!("同步时间线项 {}", item.id),
            }
        })?;
        if by_key.insert(key, item).is_some() {
            return Err(VideoDbError::InvalidState {
                current: "duplicate_native_timeline_identity".to_string(),
                operation: format!("同步章节时间线 {timeline_id}"),
            });
        }
    }

    let now = Utc::now().naive_utc();
    let mut desired_native_ids = vec![None; keys.len()];
    let mut reactivated = Vec::new();
    let mut pending_inserts = Vec::new();
    let mut pending_insert_slots = Vec::new();
    for (slot, key) in keys.iter().enumerate() {
        if let Some(item) = by_key.remove(key) {
            if item.status != TIMELINE_ACTIVE_STATUS {
                reactivated.push((item.id, next_temporary_position));
                next_temporary_position =
                    next_temporary_position.checked_add(1).ok_or_else(|| {
                        VideoDbError::InvalidArgument("时间线片段数量超出范围".to_string())
                    })?;
            }
            desired_native_ids[slot] = Some(item.id);
        } else {
            pending_insert_slots.push((slot, key.clone()));
            pending_inserts.push(chapter_video_timeline_item::ActiveModel {
                timeline_id: Set(timeline_id),
                position: Set(Some(next_temporary_position)),
                item_kind: Set(TIMELINE_NATIVE_KIND.to_string()),
                status: Set(TIMELINE_ACTIVE_STATUS.to_string()),
                native_source_shot_id: Set(Some(key.source_shot_id.clone())),
                native_segment_index_in_shot: Set(Some(key.segment_index_in_shot)),
                insert_plan_prompt_id: Set(None),
                workflow_run_id: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            });
            next_temporary_position = next_temporary_position.checked_add(1).ok_or_else(|| {
                VideoDbError::InvalidArgument("时间线片段数量超出范围".to_string())
            })?;
        }
    }
    if !reactivated.is_empty() {
        let ids = reactivated.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        let mut position_case = CaseStatement::new();
        for (id, position) in &reactivated {
            position_case = position_case.case(
                chapter_video_timeline_item::Column::Id.eq(*id),
                Expr::value(Some(*position)),
            );
        }
        let updated = chapter_video_timeline_item::Entity::update_many()
            .col_expr(
                chapter_video_timeline_item::Column::Status,
                Expr::value(TIMELINE_ACTIVE_STATUS.to_string()),
            )
            .col_expr(
                chapter_video_timeline_item::Column::Position,
                position_case
                    .finally(Expr::col(chapter_video_timeline_item::Column::Position))
                    .into(),
            )
            .col_expr(
                chapter_video_timeline_item::Column::UpdatedAt,
                Expr::value(now),
            )
            .filter(chapter_video_timeline_item::Column::TimelineId.eq(timeline_id))
            .filter(chapter_video_timeline_item::Column::Id.is_in(ids.clone()))
            .exec(&txn)
            .await
            .map_err(VideoDbError::Database)?;
        if updated.rows_affected != ids.len() as u64 {
            return Err(VideoDbError::InvalidState {
                current: format!(
                    "重新激活 native 时间线项批量更新影响 {} 行，预期 {} 行",
                    updated.rows_affected,
                    ids.len()
                ),
                operation: "同步章节时间线".to_string(),
            });
        }
    }
    if !pending_inserts.is_empty() {
        let expected = pending_inserts.len();
        let inserted = chapter_video_timeline_item::Entity::insert_many(pending_inserts)
            .exec_with_returning_many(&txn)
            .await
            .map_err(VideoDbError::Database)?;
        if inserted.len() != expected {
            return Err(VideoDbError::InvalidState {
                current: format!(
                    "新增 native 时间线项批量插入返回 {} 行，预期 {} 行",
                    inserted.len(),
                    expected
                ),
                operation: "同步章节时间线".to_string(),
            });
        }
        let inserted_by_key = inserted
            .into_iter()
            .map(|item| {
                (
                    (
                        item.native_source_shot_id.clone().unwrap_or_default(),
                        item.native_segment_index_in_shot.unwrap_or_default(),
                    ),
                    item.id,
                )
            })
            .collect::<std::collections::HashMap<_, _>>();
        for (slot, key) in pending_insert_slots {
            desired_native_ids[slot] = Some(
                *inserted_by_key
                    .get(&(key.source_shot_id, key.segment_index_in_shot))
                    .ok_or_else(|| VideoDbError::InvalidState {
                        current: "新增 native 时间线项批量插入后无法按稳定键回读".to_string(),
                        operation: "同步章节时间线".to_string(),
                    })?,
            );
        }
    }
    let desired_native_ids = desired_native_ids
        .into_iter()
        .map(|id| {
            id.ok_or_else(|| VideoDbError::InvalidState {
                current: "native 时间线稳定键缺少持久化 ID".to_string(),
                operation: "同步章节时间线".to_string(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    // 保留插入项的相对位置；没有出现在新 native 规划中的 native 项进入删除状态，
    // 不硬删除，以便历史/撤销和问题修复仍有依据。
    let stale_native_ids = by_key
        .values()
        .filter(|item| item.status != TIMELINE_DELETED_STATUS)
        .map(|item| item.id)
        .collect::<Vec<_>>();
    if !stale_native_ids.is_empty() {
        let now = Utc::now().naive_utc();
        let updated = chapter_video_timeline_item::Entity::update_many()
            .col_expr(
                chapter_video_timeline_item::Column::Status,
                Expr::value(TIMELINE_DELETED_STATUS.to_string()),
            )
            .col_expr(
                chapter_video_timeline_item::Column::Position,
                Expr::value(Option::<i32>::None),
            )
            .col_expr(
                chapter_video_timeline_item::Column::UpdatedAt,
                Expr::value(now),
            )
            .filter(chapter_video_timeline_item::Column::TimelineId.eq(timeline_id))
            .filter(chapter_video_timeline_item::Column::Id.is_in(stale_native_ids.clone()))
            .filter(chapter_video_timeline_item::Column::Status.ne(TIMELINE_DELETED_STATUS))
            .exec(&txn)
            .await
            .map_err(VideoDbError::Database)?;
        if updated.rows_affected != stale_native_ids.len() as u64 {
            return Err(VideoDbError::InvalidState {
                current: format!(
                    "失效 native 时间线项批量更新影响 {} 行，预期 {} 行",
                    updated.rows_affected,
                    stale_native_ids.len()
                ),
                operation: "同步章节时间线".to_string(),
            });
        }
    }

    let inserted_id_set = existing
        .iter()
        .filter(|item| item.item_kind == TIMELINE_INSERTED_KIND)
        .map(|item| item.id)
        .collect::<HashSet<_>>();
    let inserted_ids = current_order
        .iter()
        .copied()
        .filter(|id| inserted_id_set.contains(id))
        .collect::<Vec<_>>();
    let existing_by_id = if inserted_ids.is_empty() {
        HashMap::new()
    } else {
        existing
            .iter()
            .map(|item| (item.id, item))
            .collect::<HashMap<_, _>>()
    };
    // 以原生规划顺序为主，将插入项保留在原来相邻的 gap；无法恢复的项置于末尾并标记。
    let (rebuilt, needs_reanchor) =
        rebuild_native_timeline_order(&current_order, desired_native_ids, &inserted_id_set);
    let mut inserted_status_ids = HashMap::<&'static str, Vec<i64>>::new();
    for inserted_id in inserted_ids {
        let item = existing_by_id.get(&inserted_id).copied().ok_or_else(|| {
            VideoDbError::InvalidState {
                current: "timeline_inserted_item_unreachable".to_string(),
                operation: format!("重建时间线项 {inserted_id}"),
            }
        })?;
        let status = inserted_status_after_native_reconcile(
            &item.status,
            needs_reanchor.contains(&inserted_id),
        );
        if item.status != status {
            inserted_status_ids.entry(status).or_default().push(item.id);
        }
    }
    for (status, item_ids) in inserted_status_ids {
        let updated = chapter_video_timeline_item::Entity::update_many()
            .col_expr(
                chapter_video_timeline_item::Column::Status,
                Expr::value(status.to_string()),
            )
            .col_expr(
                chapter_video_timeline_item::Column::UpdatedAt,
                Expr::value(Utc::now().naive_utc()),
            )
            .filter(chapter_video_timeline_item::Column::TimelineId.eq(timeline_id))
            .filter(chapter_video_timeline_item::Column::Id.is_in(item_ids.clone()))
            .exec(&txn)
            .await
            .map_err(VideoDbError::Database)?;
        if updated.rows_affected != item_ids.len() as u64 {
            return Err(VideoDbError::InvalidState {
                current: format!(
                    "插入时间线项状态批量更新影响 {} 行，预期 {} 行",
                    updated.rows_affected,
                    item_ids.len()
                ),
                operation: "同步章节时间线".to_string(),
            });
        }
    }
    rewrite_positions(&txn, &timeline, &rebuilt).await?;
    sync_inserted_prompt_shot_numbers(&txn, &rebuilt).await?;
    let timeline = chapter_video_timeline::Entity::find_by_id(timeline_id)
        .one(&txn)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("时间线不存在: {timeline_id}")))?;
    let items = chapter_video_timeline_item::Entity::find()
        .filter(chapter_video_timeline_item::Column::TimelineId.eq(timeline_id))
        .all(&txn)
        .await
        .map_err(VideoDbError::Database)?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(VideoTimelineSnapshot { timeline, items })
}

pub async fn move_inserted_timeline_item_persisted(
    db: &DatabaseConnection,
    timeline_id: i64,
    expected_revision: i64,
    item_id: i64,
    gap: VideoTimelineGap,
) -> Result<(), VideoDbError> {
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let timeline = lock_active_timeline_for_update(&txn, timeline_id).await?;
    ensure_revision(&timeline, expected_revision)?;
    let existing = load_items(&txn, timeline.id).await?;
    let target = existing
        .iter()
        .find(|item| item.id == item_id)
        .ok_or_else(|| VideoDbError::NotFound(format!("时间线项不存在: {item_id}")))?;
    if target.item_kind != TIMELINE_INSERTED_KIND {
        return Err(VideoDbError::InvalidArgument(
            "只能移动插入片段".to_string(),
        ));
    }
    ensure_gap_anchors_active(&existing, gap)?;
    let order = positioned_order(&existing)?;
    let new_order = move_inserted_timeline_item(&order, item_id, gap)
        .map_err(|error| VideoDbError::InvalidArgument(error.to_string()))?;
    if new_order == order && target.status == TIMELINE_ACTIVE_STATUS {
        txn.commit().await.map_err(VideoDbError::Database)?;
        return Ok(());
    }
    if target.status != TIMELINE_ACTIVE_STATUS {
        let mut active: chapter_video_timeline_item::ActiveModel = target.clone().into();
        active.status = Set(TIMELINE_ACTIVE_STATUS.to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(&txn).await.map_err(VideoDbError::Database)?;
    }
    rewrite_positions(&txn, &timeline, &new_order).await?;
    sync_inserted_prompt_shot_numbers(&txn, &new_order).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(())
}

pub async fn delete_inserted_timeline_item(
    db: &DatabaseConnection,
    timeline_id: i64,
    expected_revision: i64,
    item_id: i64,
) -> Result<(), VideoDbError> {
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let timeline = lock_active_timeline_for_update(&txn, timeline_id).await?;
    ensure_revision(&timeline, expected_revision)?;
    let existing = load_items(&txn, timeline.id).await?;
    let target = existing
        .iter()
        .find(|item| item.id == item_id)
        .ok_or_else(|| VideoDbError::NotFound(format!("时间线项不存在: {item_id}")))?;
    match target.item_kind.as_str() {
        TIMELINE_INSERTED_KIND => {}
        TIMELINE_NATIVE_KIND => {
            return Err(VideoDbError::InvalidArgument(
                "只能删除插入片段".to_string(),
            ));
        }
        other => {
            return Err(VideoDbError::InvalidState {
                current: format!("unknown_timeline_item_kind:{other}"),
                operation: format!("删除时间线项 {item_id}"),
            });
        }
    }
    let order = positioned_order(&existing)?;
    let new_order = remove_inserted_timeline_item(&order, item_id)
        .map_err(|error| VideoDbError::InvalidArgument(error.to_string()))?;
    let mut active: chapter_video_timeline_item::ActiveModel = target.clone().into();
    active.status = Set(TIMELINE_DELETED_STATUS.to_string());
    active.position = Set(None);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(&txn).await.map_err(VideoDbError::Database)?;
    rewrite_positions(&txn, &timeline, &new_order).await?;
    sync_inserted_prompt_shot_numbers(&txn, &new_order).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(())
}
