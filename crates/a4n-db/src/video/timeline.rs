//! 章节视频时间线持久化。
//!
//! 这里是唯一负责修改章节时间线顺序和 revision 的数据库边界。普通共享
//! 片段规划只通过 `reconcile_native_timeline` 同步 native 项；插入项不会
//! 写回共享 `segments` JSON。

use a4n_core::{
    models::chapter::{chapter_video_prompt, chapter_video_timeline, chapter_video_timeline_item},
    video_execution::{
        NativeVideoTimelineKey, VideoTimelineGap, move_inserted_timeline_item,
        remove_inserted_timeline_item,
    },
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait, sea_query::OnConflict,
};
use serde_json::{Value, json};

use super::{VideoDbError, lock_generated_media_project};

pub const TIMELINE_NATIVE_KIND: &str = "native";
pub const TIMELINE_INSERTED_KIND: &str = "inserted";
pub const TIMELINE_ACTIVE_STATUS: &str = "active";
pub const TIMELINE_DELETED_STATUS: &str = "deleted";
pub const TIMELINE_NEEDS_REANCHOR_STATUS: &str = "needs_reanchor";

#[derive(Debug, Clone, PartialEq)]
pub struct VideoTimelineSnapshot {
    pub timeline: chapter_video_timeline::Model,
    pub items: Vec<chapter_video_timeline_item::Model>,
}

#[derive(Debug, Clone)]
pub struct CommitInsertedTimelineItemInput {
    pub timeline_id: i64,
    pub expected_revision: i64,
    pub gap: VideoTimelineGap,
    pub prompt: super::ChapterVideoPromptRecordFields,
    pub workflow_run_id: i64,
}

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
    let insertion_index = a4n_core::video_execution::timeline_gap_index(&order, input.gap)
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
    let timeline = chapter_video_timeline::Entity::find_by_id(timeline_id)
        .one(db)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("时间线不存在: {timeline_id}")))?;
    let items = chapter_video_timeline_item::Entity::find()
        .filter(chapter_video_timeline_item::Column::TimelineId.eq(timeline_id))
        .order_by_asc(chapter_video_timeline_item::Column::Position)
        .order_by_asc(chapter_video_timeline_item::Column::Id)
        .all(db)
        .await
        .map_err(VideoDbError::Database)?;
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

    let mut desired_native_ids = Vec::with_capacity(keys.len());
    for key in keys {
        if let Some(item) = by_key.remove(key) {
            if item.status != TIMELINE_ACTIVE_STATUS {
                let mut active: chapter_video_timeline_item::ActiveModel = item.clone().into();
                active.status = Set(TIMELINE_ACTIVE_STATUS.to_string());
                active.position = Set(Some(next_temporary_position));
                active.updated_at = Set(Utc::now().naive_utc());
                active.update(&txn).await.map_err(VideoDbError::Database)?;
                next_temporary_position =
                    next_temporary_position.checked_add(1).ok_or_else(|| {
                        VideoDbError::InvalidArgument("时间线片段数量超出范围".to_string())
                    })?;
            }
            desired_native_ids.push(item.id);
        } else {
            let now = Utc::now().naive_utc();
            let item = chapter_video_timeline_item::ActiveModel {
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
            }
            .insert(&txn)
            .await
            .map_err(VideoDbError::Database)?;
            desired_native_ids.push(item.id);
            next_temporary_position = next_temporary_position.checked_add(1).ok_or_else(|| {
                VideoDbError::InvalidArgument("时间线片段数量超出范围".to_string())
            })?;
        }
    }

    // 保留插入项的相对位置；没有出现在新 native 规划中的 native 项进入删除状态，
    // 不硬删除，以便历史/撤销和问题修复仍有依据。
    for item in by_key.values() {
        if item.status == TIMELINE_DELETED_STATUS {
            continue;
        }
        let mut active: chapter_video_timeline_item::ActiveModel = (*item).clone().into();
        active.status = Set(TIMELINE_DELETED_STATUS.to_string());
        active.position = Set(None);
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(&txn).await.map_err(VideoDbError::Database)?;
    }

    let inserted_id_set = existing
        .iter()
        .filter(|item| item.item_kind == TIMELINE_INSERTED_KIND)
        .map(|item| item.id)
        .collect::<std::collections::HashSet<_>>();
    let inserted_ids = current_order
        .iter()
        .copied()
        .filter(|id| inserted_id_set.contains(id))
        .collect::<Vec<_>>();
    // 以原生规划顺序为主，将插入项保留在原来相邻的 gap；无法恢复的项置于末尾并标记。
    let (rebuilt, needs_reanchor) =
        rebuild_native_timeline_order(&current_order, desired_native_ids, &inserted_id_set);
    for inserted_id in inserted_ids {
        let item = existing
            .iter()
            .find(|item| item.id == inserted_id)
            .ok_or_else(|| VideoDbError::InvalidState {
                current: "timeline_inserted_item_unreachable".to_string(),
                operation: format!("重建时间线项 {inserted_id}"),
            })?;
        let status = inserted_status_after_native_reconcile(
            &item.status,
            needs_reanchor.contains(&inserted_id),
        );
        if item.status != status {
            let mut active: chapter_video_timeline_item::ActiveModel = item.clone().into();
            active.status = Set(status.to_string());
            active.updated_at = Set(Utc::now().naive_utc());
            active.update(&txn).await.map_err(VideoDbError::Database)?;
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
    inserted_ids: &std::collections::HashSet<i64>,
) -> (Vec<i64>, std::collections::HashSet<i64>) {
    let mut rebuilt = desired_native_ids;
    let mut last_inserted_id = None;
    for (old_index, inserted_id) in current_order.iter().copied().enumerate() {
        if !inserted_ids.contains(&inserted_id) {
            continue;
        }
        let previous = current_order[..old_index]
            .iter()
            .rev()
            .copied()
            .find(|item_id| rebuilt.contains(item_id));
        let next = current_order[old_index + 1..]
            .iter()
            .copied()
            .find(|item_id| rebuilt.contains(item_id));
        let insertion_index = previous
            .and_then(|item_id| rebuilt.iter().position(|candidate| *candidate == item_id))
            .map(|index| index + 1)
            .or_else(|| {
                next.and_then(|item_id| rebuilt.iter().position(|candidate| *candidate == item_id))
            })
            .unwrap_or(rebuilt.len());
        let insertion_index = last_inserted_id
            .and_then(|item_id| rebuilt.iter().position(|candidate| *candidate == item_id))
            .map(|index| insertion_index.max(index + 1))
            .unwrap_or(insertion_index);
        rebuilt.insert(insertion_index, inserted_id);
        last_inserted_id = Some(inserted_id);
    }

    let changed = inserted_ids
        .iter()
        .copied()
        .filter(|item_id| {
            timeline_item_neighbors(current_order, *item_id)
                != timeline_item_neighbors(&rebuilt, *item_id)
        })
        .collect::<std::collections::HashSet<_>>();
    let mut needs_reanchor = std::collections::HashSet::new();
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

fn timeline_item_neighbors(order: &[i64], item_id: i64) -> Option<(Option<i64>, Option<i64>)> {
    let index = order.iter().position(|candidate| *candidate == item_id)?;
    Some((
        index
            .checked_sub(1)
            .and_then(|index| order.get(index).copied()),
        order.get(index + 1).copied(),
    ))
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
    for (index, item_id) in order.iter().enumerate() {
        let item = chapter_video_timeline_item::Entity::find_by_id(*item_id)
            .one(db)
            .await
            .map_err(VideoDbError::Database)?
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
        let prompt = chapter_video_prompt::Entity::find_by_id(prompt_id)
            .one(db)
            .await
            .map_err(VideoDbError::Database)?
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
            let mut active: chapter_video_prompt::ActiveModel = prompt.into();
            active.shot_number = Set(segment_number);
            active.metadata_json = Set(Some(metadata));
            active.updated_at = Set(Utc::now().naive_utc());
            active.update(db).await.map_err(VideoDbError::Database)?;
        }
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
    for (index, item_id) in order.iter().enumerate() {
        let mut active: chapter_video_timeline_item::ActiveModel =
            chapter_video_timeline_item::Entity::find_by_id(*item_id)
                .one(db)
                .await
                .map_err(VideoDbError::Database)?
                .ok_or_else(|| VideoDbError::NotFound(format!("时间线项不存在: {item_id}")))?
                .into();
        active.position = Set(Some(i32::try_from(index + 1).map_err(|_| {
            VideoDbError::InvalidArgument("时间线片段数量超出范围".to_string())
        })?));
        active.updated_at = Set(now);
        active.update(db).await.map_err(VideoDbError::Database)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::models::ai::generated_video;

    fn timeline_item(
        id: i64,
        position: Option<i32>,
        status: &str,
    ) -> chapter_video_timeline_item::Model {
        let now = Utc::now().naive_utc();
        chapter_video_timeline_item::Model {
            id,
            timeline_id: 1,
            position,
            item_kind: TIMELINE_INSERTED_KIND.to_string(),
            status: status.to_string(),
            native_source_shot_id: None,
            native_segment_index_in_shot: None,
            insert_plan_prompt_id: Some(id + 100),
            workflow_run_id: Some(id + 200),
            created_at: now,
            updated_at: now,
        }
    }

    fn timeline(revision: i64) -> chapter_video_timeline::Model {
        let now = Utc::now().naive_utc();
        chapter_video_timeline::Model {
            id: 1,
            project_id: 1,
            chapter_id: 1,
            workflow_mode: "shared_segment_plan".to_string(),
            revision,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn native_key_rejects_empty_or_non_positive_identity() {
        assert!(NativeVideoTimelineKey::new("", 1).is_err());
        assert!(NativeVideoTimelineKey::new("shot", 0).is_err());
        assert_eq!(
            NativeVideoTimelineKey::new(" shot-1 ", 2)
                .unwrap()
                .source_shot_id,
            "shot-1"
        );
    }

    #[test]
    fn positioned_order_ignores_soft_deleted_items() {
        let items = [
            timeline_item(1, Some(1), TIMELINE_ACTIVE_STATUS),
            timeline_item(2, None, TIMELINE_DELETED_STATUS),
        ];
        assert_eq!(positioned_order(&items).unwrap(), vec![1]);
    }

    #[test]
    fn positioned_order_rejects_gaps_and_duplicates() {
        let items = [
            timeline_item(1, Some(1), TIMELINE_ACTIVE_STATUS),
            timeline_item(2, Some(3), TIMELINE_ACTIVE_STATUS),
        ];
        assert!(positioned_order(&items).is_err());

        let mut items = items;
        items[1].position = Some(1);
        assert!(positioned_order(&items).is_err());
    }

    #[test]
    fn native_reconcile_marks_collapsed_inserted_runs_for_reanchor() {
        let inserted_ids = [8, 9].into_iter().collect();
        let (rebuilt, needs_reanchor) =
            rebuild_native_timeline_order(&[1, 8, 3, 9, 2], vec![1, 2], &inserted_ids);

        assert_eq!(rebuilt, vec![1, 8, 9, 2]);
        assert_eq!(needs_reanchor, inserted_ids);
    }

    #[test]
    fn native_reconcile_keeps_unchanged_inserted_run_active() {
        let inserted_ids = [8, 9].into_iter().collect();
        let (rebuilt, needs_reanchor) =
            rebuild_native_timeline_order(&[1, 8, 9, 2], vec![1, 2], &inserted_ids);

        assert_eq!(rebuilt, vec![1, 8, 9, 2]);
        assert!(needs_reanchor.is_empty());
    }

    #[test]
    fn native_reconcile_never_reverses_inserted_items_when_native_order_changes() {
        let inserted_ids = [8, 9].into_iter().collect();
        let (rebuilt, needs_reanchor) =
            rebuild_native_timeline_order(&[1, 8, 2, 9, 3], vec![3, 2, 1], &inserted_ids);

        assert_eq!(rebuilt, vec![3, 2, 1, 8, 9]);
        assert_eq!(needs_reanchor, inserted_ids);
    }

    #[test]
    fn native_reconcile_never_clears_an_existing_reanchor_requirement() {
        assert_eq!(
            inserted_status_after_native_reconcile(TIMELINE_NEEDS_REANCHOR_STATUS, false),
            TIMELINE_NEEDS_REANCHOR_STATUS
        );
        assert_eq!(
            inserted_status_after_native_reconcile(TIMELINE_ACTIVE_STATUS, false),
            TIMELINE_ACTIVE_STATUS
        );
    }

    #[test]
    fn inserted_commit_identity_is_found_before_revision_checks() {
        let mut item = timeline_item(8, Some(1), TIMELINE_ACTIVE_STATUS);
        item.workflow_run_id = Some(42);

        assert_eq!(
            inserted_timeline_item_by_workflow_run(&[item], 42).map(|item| item.id),
            Some(8)
        );
        assert!(inserted_timeline_item_by_workflow_run(&[], 0).is_none());
    }

    #[test]
    fn inserted_prompt_position_updates_the_recipe_without_duplicating_identity() {
        let mut metadata = json!({
            "segment_recipe": { "segment_number": 1 }
        });

        update_inserted_prompt_display_position(&mut metadata, 3).unwrap();

        assert_eq!(metadata["segment_recipe"]["segment_number"], 3);
        assert!(update_inserted_prompt_display_position(&mut json!({}), 3).is_err());
    }

    #[test]
    fn stale_timeline_revision_is_rejected_explicitly() {
        let error = ensure_revision(&timeline(3), 2).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("timeline_revision_conflict:current=3")
        );
        assert!(error.to_string().contains("expected=2"));
    }

    #[tokio::test]
    async fn timeline_writes_reject_resetting_projects() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let project = crate::project::create_project_record(
            &db,
            crate::project::CreateProjectRecordInput {
                user_id: 1,
                title: "视频时间线写入门禁".to_string(),
                description: String::new(),
                project_type: crate::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
            },
        )
        .await
        .unwrap();
        let project_id = project.id;
        let now = Utc::now().naive_utc();
        let chapter = a4n_core::models::chapter::chapter::ActiveModel {
            project_id: Set(project.id),
            outline_id: Set(None),
            chapter_number: Set(1),
            title: Set(Some("第一章".to_string())),
            content: Set("正文".to_string()),
            word_count: Set(2),
            status: Set(a4n_core::models::chapter::chapter::STATUS_IMPORTED.to_string()),
            summary: Set(None),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let timeline =
            ensure_chapter_video_timeline(&db, project.id, chapter.id, "shared_segment_plan")
                .await
                .unwrap();

        let mut active: a4n_core::models::project::novel_project::ActiveModel = project.into();
        active.lifecycle_gate = Set(crate::generation::PROJECT_GATE_RESETTING.to_string());
        active.update(&db).await.unwrap();

        for error in [
            ensure_chapter_video_timeline(&db, project_id, chapter.id, "shared_segment_plan")
                .await
                .expect_err("重置期间不能创建时间线"),
            reconcile_native_timeline(&db, timeline.id, &[])
                .await
                .expect_err("重置期间不能同步时间线"),
        ] {
            assert!(matches!(
                error,
                VideoDbError::InvalidState { current, .. }
                    if current == crate::generation::PROJECT_GATE_RESETTING
            ));
        }
    }

    #[tokio::test]
    async fn completing_imported_inserted_video_copies_the_prompt_timeline_binding() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let project = crate::project::create_project_record(
            &db,
            crate::project::CreateProjectRecordInput {
                user_id: 1,
                title: "插入片段导入绑定".to_string(),
                description: String::new(),
                project_type: crate::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
            },
        )
        .await
        .unwrap();
        let now = Utc::now().naive_utc();
        let chapter = a4n_core::models::chapter::chapter::ActiveModel {
            project_id: Set(project.id),
            outline_id: Set(None),
            chapter_number: Set(1),
            title: Set(Some("第一章".to_string())),
            content: Set("正文".to_string()),
            word_count: Set(2),
            status: Set(a4n_core::models::chapter::chapter::STATUS_IMPORTED.to_string()),
            summary: Set(None),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let timeline =
            ensure_chapter_video_timeline(&db, project.id, chapter.id, "shared_segment_plan")
                .await
                .unwrap();
        commit_inserted_timeline_item(
            &db,
            CommitInsertedTimelineItemInput {
                timeline_id: timeline.id,
                expected_revision: timeline.revision,
                gap: VideoTimelineGap {
                    previous_item_id: None,
                    next_item_id: None,
                },
                prompt: crate::video::ChapterVideoPromptRecordFields {
                    project_id: project.id,
                    chapter_id: chapter.id,
                    style: Some("电影感".to_string()),
                    prompt_type: "video_prompt".to_string(),
                    workflow_mode: "boundary_frames".to_string(),
                    shot_number: 1,
                    title: Some("补镜".to_string()),
                    scene_summary: Some("补充人物反应".to_string()),
                    visual_prompt: "visual".to_string(),
                    motion_prompt: Some("motion".to_string()),
                    negative_prompt: Some("negative".to_string()),
                    duration_secs: 6,
                    aspect_ratio: Some("16:9".to_string()),
                    reference_image_ids_json: Some(json!([])),
                    metadata_json: Some(json!({
                        "source_prompt_type": "inserted_segment",
                        "segment_recipe": {},
                        "video_execution_identity": {
                            "contract_epoch": a4n_core::video_execution::VIDEO_EXECUTION_CONTRACT_EPOCH,
                            "source_content_sha256": "source",
                            "segment_sha256": "segment"
                        }
                    })),
                },
                workflow_run_id: 1,
            },
        )
        .await
        .unwrap();
        let prompt = chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        let video = crate::video::create_generated_video_record(
            &db,
            crate::video::CreateGeneratedVideoRecordDraftInput {
                project_id: project.id,
                chapter_id: Some(chapter.id),
                video_type: generated_video::VIDEO_TYPE_CHAPTER.to_string(),
                workflow_mode: Some("boundary_frames".to_string()),
                prompt: "external import".to_string(),
                negative_prompt: None,
                model: "external-import".to_string(),
                width: 1280,
                height: 720,
                duration_secs: 6.0,
                fps: 24,
                source_video_id: None,
                source_image_id: None,
                provider_job_id: None,
                metadata_json: Some(json!({
                    "source_prompt_type": "inserted_segment",
                    "video_execution_identity": {
                        "contract_epoch": a4n_core::video_execution::VIDEO_EXECUTION_CONTRACT_EPOCH,
                        "source_content_sha256": "source",
                        "segment_sha256": "segment"
                    }
                })),
            },
        )
        .await
        .unwrap();

        let completed = crate::video::mark_generated_video_completed_and_link_prompt_record(
            &db,
            video.id,
            "projects/imported.mp4",
            None,
            1,
            None,
            Some(prompt.id),
        )
        .await
        .unwrap();

        assert_eq!(completed.timeline_item_id, prompt.timeline_item_id);
    }
}
