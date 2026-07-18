use crate::galgame::{GalgameContextRecordError, GalgamePersistValidationScopeReadModel};
use a4n_core::models::{
    character::protagonist_profile,
    project::{source_snapshot_chapter, text_anchor},
};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};

pub async fn galgame_speaker_profile_records<C>(
    db: &C,
    root_project_id: i64,
) -> Result<Vec<protagonist_profile::Model>, DbErr>
where
    C: ConnectionTrait,
{
    protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(db)
        .await
}

pub async fn galgame_speaker_profile_in_root_project_record<C>(
    db: &C,
    root_project_id: i64,
    speaker_profile_id: i64,
) -> Result<protagonist_profile::Model, GalgameContextRecordError>
where
    C: ConnectionTrait,
{
    let profile = protagonist_profile::Entity::find_by_id(speaker_profile_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            GalgameContextRecordError::NotFound(format!("角色档案不存在: {speaker_profile_id}"))
        })?;
    if profile.project_id != root_project_id {
        return Err(GalgameContextRecordError::InvalidArgument(format!(
            "角色档案 {} 不属于当前 Galgame 根项目",
            profile.id
        )));
    }
    Ok(profile)
}

pub async fn galgame_text_anchor_in_source_snapshot_record<C>(
    db: &C,
    root_project_id: i64,
    source_snapshot_id: i64,
    text_anchor_id: i64,
    expected_source_snapshot_chapter_id: Option<i64>,
) -> Result<text_anchor::Model, GalgameContextRecordError>
where
    C: ConnectionTrait,
{
    let anchor = text_anchor::Entity::find_by_id(text_anchor_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            GalgameContextRecordError::NotFound(format!("文本锚点不存在: {text_anchor_id}"))
        })?;
    if anchor.project_id != root_project_id
        || anchor.source_snapshot_id != Some(source_snapshot_id)
        || anchor.source_snapshot_chapter_id.is_none()
    {
        return Err(GalgameContextRecordError::InvalidArgument(format!(
            "文本锚点 {} 不属于当前 Galgame 源快照",
            anchor.id
        )));
    }
    if let Some(expected_source_snapshot_chapter_id) = expected_source_snapshot_chapter_id
        && anchor.source_snapshot_chapter_id != Some(expected_source_snapshot_chapter_id)
    {
        return Err(GalgameContextRecordError::InvalidArgument(format!(
            "文本锚点 {} 不属于当前 Galgame 场源快照章节 {}",
            anchor.id, expected_source_snapshot_chapter_id
        )));
    }
    Ok(anchor)
}

pub async fn galgame_persist_validation_scope_read_model<C>(
    db: &C,
    source_snapshot_chapter_ids: &[i64],
    speaker_profile_ids: &[i64],
    text_anchor_ids: &[i64],
) -> Result<GalgamePersistValidationScopeReadModel, DbErr>
where
    C: ConnectionTrait,
{
    let source_snapshot_chapters = if source_snapshot_chapter_ids.is_empty() {
        Vec::new()
    } else {
        source_snapshot_chapter::Entity::find()
            .filter(source_snapshot_chapter::Column::Id.is_in(source_snapshot_chapter_ids.to_vec()))
            .all(db)
            .await?
    };
    let speaker_profiles = if speaker_profile_ids.is_empty() {
        Vec::new()
    } else {
        protagonist_profile::Entity::find()
            .filter(protagonist_profile::Column::Id.is_in(speaker_profile_ids.to_vec()))
            .all(db)
            .await?
    };
    let text_anchors = if text_anchor_ids.is_empty() {
        Vec::new()
    } else {
        text_anchor::Entity::find()
            .filter(text_anchor::Column::Id.is_in(text_anchor_ids.to_vec()))
            .all(db)
            .await?
    };
    Ok(GalgamePersistValidationScopeReadModel {
        source_snapshot_chapters,
        speaker_profiles,
        text_anchors,
    })
}
