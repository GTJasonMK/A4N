//! 章节视频时间线持久化。
//!
//! 这里是唯一负责修改章节时间线指针和 revision 的数据库边界。普通共享
//! 片段规划只通过 `reconcile_native_timeline` 同步 native 项；插入项不会
//! 写回共享 `segments` JSON。

use a4n_core::{
    models::chapter::{
        chapter_video_prompt, chapter_video_timeline, chapter_video_timeline_event,
        chapter_video_timeline_item,
    },
    video_execution::{
        NativeVideoTimelineKey, VideoTimelineGap, insert_timeline_item,
        move_inserted_timeline_item, remove_inserted_timeline_item,
    },
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait, sea_query::OnConflict,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{VideoDbError, lock_generated_media_project};

pub const TIMELINE_NATIVE_KIND: &str = "native";
pub const TIMELINE_INSERTED_KIND: &str = "inserted";
pub const TIMELINE_ACTIVE_STATUS: &str = "active";
pub const TIMELINE_DELETED_STATUS: &str = "deleted";
pub const TIMELINE_NEEDS_REANCHOR_STATUS: &str = "needs_reanchor";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeTimelineSeed {
    pub key: NativeVideoTimelineKey,
    pub segment_number: i32,
}

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
    pub provenance_json: Option<Value>,
    pub actor_user_id: Option<i64>,
}

/// 在同一事务中提交插入片段提示词和时间线项。
///
/// 草稿阶段不会触碰 canonical 表；只有这里成功后，提示词、时间线链和
/// `timeline_item_id` 才会一起可见。
pub async fn commit_inserted_timeline_item(
    db: &DatabaseConnection,
    input: CommitInsertedTimelineItemInput,
) -> Result<(), VideoDbError> {
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let timeline = lock_active_timeline_for_update(&txn, input.timeline_id).await?;
    let existing = load_items(&txn, timeline.id).await?;
    if let Some(workflow_run_id) = provenance_workflow_run_id(input.provenance_json.as_ref())
        && let Some(item) = inserted_timeline_item_by_workflow_run(&existing, workflow_run_id)
    {
        if item.status == TIMELINE_DELETED_STATUS {
            return Err(VideoDbError::InvalidState {
                current: "inserted_segment_already_deleted".to_string(),
                operation: format!("恢复插入草稿 workflow_run_id={workflow_run_id}"),
            });
        }
        if item.insert_plan_prompt_id.is_none() {
            return Err(VideoDbError::InvalidState {
                current: "inserted_timeline_item_missing_prompt".to_string(),
                operation: format!("恢复插入草稿 workflow_run_id={workflow_run_id}"),
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
        status: Set("completed".to_string()),
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

    let order = linked_order(&timeline, &existing)?;
    // 插入空项让数据库按自身主键策略分配 ID；若 gap 无效，事务会回滚。
    let item = chapter_video_timeline_item::ActiveModel {
        timeline_id: Set(timeline.id),
        item_kind: Set(TIMELINE_INSERTED_KIND.to_string()),
        status: Set(TIMELINE_ACTIVE_STATUS.to_string()),
        native_source_shot_id: Set(None),
        native_segment_index_in_shot: Set(None),
        native_segment_number: Set(None),
        insert_plan_prompt_id: Set(Some(prompt.id)),
        previous_item_id: Set(None),
        next_item_id: Set(None),
        provenance_json: Set(input.provenance_json),
        deleted_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(VideoDbError::Database)?;
    let new_order = insert_timeline_item(&order, item.id, input.gap)
        .map_err(|error| VideoDbError::InvalidArgument(error.to_string()))?;
    let revision = timeline
        .revision
        .checked_add(1)
        .ok_or_else(|| VideoDbError::InvalidArgument("时间线 revision 超出范围".to_string()))?;
    rewrite_links(
        &txn,
        &timeline,
        &new_order,
        revision,
        input.actor_user_id,
        "commit_insert",
        Some(item.id),
    )
    .await?;

    // 保留草稿 metadata，只补正式时间线身份。
    let mut metadata = prompt
        .metadata_json
        .clone()
        .ok_or_else(|| VideoDbError::InvalidState {
            current: "inserted_prompt_metadata_missing".to_string(),
            operation: format!("绑定插入片段提示词 #{}", prompt.id),
        })?;
    bind_inserted_execution_identity(&mut metadata, item.id)?;
    let mut prompt_active: chapter_video_prompt::ActiveModel = prompt.into();
    prompt_active.timeline_item_id = Set(Some(item.id));
    prompt_active.metadata_json = Set(Some(metadata));
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
        head_item_id: Set(None),
        tail_item_id: Set(None),
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

/// 读取并验证活动链表顺序；软删除项保留审计记录，但不再属于链表。
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
        .order_by_asc(chapter_video_timeline_item::Column::Id)
        .all(db)
        .await
        .map_err(VideoDbError::Database)?;
    validate_linked_items(&timeline, &items)?;
    Ok(VideoTimelineSnapshot { timeline, items })
}

pub fn active_timeline_order(snapshot: &VideoTimelineSnapshot) -> Result<Vec<i64>, VideoDbError> {
    linked_order(&snapshot.timeline, &snapshot.items)
}

pub fn inserted_timeline_item_by_workflow_run(
    items: &[chapter_video_timeline_item::Model],
    workflow_run_id: i64,
) -> Option<&chapter_video_timeline_item::Model> {
    (workflow_run_id > 0).then_some(())?;
    items.iter().find(|item| {
        item.item_kind == TIMELINE_INSERTED_KIND
            && provenance_workflow_run_id(item.provenance_json.as_ref()) == Some(workflow_run_id)
    })
}

/// 按共享规划的稳定来源键同步 native 项。
pub async fn reconcile_native_timeline(
    db: &DatabaseConnection,
    timeline_id: i64,
    seeds: &[NativeTimelineSeed],
    actor_user_id: Option<i64>,
) -> Result<VideoTimelineSnapshot, VideoDbError> {
    if timeline_id <= 0 {
        return Err(VideoDbError::InvalidArgument(
            "timeline_id 必须大于 0".to_string(),
        ));
    }
    let mut seed_keys = std::collections::HashSet::with_capacity(seeds.len());
    if seeds
        .iter()
        .any(|seed| seed.segment_number <= 0 || !seed_keys.insert(seed.key.clone()))
    {
        return Err(VideoDbError::InvalidArgument(
            "原生时间线种子必须具有正序号且稳定身份不能重复".to_string(),
        ));
    }
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let timeline = lock_active_timeline_for_update(&txn, timeline_id).await?;
    let existing = chapter_video_timeline_item::Entity::find()
        .filter(chapter_video_timeline_item::Column::TimelineId.eq(timeline_id))
        .all(&txn)
        .await
        .map_err(VideoDbError::Database)?;
    let current_order = linked_order(&timeline, &existing)?;
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

    let mut desired_native_ids = Vec::with_capacity(seeds.len());
    for seed in seeds {
        if let Some(item) = by_key.remove(&seed.key) {
            if item.native_segment_number != Some(seed.segment_number)
                || item.status != TIMELINE_ACTIVE_STATUS
                || item.deleted_at.is_some()
            {
                let mut active: chapter_video_timeline_item::ActiveModel = item.clone().into();
                active.native_segment_number = Set(Some(seed.segment_number));
                active.status = Set(TIMELINE_ACTIVE_STATUS.to_string());
                active.deleted_at = Set(None);
                active.updated_at = Set(Utc::now().naive_utc());
                active.update(&txn).await.map_err(VideoDbError::Database)?;
            }
            desired_native_ids.push(item.id);
        } else {
            let now = Utc::now().naive_utc();
            let item = chapter_video_timeline_item::ActiveModel {
                timeline_id: Set(timeline_id),
                item_kind: Set(TIMELINE_NATIVE_KIND.to_string()),
                status: Set(TIMELINE_ACTIVE_STATUS.to_string()),
                native_source_shot_id: Set(Some(seed.key.source_shot_id.clone())),
                native_segment_index_in_shot: Set(Some(seed.key.segment_index_in_shot)),
                native_segment_number: Set(Some(seed.segment_number)),
                insert_plan_prompt_id: Set(None),
                previous_item_id: Set(None),
                next_item_id: Set(None),
                provenance_json: Set(None),
                deleted_at: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&txn)
            .await
            .map_err(VideoDbError::Database)?;
            desired_native_ids.push(item.id);
        }
    }

    // 保留插入项的相对位置；没有出现在新 native 规划中的 native 项进入删除状态，
    // 不硬删除，以便历史/撤销和问题修复仍有依据。
    for item in by_key.values() {
        let mut active: chapter_video_timeline_item::ActiveModel = (*item).clone().into();
        active.status = Set(TIMELINE_DELETED_STATUS.to_string());
        active.deleted_at = Set(Some(Utc::now().naive_utc()));
        active.previous_item_id = Set(None);
        active.next_item_id = Set(None);
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(&txn).await.map_err(VideoDbError::Database)?;
    }

    let inserted_ids = current_order
        .iter()
        .copied()
        .filter(|id| {
            existing
                .iter()
                .find(|item| item.id == *id)
                .is_some_and(|item| item.item_kind == TIMELINE_INSERTED_KIND)
        })
        .collect::<Vec<_>>();
    // 以原生规划顺序为主，将插入项保留在原来相邻的 gap；无法恢复的项置于末尾并标记。
    let inserted_id_set = inserted_ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
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
        if item.status != status || item.deleted_at.is_some() {
            let mut active: chapter_video_timeline_item::ActiveModel = item.clone().into();
            active.status = Set(status.to_string());
            active.deleted_at = Set(None);
            active.updated_at = Set(Utc::now().naive_utc());
            active.update(&txn).await.map_err(VideoDbError::Database)?;
        }
    }
    let next_revision = timeline
        .revision
        .checked_add(1)
        .ok_or_else(|| VideoDbError::InvalidArgument("时间线 revision 超出范围".to_string()))?;
    rewrite_links(
        &txn,
        &timeline,
        &rebuilt,
        next_revision,
        actor_user_id,
        "reconcile_native",
        None,
    )
    .await?;
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
    actor_user_id: Option<i64>,
) -> Result<chapter_video_timeline::Model, VideoDbError> {
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
    let order = linked_order(&timeline, &existing)?;
    let new_order = move_inserted_timeline_item(&order, item_id, gap)
        .map_err(|error| VideoDbError::InvalidArgument(error.to_string()))?;
    if new_order == order && target.status == TIMELINE_ACTIVE_STATUS && target.deleted_at.is_none()
    {
        sync_inserted_prompt_shot_numbers(&txn, &order).await?;
        txn.commit().await.map_err(VideoDbError::Database)?;
        return Ok(timeline);
    }
    let revision = timeline
        .revision
        .checked_add(1)
        .ok_or_else(|| VideoDbError::InvalidArgument("时间线 revision 超出范围".to_string()))?;
    rewrite_links(
        &txn,
        &timeline,
        &new_order,
        revision,
        actor_user_id,
        "move",
        Some(item_id),
    )
    .await?;
    if target.status != TIMELINE_ACTIVE_STATUS || target.deleted_at.is_some() {
        let mut active: chapter_video_timeline_item::ActiveModel = target.clone().into();
        active.status = Set(TIMELINE_ACTIVE_STATUS.to_string());
        active.deleted_at = Set(None);
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(&txn).await.map_err(VideoDbError::Database)?;
    }
    sync_inserted_prompt_shot_numbers(&txn, &new_order).await?;
    let timeline = refreshed_timeline(&txn, timeline.id).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(timeline)
}

pub async fn delete_inserted_timeline_item(
    db: &DatabaseConnection,
    timeline_id: i64,
    expected_revision: i64,
    item_id: i64,
    actor_user_id: Option<i64>,
) -> Result<chapter_video_timeline::Model, VideoDbError> {
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let timeline = lock_timeline(&txn, timeline_id).await?;
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
    let order = linked_order(&timeline, &existing)?;
    let new_order = remove_inserted_timeline_item(&order, item_id)
        .map_err(|error| VideoDbError::InvalidArgument(error.to_string()))?;
    let mut active: chapter_video_timeline_item::ActiveModel = target.clone().into();
    active.status = Set(TIMELINE_DELETED_STATUS.to_string());
    active.deleted_at = Set(Some(Utc::now().naive_utc()));
    active.previous_item_id = Set(None);
    active.next_item_id = Set(None);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(&txn).await.map_err(VideoDbError::Database)?;
    let revision = timeline
        .revision
        .checked_add(1)
        .ok_or_else(|| VideoDbError::InvalidArgument("时间线 revision 超出范围".to_string()))?;
    rewrite_links(
        &txn,
        &timeline,
        &new_order,
        revision,
        actor_user_id,
        "delete",
        Some(item_id),
    )
    .await?;
    sync_inserted_prompt_shot_numbers(&txn, &new_order).await?;
    let timeline = refreshed_timeline(&txn, timeline.id).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(timeline)
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

async fn refreshed_timeline<C: ConnectionTrait>(
    db: &C,
    timeline_id: i64,
) -> Result<chapter_video_timeline::Model, VideoDbError> {
    chapter_video_timeline::Entity::find_by_id(timeline_id)
        .one(db)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("时间线不存在: {timeline_id}")))
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

fn linked_order(
    timeline: &chapter_video_timeline::Model,
    items: &[chapter_video_timeline_item::Model],
) -> Result<Vec<i64>, VideoDbError> {
    let mut by_id = std::collections::HashMap::new();
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
            item.native_source_shot_id.is_some()
                && item.native_segment_index_in_shot.is_some()
                && item.native_segment_number.is_some_and(|number| number > 0)
                && item.insert_plan_prompt_id.is_none()
                && item.status != TIMELINE_NEEDS_REANCHOR_STATUS
        } else {
            item.native_source_shot_id.is_none()
                && item.native_segment_index_in_shot.is_none()
                && item.native_segment_number.is_none()
                && item.insert_plan_prompt_id.is_some()
        };
        if !identity_valid {
            return Err(VideoDbError::InvalidState {
                current: "invalid_timeline_item_identity".to_string(),
                operation: format!("读取时间线项 {}", item.id),
            });
        }
        match item.status.as_str() {
            TIMELINE_DELETED_STATUS => {
                if item.deleted_at.is_none()
                    || item.previous_item_id.is_some()
                    || item.next_item_id.is_some()
                {
                    return Err(VideoDbError::InvalidState {
                        current: "invalid_deleted_timeline_item".to_string(),
                        operation: format!("读取时间线项 {}", item.id),
                    });
                }
                continue;
            }
            TIMELINE_ACTIVE_STATUS | TIMELINE_NEEDS_REANCHOR_STATUS => {
                if item.deleted_at.is_some() {
                    return Err(VideoDbError::InvalidState {
                        current: "active_timeline_item_has_deleted_at".to_string(),
                        operation: format!("读取时间线项 {}", item.id),
                    });
                }
            }
            other => {
                return Err(VideoDbError::InvalidState {
                    current: format!("unknown_timeline_item_status:{other}"),
                    operation: format!("读取时间线项 {}", item.id),
                });
            }
        }
        by_id.insert(item.id, item);
    }
    let mut result = Vec::with_capacity(by_id.len());
    let mut seen = std::collections::HashSet::with_capacity(by_id.len());
    let mut current = timeline.head_item_id;
    let mut previous = None;
    while let Some(id) = current {
        if !seen.insert(id) {
            return Err(VideoDbError::InvalidState {
                current: "timeline_cycle".to_string(),
                operation: "读取章节视频时间线".to_string(),
            });
        }
        let item = by_id
            .remove(&id)
            .ok_or_else(|| VideoDbError::InvalidState {
                current: "timeline_dangling_pointer".to_string(),
                operation: format!("时间线项 {id}"),
            })?;
        if item.previous_item_id != previous {
            return Err(VideoDbError::InvalidState {
                current: "timeline_previous_pointer_mismatch".to_string(),
                operation: format!("时间线项 {id}"),
            });
        }
        result.push(id);
        previous = Some(id);
        current = item.next_item_id;
    }
    if !by_id.is_empty() {
        return Err(VideoDbError::InvalidState {
            current: "timeline_unreachable_item".to_string(),
            operation: "读取章节视频时间线".to_string(),
        });
    }
    if result.last().copied() != timeline.tail_item_id {
        return Err(VideoDbError::InvalidState {
            current: "timeline_tail_pointer_mismatch".to_string(),
            operation: "读取章节视频时间线".to_string(),
        });
    }
    Ok(result)
}

fn provenance_workflow_run_id(provenance: Option<&Value>) -> Option<i64> {
    provenance
        .and_then(|value| value.get("workflow_run_id"))
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
}

fn bind_inserted_execution_identity(
    metadata: &mut Value,
    timeline_item_id: i64,
) -> Result<(), VideoDbError> {
    let identity = metadata
        .as_object_mut()
        .and_then(|object| object.get_mut("video_execution_identity"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            VideoDbError::InvalidArgument(
                "插入片段 metadata_json 缺少 video_execution_identity 对象".to_string(),
            )
        })?;
    identity.insert("timeline_item_id".to_string(), json!(timeline_item_id));
    identity.insert("segment_kind".to_string(), json!(TIMELINE_INSERTED_KIND));
    identity.remove("segment_number");
    Ok(())
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
    if rebuilt.iter().all(|item_id| inserted_ids.contains(item_id)) {
        needs_reanchor.extend(inserted_ids.iter().copied());
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

fn validate_linked_items(
    timeline: &chapter_video_timeline::Model,
    items: &[chapter_video_timeline_item::Model],
) -> Result<(), VideoDbError> {
    let _ = linked_order(timeline, items)?;
    Ok(())
}

async fn rewrite_links<C: ConnectionTrait>(
    db: &C,
    timeline: &chapter_video_timeline::Model,
    order: &[i64],
    revision: i64,
    actor_user_id: Option<i64>,
    operation: &str,
    event_item_id: Option<i64>,
) -> Result<(), VideoDbError> {
    let now = Utc::now().naive_utc();
    for (index, item_id) in order.iter().enumerate() {
        let mut active: chapter_video_timeline_item::ActiveModel =
            chapter_video_timeline_item::Entity::find_by_id(*item_id)
                .one(db)
                .await
                .map_err(VideoDbError::Database)?
                .ok_or_else(|| VideoDbError::NotFound(format!("时间线项不存在: {item_id}")))?
                .into();
        active.previous_item_id = Set(index.checked_sub(1).and_then(|i| order.get(i).copied()));
        active.next_item_id = Set(order.get(index + 1).copied());
        active.updated_at = Set(now);
        active.update(db).await.map_err(VideoDbError::Database)?;
    }
    let mut timeline_active: chapter_video_timeline::ActiveModel = timeline.clone().into();
    timeline_active.revision = Set(revision);
    timeline_active.head_item_id = Set(order.first().copied());
    timeline_active.tail_item_id = Set(order.last().copied());
    timeline_active.updated_at = Set(now);
    timeline_active
        .update(db)
        .await
        .map_err(VideoDbError::Database)?;
    chapter_video_timeline_event::ActiveModel {
        timeline_id: Set(timeline.id),
        revision: Set(revision),
        operation: Set(operation.to_string()),
        item_id: Set(event_item_id),
        payload_json: Set(json!({ "order": order })),
        actor_user_id: Set(actor_user_id),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(VideoDbError::Database)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timeline_item(
        id: i64,
        status: &str,
        next_item_id: Option<i64>,
    ) -> chapter_video_timeline_item::Model {
        let now = Utc::now().naive_utc();
        chapter_video_timeline_item::Model {
            id,
            timeline_id: 1,
            item_kind: TIMELINE_INSERTED_KIND.to_string(),
            status: status.to_string(),
            native_source_shot_id: None,
            native_segment_index_in_shot: None,
            native_segment_number: None,
            insert_plan_prompt_id: Some(id + 100),
            previous_item_id: None,
            next_item_id,
            provenance_json: None,
            deleted_at: (status == TIMELINE_DELETED_STATUS).then_some(now),
            created_at: now,
            updated_at: now,
        }
    }

    fn timeline(
        revision: i64,
        head_item_id: Option<i64>,
        tail_item_id: Option<i64>,
    ) -> chapter_video_timeline::Model {
        let now = Utc::now().naive_utc();
        chapter_video_timeline::Model {
            id: 1,
            project_id: 1,
            chapter_id: 1,
            workflow_mode: "shared_segment_plan".to_string(),
            revision,
            head_item_id,
            tail_item_id,
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
    fn linked_order_ignores_soft_deleted_items() {
        let timeline = timeline(2, Some(1), Some(1));
        let items = [
            timeline_item(1, TIMELINE_ACTIVE_STATUS, None),
            timeline_item(2, TIMELINE_DELETED_STATUS, None),
        ];
        assert_eq!(linked_order(&timeline, &items).unwrap(), vec![1]);
    }

    #[test]
    fn linked_order_rejects_inconsistent_previous_or_tail_pointers() {
        let invalid_previous = timeline(2, Some(1), Some(2));
        let items = [
            timeline_item(1, TIMELINE_ACTIVE_STATUS, Some(2)),
            timeline_item(2, TIMELINE_ACTIVE_STATUS, None),
        ];
        assert!(linked_order(&invalid_previous, &items).is_err());

        let mut items = items;
        items[1].previous_item_id = Some(1);
        let invalid_tail = timeline(2, Some(1), Some(1));
        assert!(linked_order(&invalid_tail, &items).is_err());
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
    fn native_reconcile_marks_every_unanchored_inserted_item() {
        let inserted_ids = [8, 9].into_iter().collect();
        let (rebuilt, needs_reanchor) =
            rebuild_native_timeline_order(&[8, 9], Vec::new(), &inserted_ids);

        assert_eq!(rebuilt, vec![8, 9]);
        assert_eq!(needs_reanchor, inserted_ids);
    }

    #[test]
    fn inserted_commit_identity_is_found_before_revision_checks() {
        let mut item = timeline_item(8, TIMELINE_ACTIVE_STATUS, None);
        item.provenance_json = Some(json!({ "workflow_run_id": 42 }));

        assert_eq!(
            inserted_timeline_item_by_workflow_run(&[item], 42).map(|item| item.id),
            Some(8)
        );
        assert!(inserted_timeline_item_by_workflow_run(&[], 0).is_none());
    }

    #[test]
    fn inserted_identity_binding_requires_the_execution_identity_object() {
        let mut metadata = json!({
            "video_execution_identity": { "segment_number": 3 }
        });
        bind_inserted_execution_identity(&mut metadata, 8).unwrap();
        assert_eq!(
            metadata["video_execution_identity"],
            json!({ "timeline_item_id": 8, "segment_kind": "inserted" })
        );

        assert!(bind_inserted_execution_identity(&mut json!({}), 8).is_err());
    }

    #[test]
    fn inserted_prompt_position_updates_the_recipe_without_duplicating_identity() {
        let mut metadata = json!({
            "segment_recipe": { "segment_number": 1 },
            "video_execution_identity": {
                "timeline_item_id": 8,
                "segment_kind": "inserted"
            }
        });

        update_inserted_prompt_display_position(&mut metadata, 3).unwrap();

        assert_eq!(metadata["segment_recipe"]["segment_number"], 3);
        assert!(
            metadata["video_execution_identity"]
                .get("segment_number")
                .is_none()
        );
        assert!(update_inserted_prompt_display_position(&mut json!({}), 3).is_err());
    }

    #[test]
    fn stale_timeline_revision_is_rejected_explicitly() {
        let error = ensure_revision(&timeline(3, None, None), 2).unwrap_err();
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
            reconcile_native_timeline(&db, timeline.id, &[], None)
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
}
