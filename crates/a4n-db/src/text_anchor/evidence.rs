use super::validation::ensure_profile_belongs_to_root;
use super::{
    TextAnchorDbResult, TextAnchorEvidenceInput, TextAnchorProfileKind, validate_anchor_ref_records,
};
use a4n_core::{
    models::{
        chapter::chapter,
        project::{profile_evidence, text_anchor},
    },
    source_spans::source_text_hash,
};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, QueryTrait, Set,
};
use std::collections::{HashMap, HashSet};

pub async fn delete_chapter_content_profile_evidence_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> TextAnchorDbResult<u64>
where
    C: ConnectionTrait,
{
    if project_id <= 0 || chapter_id <= 0 {
        return Err(super::TextAnchorDbError::InvalidArgument(
            "章节证据清理 project_id/chapter_id 必须大于 0".to_string(),
        ));
    }
    let anchor_ids = text_anchor::Entity::find()
        .select_only()
        .column(text_anchor::Column::Id)
        .filter(text_anchor::Column::ProjectId.eq(project_id))
        .filter(text_anchor::Column::SourceKind.eq(super::SOURCE_KIND_CHAPTER_CONTENT))
        .filter(text_anchor::Column::SourceId.eq(chapter_id))
        .filter(text_anchor::Column::SourceSnapshotId.is_null())
        .into_query();
    Ok(profile_evidence::Entity::delete_many()
        .filter(profile_evidence::Column::TextAnchorId.in_subquery(anchor_ids))
        .exec(db)
        .await?
        .rows_affected)
}

pub(crate) async fn delete_chapter_content_profile_evidence_records_batch<C>(
    db: &C,
    project_id: i64,
    chapter_ids: &[i64],
) -> TextAnchorDbResult<u64>
where
    C: ConnectionTrait,
{
    if chapter_ids.is_empty() {
        return Ok(0);
    }
    if project_id <= 0 || chapter_ids.iter().any(|chapter_id| *chapter_id <= 0) {
        return Err(super::TextAnchorDbError::InvalidArgument(
            "章节证据批量清理 project_id/chapter_id 必须大于 0".to_string(),
        ));
    }
    let anchor_ids = text_anchor::Entity::find()
        .select_only()
        .column(text_anchor::Column::Id)
        .filter(text_anchor::Column::ProjectId.eq(project_id))
        .filter(text_anchor::Column::SourceKind.eq(super::SOURCE_KIND_CHAPTER_CONTENT))
        .filter(text_anchor::Column::SourceId.is_in(chapter_ids.iter().copied()))
        .filter(text_anchor::Column::SourceSnapshotId.is_null())
        .into_query();
    Ok(profile_evidence::Entity::delete_many()
        .filter(profile_evidence::Column::TextAnchorId.in_subquery(anchor_ids))
        .exec(db)
        .await?
        .rows_affected)
}

pub async fn merge_profile_evidence_records_batch<C>(
    db: &C,
    root_project_id: i64,
    source_snapshot_id: Option<i64>,
    profile_kind: TextAnchorProfileKind,
    profile_refs: Vec<(i64, Vec<TextAnchorEvidenceInput>)>,
) -> TextAnchorDbResult<Vec<profile_evidence::Model>>
where
    C: ConnectionTrait,
{
    if profile_refs.is_empty() {
        return Ok(Vec::new());
    }
    let profile_ids = profile_refs
        .iter()
        .map(|(profile_id, _)| *profile_id)
        .collect::<Vec<_>>();
    if profile_ids.iter().collect::<HashSet<_>>().len() != profile_ids.len() {
        return Err(super::TextAnchorDbError::InvalidArgument(
            "档案证据批量写入包含重复 profile_id".to_string(),
        ));
    }
    validate_profiles_belong_to_root(db, root_project_id, profile_kind, &profile_ids).await?;

    let mut anchor_ids = Vec::new();
    for (_, refs) in &profile_refs {
        if refs.is_empty() {
            return Err(super::TextAnchorDbError::InvalidArgument(
                "text_anchor_ids 不能为空".to_string(),
            ));
        }
        anchor_ids.extend(refs.iter().map(|item| item.text_anchor_id));
    }
    validate_anchor_ref_records(db, root_project_id, source_snapshot_id, &[], &anchor_ids).await?;

    let profile_kind_value = profile_kind.as_str();
    let existing = profile_evidence::Entity::find()
        .select_only()
        .column(profile_evidence::Column::ProfileId)
        .column(profile_evidence::Column::TextAnchorId)
        .column(profile_evidence::Column::EvidenceRole)
        .filter(profile_evidence::Column::ProfileKind.eq(profile_kind_value))
        .filter(profile_evidence::Column::ProfileId.is_in(profile_ids.iter().copied()))
        .into_tuple::<(i64, i64, String)>()
        .all(db)
        .await?
        .into_iter()
        .collect::<HashSet<_>>();
    let now = Utc::now().naive_utc();
    let mut pending = Vec::new();
    let mut pending_keys = Vec::new();
    for (profile_id, refs) in profile_refs {
        for item in refs {
            if existing.contains(&(profile_id, item.text_anchor_id, item.evidence_role.clone())) {
                continue;
            }
            pending_keys.push((profile_id, item.text_anchor_id, item.evidence_role.clone()));
            pending.push(profile_evidence::ActiveModel {
                profile_kind: Set(profile_kind_value.to_string()),
                profile_id: Set(profile_id),
                text_anchor_id: Set(item.text_anchor_id),
                evidence_role: Set(item.evidence_role),
                created_at: Set(now),
                ..Default::default()
            });
        }
    }
    if pending.is_empty() {
        return Ok(Vec::new());
    }
    let inserted = profile_evidence::Entity::insert_many(pending)
        .exec_with_returning_many(db)
        .await?;
    if inserted.len() != pending_keys.len() {
        return Err(super::TextAnchorDbError::InvalidArgument(
            "档案证据批量写入返回行数不一致".to_string(),
        ));
    }
    let mut inserted_by_key = HashMap::<(i64, i64, String), Vec<_>>::new();
    for row in inserted {
        inserted_by_key
            .entry((
                row.profile_id,
                row.text_anchor_id,
                row.evidence_role.clone(),
            ))
            .or_default()
            .push(row);
    }
    for rows in inserted_by_key.values_mut() {
        rows.reverse();
    }
    pending_keys
        .into_iter()
        .map(|key| {
            inserted_by_key
                .get_mut(&key)
                .and_then(Vec::pop)
                .ok_or_else(|| {
                    super::TextAnchorDbError::InvalidArgument(
                        "档案证据批量写入缺少返回记录".to_string(),
                    )
                })
        })
        .collect()
}

async fn validate_profiles_belong_to_root<C>(
    db: &C,
    root_project_id: i64,
    profile_kind: TextAnchorProfileKind,
    profile_ids: &[i64],
) -> TextAnchorDbResult<()>
where
    C: ConnectionTrait,
{
    super::validation::ensure_text_root_project(db, root_project_id).await?;
    match profile_kind {
        TextAnchorProfileKind::Character => {
            let profiles = a4n_core::models::character::protagonist_profile::Entity::find()
                .select_only()
                .column(a4n_core::models::character::protagonist_profile::Column::Id)
                .column(a4n_core::models::character::protagonist_profile::Column::ProjectId)
                .filter(
                    a4n_core::models::character::protagonist_profile::Column::Id
                        .is_in(profile_ids.iter().copied()),
                )
                .into_tuple::<(i64, i64)>()
                .all(db)
                .await?
                .into_iter()
                .collect::<HashMap<_, _>>();
            for profile_id in profile_ids {
                let project_id = profiles.get(profile_id).copied().ok_or_else(|| {
                    super::TextAnchorDbError::NotFound(format!("人物档案不存在: {profile_id}"))
                })?;
                if project_id != root_project_id {
                    return Err(super::TextAnchorDbError::InvalidArgument(format!(
                        "人物档案不属于文本根项目: profile_id={profile_id}"
                    )));
                }
            }
        }
        TextAnchorProfileKind::Object => {
            let profiles = a4n_core::models::asset::object_profile::Entity::find()
                .select_only()
                .column(a4n_core::models::asset::object_profile::Column::Id)
                .column(a4n_core::models::asset::object_profile::Column::ProjectId)
                .filter(
                    a4n_core::models::asset::object_profile::Column::Id
                        .is_in(profile_ids.iter().copied()),
                )
                .into_tuple::<(i64, i64)>()
                .all(db)
                .await?
                .into_iter()
                .collect::<HashMap<_, _>>();
            for profile_id in profile_ids {
                let project_id = profiles.get(profile_id).copied().ok_or_else(|| {
                    super::TextAnchorDbError::NotFound(format!("物体档案不存在: {profile_id}"))
                })?;
                if project_id != root_project_id {
                    return Err(super::TextAnchorDbError::InvalidArgument(format!(
                        "物体档案不属于文本根项目: profile_id={profile_id}"
                    )));
                }
            }
        }
        TextAnchorProfileKind::Scene => {
            let profiles = a4n_core::models::scene::scene_profile::Entity::find()
                .select_only()
                .column(a4n_core::models::scene::scene_profile::Column::Id)
                .column(a4n_core::models::scene::scene_profile::Column::ProjectId)
                .filter(
                    a4n_core::models::scene::scene_profile::Column::Id
                        .is_in(profile_ids.iter().copied()),
                )
                .into_tuple::<(i64, i64)>()
                .all(db)
                .await?
                .into_iter()
                .collect::<HashMap<_, _>>();
            for profile_id in profile_ids {
                let project_id = profiles.get(profile_id).copied().ok_or_else(|| {
                    super::TextAnchorDbError::NotFound(format!("场景档案不存在: {profile_id}"))
                })?;
                if project_id != root_project_id {
                    return Err(super::TextAnchorDbError::InvalidArgument(format!(
                        "场景档案不属于文本根项目: profile_id={profile_id}"
                    )));
                }
            }
        }
    }
    Ok(())
}

pub async fn earliest_current_profile_evidence_chapter<C>(
    db: &C,
    root_project_id: i64,
    profile_kind: TextAnchorProfileKind,
    profile_id: i64,
) -> TextAnchorDbResult<Option<i32>>
where
    C: ConnectionTrait,
{
    ensure_profile_belongs_to_root(db, root_project_id, profile_kind, profile_id).await?;
    let evidence_anchor_ids = profile_evidence::Entity::find()
        .select_only()
        .column(profile_evidence::Column::TextAnchorId)
        .filter(profile_evidence::Column::ProfileKind.eq(profile_kind.as_str()))
        .filter(profile_evidence::Column::ProfileId.eq(profile_id))
        .filter(profile_evidence::Column::EvidenceRole.eq(super::EVIDENCE_ROLE_FIRST_SEEN))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if evidence_anchor_ids.is_empty() {
        return Ok(None);
    }

    let anchors = text_anchor::Entity::find()
        .select_only()
        .column(text_anchor::Column::SourceChapterId)
        .column(text_anchor::Column::SourceVersionHash)
        .filter(text_anchor::Column::Id.is_in(evidence_anchor_ids))
        .filter(text_anchor::Column::ProjectId.eq(root_project_id))
        .filter(text_anchor::Column::SourceKind.eq(super::SOURCE_KIND_CHAPTER_CONTENT))
        .filter(text_anchor::Column::SourceSnapshotId.is_null())
        .filter(text_anchor::Column::SourceChapterId.is_not_null())
        .into_tuple::<(Option<i64>, String)>()
        .all(db)
        .await?;
    let chapter_ids = anchors
        .iter()
        .filter_map(|(source_chapter_id, _)| *source_chapter_id)
        .collect::<HashSet<_>>();
    if chapter_ids.is_empty() {
        return Ok(None);
    }

    let chapters = chapter::Entity::find()
        .select_only()
        .column(chapter::Column::Id)
        .column(chapter::Column::ChapterNumber)
        .column(chapter::Column::Content)
        .column(chapter::Column::Status)
        .filter(chapter::Column::ProjectId.eq(root_project_id))
        .filter(chapter::Column::Id.is_in(chapter_ids))
        .into_tuple::<(i64, i32, String, String)>()
        .all(db)
        .await?
        .into_iter()
        .map(|(id, chapter_number, content, status)| (id, (chapter_number, content, status)))
        .collect::<HashMap<_, _>>();
    Ok(anchors
        .iter()
        .filter_map(|(source_chapter_id, source_version_hash)| {
            let chapter = chapters.get(&(*source_chapter_id)?)?;
            let (chapter_number, content, status) = chapter;
            (status != chapter::STATUS_DELETED
                && !content.trim().is_empty()
                && status != chapter::STATUS_GENERATING
                && source_version_hash == &source_text_hash(content))
                .then_some(*chapter_number)
        })
        .min())
}
