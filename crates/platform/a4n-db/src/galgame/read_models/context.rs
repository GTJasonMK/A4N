use crate::galgame::{
    GalgameContextRecordError, GalgamePersistValidationScopeReadModel,
    GalgamePersistValidationSourceSnapshotChapterRecord,
    GalgamePersistValidationSpeakerProfileRecord, GalgamePersistValidationTextAnchorRecord,
    GalgamePipelinePromptSpeakerProfileRecord,
};
use a4n_db_models::models::{
    character::protagonist_profile,
    project::{source_snapshot_chapter, text_anchor},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

pub async fn galgame_speaker_profile_records<C>(
    db: &C,
    root_project_id: i64,
) -> Result<Vec<GalgamePipelinePromptSpeakerProfileRecord>, DbErr>
where
    C: ConnectionTrait,
{
    protagonist_profile::Entity::find()
        .select_only()
        .column(protagonist_profile::Column::Id)
        .column(protagonist_profile::Column::Name)
        .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .into_model::<GalgamePipelinePromptSpeakerProfileRecord>()
        .all(db)
        .await
}

pub async fn validate_galgame_speaker_profile_in_root_project<C>(
    db: &C,
    root_project_id: i64,
    speaker_profile_id: i64,
) -> Result<(), GalgameContextRecordError>
where
    C: ConnectionTrait,
{
    let profile_project_id = protagonist_profile::Entity::find_by_id(speaker_profile_id)
        .select_only()
        .column(protagonist_profile::Column::ProjectId)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .ok_or_else(|| {
            GalgameContextRecordError::NotFound(format!("角色档案不存在: {speaker_profile_id}"))
        })?;
    if profile_project_id != root_project_id {
        return Err(GalgameContextRecordError::InvalidArgument(format!(
            "角色档案 {speaker_profile_id} 不属于当前 Galgame 根项目"
        )));
    }
    Ok(())
}

pub async fn validate_galgame_text_anchor_in_source_snapshot<C>(
    db: &C,
    root_project_id: i64,
    source_snapshot_id: i64,
    text_anchor_id: i64,
    expected_source_snapshot_chapter_id: Option<i64>,
) -> Result<(), GalgameContextRecordError>
where
    C: ConnectionTrait,
{
    let (anchor_id, project_id, anchor_snapshot_id, anchor_chapter_id) =
        text_anchor::Entity::find_by_id(text_anchor_id)
            .select_only()
            .column(text_anchor::Column::Id)
            .column(text_anchor::Column::ProjectId)
            .column(text_anchor::Column::SourceSnapshotId)
            .column(text_anchor::Column::SourceSnapshotChapterId)
            .into_tuple::<(i64, i64, Option<i64>, Option<i64>)>()
            .one(db)
            .await?
            .ok_or_else(|| {
                GalgameContextRecordError::NotFound(format!("文本锚点不存在: {text_anchor_id}"))
            })?;
    if project_id != root_project_id
        || anchor_snapshot_id != Some(source_snapshot_id)
        || anchor_chapter_id.is_none()
    {
        return Err(GalgameContextRecordError::InvalidArgument(format!(
            "文本锚点 {} 不属于当前 Galgame 源快照",
            anchor_id
        )));
    }
    if let Some(expected_source_snapshot_chapter_id) = expected_source_snapshot_chapter_id
        && anchor_chapter_id != Some(expected_source_snapshot_chapter_id)
    {
        return Err(GalgameContextRecordError::InvalidArgument(format!(
            "文本锚点 {} 不属于当前 Galgame 场源快照章节 {}",
            anchor_id, expected_source_snapshot_chapter_id
        )));
    }
    Ok(())
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
    // Validation inputs are independent; keep result extraction in the old error order.
    let (source_snapshot_chapters_result, speaker_profiles_result, text_anchors_result) = futures::join!(
        async {
            if source_snapshot_chapter_ids.is_empty() {
                Ok(Vec::new())
            } else {
                source_snapshot_chapter::Entity::find()
                    .select_only()
                    .column(source_snapshot_chapter::Column::Id)
                    .column(source_snapshot_chapter::Column::SnapshotId)
                    .filter(
                        source_snapshot_chapter::Column::Id
                            .is_in(source_snapshot_chapter_ids.to_vec()),
                    )
                    .into_model::<GalgamePersistValidationSourceSnapshotChapterRecord>()
                    .all(db)
                    .await
            }
        },
        async {
            if speaker_profile_ids.is_empty() {
                Ok(Vec::new())
            } else {
                protagonist_profile::Entity::find()
                    .select_only()
                    .column(protagonist_profile::Column::Id)
                    .column(protagonist_profile::Column::ProjectId)
                    .filter(protagonist_profile::Column::Id.is_in(speaker_profile_ids.to_vec()))
                    .into_model::<GalgamePersistValidationSpeakerProfileRecord>()
                    .all(db)
                    .await
            }
        },
        async {
            if text_anchor_ids.is_empty() {
                Ok(Vec::new())
            } else {
                text_anchor::Entity::find()
                    .select_only()
                    .column(text_anchor::Column::Id)
                    .column(text_anchor::Column::ProjectId)
                    .column(text_anchor::Column::SourceSnapshotId)
                    .column(text_anchor::Column::SourceSnapshotChapterId)
                    .filter(text_anchor::Column::Id.is_in(text_anchor_ids.to_vec()))
                    .into_model::<GalgamePersistValidationTextAnchorRecord>()
                    .all(db)
                    .await
            }
        },
    );
    let source_snapshot_chapters = source_snapshot_chapters_result?;
    let speaker_profiles = speaker_profiles_result?;
    let text_anchors = text_anchors_result?;
    Ok(GalgamePersistValidationScopeReadModel {
        source_snapshot_chapters,
        speaker_profiles,
        text_anchors,
    })
}
