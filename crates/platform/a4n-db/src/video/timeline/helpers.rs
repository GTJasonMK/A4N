async fn lock_timeline<C: ConnectionTrait>(
    db: &C,
    timeline_id: i64,
) -> Result<chapter_video_timeline::Model, VideoDbError> {
    chapter_video_timeline::Entity::find_by_id(timeline_id)
        .lock_exclusive()
        .one(db)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("时间线不存在: {timeline_id}")))
}

async fn lock_active_timeline_for_update<C: ConnectionTrait>(
    db: &C,
    timeline_id: i64,
) -> Result<chapter_video_timeline::Model, VideoDbError> {
    let hint = chapter_video_timeline::Entity::find_by_id(timeline_id)
        .one(db)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("时间线不存在: {timeline_id}")))?;
    lock_generated_media_project(db, hint.project_id).await?;
    let timeline = lock_timeline(db, timeline_id).await?;
    if timeline.project_id != hint.project_id {
        return Err(VideoDbError::InvalidState {
            current: "timeline_project_changed".to_string(),
            operation: format!("锁定章节视频时间线 {timeline_id}"),
        });
    }
    Ok(timeline)
}

async fn load_items<C: ConnectionTrait>(
    db: &C,
    timeline_id: i64,
) -> Result<Vec<chapter_video_timeline_item::Model>, VideoDbError> {
    chapter_video_timeline_item::Entity::find()
        .filter(chapter_video_timeline_item::Column::TimelineId.eq(timeline_id))
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}

fn validate_scope(
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
) -> Result<(), VideoDbError> {
    if project_id <= 0 || chapter_id <= 0 {
        return Err(VideoDbError::InvalidArgument(
            "project_id 和 chapter_id 必须大于 0".to_string(),
        ));
    }
    if workflow_mode.trim().is_empty() {
        return Err(VideoDbError::InvalidArgument(
            "workflow_mode 不能为空".to_string(),
        ));
    }
    Ok(())
}

fn ensure_revision(
    timeline: &chapter_video_timeline::Model,
    expected_revision: i64,
) -> Result<(), VideoDbError> {
    if expected_revision <= 0 {
        return Err(VideoDbError::InvalidArgument(
            "expected_timeline_revision 必须大于 0".to_string(),
        ));
    }
    if timeline.revision != expected_revision {
        return Err(VideoDbError::InvalidState {
            current: format!("timeline_revision_conflict:current={}", timeline.revision),
            operation: format!("expected={expected_revision}"),
        });
    }
    Ok(())
}

fn positioned_order(
    items: &[chapter_video_timeline_item::Model],
) -> Result<Vec<i64>, VideoDbError> {
    let mut active = Vec::new();
    for item in items {
        if !matches!(
            item.item_kind.as_str(),
            TIMELINE_NATIVE_KIND | TIMELINE_INSERTED_KIND
        ) {
            return Err(VideoDbError::InvalidState {
                current: format!("unknown_timeline_item_kind:{}", item.item_kind),
                operation: format!("读取时间线项 {}", item.id),
            });
        }
        let identity_valid = if item.item_kind == TIMELINE_NATIVE_KIND {
            item.native_source_shot_id
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
                && item
                    .native_segment_index_in_shot
                    .is_some_and(|value| value > 0)
                && item.insert_plan_prompt_id.is_none()
                && item.workflow_run_id.is_none()
                && item.status != TIMELINE_NEEDS_REANCHOR_STATUS
        } else {
            item.native_source_shot_id.is_none()
                && item.native_segment_index_in_shot.is_none()
                && item.insert_plan_prompt_id.is_some_and(|value| value > 0)
                && item.workflow_run_id.is_some_and(|value| value > 0)
        };
        if !identity_valid {
            return Err(VideoDbError::InvalidState {
                current: "invalid_timeline_item_identity".to_string(),
                operation: format!("读取时间线项 {}", item.id),
            });
        }
        match item.status.as_str() {
            TIMELINE_DELETED_STATUS => {
                if item.position.is_some() {
                    return Err(VideoDbError::InvalidState {
                        current: "invalid_deleted_timeline_item".to_string(),
                        operation: format!("读取时间线项 {}", item.id),
                    });
                }
                continue;
            }
            TIMELINE_ACTIVE_STATUS | TIMELINE_NEEDS_REANCHOR_STATUS => {
                let position = item.position.ok_or_else(|| VideoDbError::InvalidState {
                    current: "active_timeline_item_missing_position".to_string(),
                    operation: format!("读取时间线项 {}", item.id),
                })?;
                if position <= 0 {
                    return Err(VideoDbError::InvalidState {
                        current: "invalid_timeline_item_position".to_string(),
                        operation: format!("读取时间线项 {}", item.id),
                    });
                }
                active.push((position, item.id));
            }
            other => {
                return Err(VideoDbError::InvalidState {
                    current: format!("unknown_timeline_item_status:{other}"),
                    operation: format!("读取时间线项 {}", item.id),
                });
            }
        }
    }
    active.sort_unstable();
    for (index, (position, _)) in active.iter().enumerate() {
        let expected = i32::try_from(index + 1)
            .map_err(|_| VideoDbError::InvalidArgument("时间线片段数量超出范围".to_string()))?;
        if *position != expected {
            return Err(VideoDbError::InvalidState {
                current: "timeline_position_gap_or_duplicate".to_string(),
                operation: "读取章节视频时间线".to_string(),
            });
        }
    }
    Ok(active.into_iter().map(|(_, id)| id).collect())
}

fn ensure_gap_anchors_active(
    items: &[chapter_video_timeline_item::Model],
    gap: VideoTimelineGap,
) -> Result<(), VideoDbError> {
    for item_id in [gap.previous_item_id, gap.next_item_id]
        .into_iter()
        .flatten()
    {
        let item = items
            .iter()
            .find(|item| item.id == item_id)
            .ok_or_else(|| VideoDbError::NotFound(format!("时间线项不存在: {item_id}")))?;
        if item.status != TIMELINE_ACTIVE_STATUS {
            return Err(VideoDbError::InvalidArgument(format!(
                "时间线项 {item_id} 不是活动项，不能作为移动锚点"
            )));
        }
    }
    Ok(())
}

fn rebuild_native_timeline_order(
    current_order: &[i64],
    desired_native_ids: Vec<i64>,
    inserted_ids: &HashSet<i64>,
) -> (Vec<i64>, HashSet<i64>) {
    let native_index = desired_native_ids
        .iter()
        .enumerate()
        .map(|(index, item_id)| (*item_id, index))
        .collect::<HashMap<_, _>>();
    let native_ids = native_index.keys().copied().collect::<HashSet<_>>();

    // The old loop searched the current prefix/suffix for every inserted item.
    // Both anchor directions are stable once the current order is known, so index
    // them once and assign each inserted item to a native gap.
    let mut previous_by_inserted = HashMap::with_capacity(inserted_ids.len());
    let mut previous = None;
    for item_id in current_order {
        if inserted_ids.contains(item_id) {
            previous_by_inserted.insert(*item_id, previous);
        }
        if native_ids.contains(item_id) || inserted_ids.contains(item_id) {
            previous = Some(*item_id);
        }
    }
    let mut next_native_by_inserted = HashMap::with_capacity(inserted_ids.len());
    let mut next_native = None;
    for item_id in current_order.iter().rev() {
        if inserted_ids.contains(item_id) {
            next_native_by_inserted.insert(*item_id, next_native);
        }
        if native_ids.contains(item_id) {
            next_native = Some(*item_id);
        }
    }

    let native_count = desired_native_ids.len();
    let mut inserted_by_gap = vec![Vec::new(); native_count + 1];
    let mut assigned_gap_by_inserted = HashMap::with_capacity(inserted_ids.len());
    let mut last_gap = None;
    for inserted_id in current_order
        .iter()
        .copied()
        .filter(|item_id| inserted_ids.contains(item_id))
    {
        let base_gap = match previous_by_inserted.get(&inserted_id).copied().flatten() {
            Some(previous_id) => native_index
                .get(&previous_id)
                .copied()
                .map(|index| index + 1)
                .or_else(|| assigned_gap_by_inserted.get(&previous_id).copied())
                .expect("previous timeline anchor must have a native gap"),
            None => next_native_by_inserted
                .get(&inserted_id)
                .copied()
                .flatten()
                .and_then(|item_id| native_index.get(&item_id).copied())
                .unwrap_or(native_count),
        };
        let gap = last_gap.map_or(base_gap, |last| base_gap.max(last));
        inserted_by_gap[gap].push(inserted_id);
        assigned_gap_by_inserted.insert(inserted_id, gap);
        last_gap = Some(gap);
    }

    let mut gap_groups = inserted_by_gap.into_iter();
    let mut rebuilt = Vec::with_capacity(current_order.len());
    for native_id in desired_native_ids {
        rebuilt.extend(gap_groups.next().expect("native gap group must exist"));
        rebuilt.push(native_id);
    }
    rebuilt.extend(
        gap_groups
            .next()
            .expect("trailing native gap group must exist"),
    );

    let current_neighbors = timeline_item_neighbor_map(current_order, inserted_ids);
    let rebuilt_neighbors = timeline_item_neighbor_map(&rebuilt, inserted_ids);
    let changed = inserted_ids
        .iter()
        .copied()
        .filter(|item_id| current_neighbors.get(item_id) != rebuilt_neighbors.get(item_id))
        .collect::<HashSet<_>>();
    let mut needs_reanchor = HashSet::new();
    let mut index = 0;
    while index < rebuilt.len() {
        if !inserted_ids.contains(&rebuilt[index]) {
            index += 1;
            continue;
        }
        let start = index;
        while index < rebuilt.len() && inserted_ids.contains(&rebuilt[index]) {
            index += 1;
        }
        if rebuilt[start..index]
            .iter()
            .any(|item_id| changed.contains(item_id))
        {
            needs_reanchor.extend(rebuilt[start..index].iter().copied());
        }
    }
    (rebuilt, needs_reanchor)
}

fn timeline_item_neighbor_map(
    order: &[i64],
    item_ids: &HashSet<i64>,
) -> HashMap<i64, (Option<i64>, Option<i64>)> {
    let mut neighbors = HashMap::with_capacity(item_ids.len());
    for (index, item_id) in order.iter().enumerate() {
        if item_ids.contains(item_id) {
            neighbors.insert(
                *item_id,
                (
                    index
                        .checked_sub(1)
                        .and_then(|index| order.get(index).copied()),
                    order.get(index + 1).copied(),
                ),
            );
        }
    }
    neighbors
}

fn inserted_status_after_native_reconcile(
    current_status: &str,
    neighbors_changed: bool,
) -> &'static str {
    if current_status == TIMELINE_NEEDS_REANCHOR_STATUS || neighbors_changed {
        TIMELINE_NEEDS_REANCHOR_STATUS
    } else {
        TIMELINE_ACTIVE_STATUS
    }
}

fn update_inserted_prompt_display_position(
    metadata: &mut Value,
    segment_number: i32,
) -> Result<(), VideoDbError> {
    let recipe = metadata
        .as_object_mut()
        .and_then(|root| root.get_mut("segment_recipe"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| VideoDbError::InvalidState {
            current: "inserted_prompt_segment_recipe_missing".to_string(),
            operation: "同步插入片段展示位置".to_string(),
        })?;
    recipe.insert("segment_number".to_string(), json!(segment_number));
    Ok(())
}

async fn sync_inserted_prompt_shot_numbers<C: ConnectionTrait>(
    db: &C,
    order: &[i64],
) -> Result<(), VideoDbError> {
    if order.is_empty() {
        return Ok(());
    }
    let items_by_id = chapter_video_timeline_item::Entity::find()
        .filter(chapter_video_timeline_item::Column::Id.is_in(order.to_vec()))
        .all(db)
        .await
        .map_err(VideoDbError::Database)?
        .into_iter()
        .map(|item| (item.id, item))
        .collect::<std::collections::HashMap<_, _>>();
    let prompt_ids = order
        .iter()
        .filter_map(|item_id| items_by_id.get(item_id))
        .filter(|item| item.item_kind == TIMELINE_INSERTED_KIND)
        .filter_map(|item| item.insert_plan_prompt_id)
        .collect::<Vec<_>>();
    let prompts_by_id = if prompt_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::Id.is_in(prompt_ids))
            .all(db)
            .await
            .map_err(VideoDbError::Database)?
            .into_iter()
            .map(|prompt| (prompt.id, prompt))
            .collect::<std::collections::HashMap<_, _>>()
    };
    let mut pending_updates = std::collections::HashMap::new();
    for (index, item_id) in order.iter().enumerate() {
        let item = items_by_id
            .get(item_id)
            .ok_or_else(|| VideoDbError::NotFound(format!("时间线项不存在: {item_id}")))?;
        if item.item_kind != TIMELINE_INSERTED_KIND {
            continue;
        }
        let prompt_id = item
            .insert_plan_prompt_id
            .ok_or_else(|| VideoDbError::InvalidState {
                current: "inserted_timeline_item_missing_prompt".to_string(),
                operation: format!("同步时间线项 {item_id}"),
            })?;
        let prompt = prompts_by_id
            .get(&prompt_id)
            .ok_or_else(|| VideoDbError::NotFound(format!("视频提示词不存在: {prompt_id}")))?;
        let segment_number = i32::try_from(index + 1)
            .map_err(|_| VideoDbError::InvalidArgument("时间线片段数量超出范围".to_string()))?;
        let mut metadata =
            prompt
                .metadata_json
                .clone()
                .ok_or_else(|| VideoDbError::InvalidState {
                    current: "inserted_prompt_metadata_missing".to_string(),
                    operation: format!("同步插入片段提示词 #{prompt_id}"),
                })?;
        update_inserted_prompt_display_position(&mut metadata, segment_number)?;
        if prompt.shot_number != segment_number || prompt.metadata_json.as_ref() != Some(&metadata)
        {
            pending_updates.insert(prompt_id, (segment_number, metadata));
        }
    }
    if pending_updates.is_empty() {
        return Ok(());
    }
    let now = Utc::now().naive_utc();
    let mut shot_number_case = CaseStatement::new();
    let mut metadata_case = CaseStatement::new();
    let mut updated_at_case = CaseStatement::new();
    let prompt_ids = pending_updates.keys().copied().collect::<Vec<_>>();
    for (prompt_id, (segment_number, metadata)) in pending_updates {
        shot_number_case = shot_number_case.case(
            chapter_video_prompt::Column::Id.eq(prompt_id),
            Expr::value(segment_number),
        );
        metadata_case = metadata_case.case(
            chapter_video_prompt::Column::Id.eq(prompt_id),
            Expr::value(Some(metadata)).cast_as("json"),
        );
        updated_at_case = updated_at_case.case(
            chapter_video_prompt::Column::Id.eq(prompt_id),
            Expr::value(now),
        );
    }
    let updated = chapter_video_prompt::Entity::update_many()
        .col_expr(
            chapter_video_prompt::Column::ShotNumber,
            shot_number_case
                .finally(Expr::col(chapter_video_prompt::Column::ShotNumber))
                .into(),
        )
        .col_expr(
            chapter_video_prompt::Column::MetadataJson,
            metadata_case
                .finally(Expr::col(chapter_video_prompt::Column::MetadataJson))
                .into(),
        )
        .col_expr(
            chapter_video_prompt::Column::UpdatedAt,
            updated_at_case
                .finally(Expr::col(chapter_video_prompt::Column::UpdatedAt))
                .into(),
        )
        .filter(chapter_video_prompt::Column::Id.is_in(prompt_ids.clone()))
        .exec(db)
        .await
        .map_err(VideoDbError::Database)?;
    if updated.rows_affected != prompt_ids.len() as u64 {
        return Err(VideoDbError::InvalidState {
            current: format!(
                "插入片段提示词展示位置批量更新影响 {} 行，预期 {} 行",
                updated.rows_affected,
                prompt_ids.len()
            ),
            operation: "同步插入片段展示位置".to_string(),
        });
    }
    Ok(())
}

async fn rewrite_positions<C: ConnectionTrait>(
    db: &C,
    timeline: &chapter_video_timeline::Model,
    order: &[i64],
) -> Result<(), VideoDbError> {
    let now = Utc::now().naive_utc();
    let revision = timeline
        .revision
        .checked_add(1)
        .ok_or_else(|| VideoDbError::InvalidArgument("时间线 revision 超出范围".to_string()))?;
    if !order.is_empty() {
        let mut positions = CaseStatement::new();
        for (index, item_id) in order.iter().enumerate() {
            let position = i32::try_from(index + 1)
                .map_err(|_| VideoDbError::InvalidArgument("时间线片段数量超出范围".to_string()))?;
            positions = positions.case(
                chapter_video_timeline_item::Column::Id.eq(*item_id),
                position,
            );
        }
        let updated = chapter_video_timeline_item::Entity::update_many()
            .col_expr(
                chapter_video_timeline_item::Column::Position,
                positions
                    .finally(Expr::col(chapter_video_timeline_item::Column::Position))
                    .into(),
            )
            .col_expr(
                chapter_video_timeline_item::Column::UpdatedAt,
                Expr::value(now),
            )
            .filter(chapter_video_timeline_item::Column::TimelineId.eq(timeline.id))
            .filter(chapter_video_timeline_item::Column::Id.is_in(order.to_vec()))
            .exec(db)
            .await
            .map_err(VideoDbError::Database)?;
        if updated.rows_affected != order.len() as u64 {
            let existing_ids = chapter_video_timeline_item::Entity::find()
                .select_only()
                .column(chapter_video_timeline_item::Column::Id)
                .filter(chapter_video_timeline_item::Column::TimelineId.eq(timeline.id))
                .filter(chapter_video_timeline_item::Column::Id.is_in(order.to_vec()))
                .into_tuple::<i64>()
                .all(db)
                .await
                .map_err(VideoDbError::Database)?
                .into_iter()
                .collect::<std::collections::HashSet<_>>();
            let missing_id = order
                .iter()
                .find(|item_id| !existing_ids.contains(item_id))
                .copied()
                .unwrap_or_default();
            return Err(VideoDbError::NotFound(format!(
                "时间线项不存在: {missing_id}"
            )));
        }
    }
    let mut timeline_active: chapter_video_timeline::ActiveModel = timeline.clone().into();
    timeline_active.revision = Set(revision);
    timeline_active.updated_at = Set(now);
    timeline_active
        .update(db)
        .await
        .map_err(VideoDbError::Database)?;
    Ok(())
}
