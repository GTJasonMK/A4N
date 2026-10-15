use super::validation::ensure_profiles_belong_to_root;
use super::{TextAnchorDbResult, validate_anchor_ref_records};
use a4n_db_models::models::{
    chapter::chapter,
    project::{profile_evidence, text_anchor},
};
use a4n_domain::{
    source_text_hash,
    text_anchor::{TextAnchorEvidenceInput, TextAnchorProfileKind},
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
    delete_chapter_content_profile_evidence_records_batch(db, project_id, &[chapter_id]).await
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
    let affected_object_profile_ids = profile_evidence::Entity::find()
        .select_only()
        .column(profile_evidence::Column::ProfileId)
        .filter(profile_evidence::Column::ProfileKind.eq(TextAnchorProfileKind::Object.as_str()))
        .filter(
            profile_evidence::Column::TextAnchorId.in_subquery(
                text_anchor::Entity::find()
                    .select_only()
                    .column(text_anchor::Column::Id)
                    .filter(text_anchor::Column::ProjectId.eq(project_id))
                    .filter(text_anchor::Column::SourceKind.eq(super::SOURCE_KIND_CHAPTER_CONTENT))
                    .filter(text_anchor::Column::SourceId.is_in(chapter_ids.iter().copied()))
                    .filter(text_anchor::Column::SourceSnapshotId.is_null())
                    .into_query(),
            ),
        )
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let anchor_ids = text_anchor::Entity::find()
        .select_only()
        .column(text_anchor::Column::Id)
        .filter(text_anchor::Column::ProjectId.eq(project_id))
        .filter(text_anchor::Column::SourceKind.eq(super::SOURCE_KIND_CHAPTER_CONTENT))
        .filter(text_anchor::Column::SourceId.is_in(chapter_ids.iter().copied()))
        .filter(text_anchor::Column::SourceSnapshotId.is_null())
        .into_query();
    let deleted = profile_evidence::Entity::delete_many()
        .filter(profile_evidence::Column::TextAnchorId.in_subquery(anchor_ids))
        .exec(db)
        .await?
        .rows_affected;
    crate::asset::reconcile_object_profile_evidence_from_anchors(
        db,
        project_id,
        &affected_object_profile_ids,
    )
    .await?;
    Ok(deleted)
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
            let profiles = a4n_db_models::models::character::protagonist_profile::Entity::find()
                .select_only()
                .column(a4n_db_models::models::character::protagonist_profile::Column::Id)
                .column(a4n_db_models::models::character::protagonist_profile::Column::ProjectId)
                .filter(
                    a4n_db_models::models::character::protagonist_profile::Column::Id
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
            let profiles = a4n_db_models::models::asset::object_profile::Entity::find()
                .select_only()
                .column(a4n_db_models::models::asset::object_profile::Column::Id)
                .column(a4n_db_models::models::asset::object_profile::Column::ProjectId)
                .filter(
                    a4n_db_models::models::asset::object_profile::Column::Id
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
            let profiles = a4n_db_models::models::scene::scene_profile::Entity::find()
                .select_only()
                .column(a4n_db_models::models::scene::scene_profile::Column::Id)
                .column(a4n_db_models::models::scene::scene_profile::Column::ProjectId)
                .filter(
                    a4n_db_models::models::scene::scene_profile::Column::Id
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
    Ok(
        earliest_current_profile_evidence_chapters(
            db,
            root_project_id,
            profile_kind,
            &[profile_id],
        )
        .await?
        .get(&profile_id)
        .copied()
        .flatten(),
    )
}

pub(crate) async fn earliest_current_profile_evidence_chapters<C>(
    db: &C,
    root_project_id: i64,
    profile_kind: TextAnchorProfileKind,
    profile_ids: &[i64],
) -> TextAnchorDbResult<HashMap<i64, Option<i32>>>
where
    C: ConnectionTrait,
{
    ensure_profiles_belong_to_root(db, root_project_id, profile_kind, profile_ids).await?;
    let mut earliest_by_profile = profile_ids
        .iter()
        .copied()
        .map(|profile_id| (profile_id, None))
        .collect::<HashMap<_, Option<i32>>>();
    if profile_ids.is_empty() {
        return Ok(earliest_by_profile);
    }

    let evidence_anchor_ids = profile_evidence::Entity::find()
        .select_only()
        .column(profile_evidence::Column::ProfileId)
        .column(profile_evidence::Column::TextAnchorId)
        .filter(profile_evidence::Column::ProfileKind.eq(profile_kind.as_str()))
        .filter(profile_evidence::Column::ProfileId.is_in(profile_ids.iter().copied()))
        .filter(profile_evidence::Column::EvidenceRole.eq(super::EVIDENCE_ROLE_FIRST_SEEN))
        .into_tuple::<(i64, i64)>()
        .all(db)
        .await?;
    if evidence_anchor_ids.is_empty() {
        return Ok(earliest_by_profile);
    }

    let anchor_ids = evidence_anchor_ids
        .iter()
        .map(|(_, anchor_id)| *anchor_id)
        .collect::<HashSet<_>>();
    let anchors = text_anchor::Entity::find()
        .select_only()
        .column(text_anchor::Column::Id)
        .column(text_anchor::Column::SourceChapterId)
        .column(text_anchor::Column::SourceVersionHash)
        .filter(text_anchor::Column::Id.is_in(anchor_ids))
        .filter(text_anchor::Column::ProjectId.eq(root_project_id))
        .filter(text_anchor::Column::SourceKind.eq(super::SOURCE_KIND_CHAPTER_CONTENT))
        .filter(text_anchor::Column::SourceSnapshotId.is_null())
        .filter(text_anchor::Column::SourceChapterId.is_not_null())
        .into_tuple::<(i64, Option<i64>, String)>()
        .all(db)
        .await?;
    let chapter_ids = anchors
        .iter()
        .filter_map(|(_, source_chapter_id, _)| *source_chapter_id)
        .collect::<HashSet<_>>();
    if chapter_ids.is_empty() {
        return Ok(earliest_by_profile);
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
        .map(|(id, chapter_number, content, status)| {
            let source_version_hash = (status != chapter::STATUS_DELETED
                && status != chapter::STATUS_GENERATING
                && !content.trim().is_empty())
            .then(|| source_text_hash(&content));
            (id, (chapter_number, source_version_hash))
        })
        .collect::<HashMap<_, _>>();
    let anchors_by_id = anchors
        .into_iter()
        .map(|(id, source_chapter_id, source_version_hash)| {
            (id, (source_chapter_id, source_version_hash))
        })
        .collect::<HashMap<_, _>>();
    for (profile_id, anchor_id) in evidence_anchor_ids {
        let Some((Some(chapter_id), source_version_hash)) = anchors_by_id.get(&anchor_id) else {
            continue;
        };
        let Some((chapter_number, Some(current_hash))) = chapters.get(chapter_id) else {
            continue;
        };
        if source_version_hash == current_hash {
            earliest_by_profile
                .entry(profile_id)
                .and_modify(|earliest| {
                    *earliest = Some(
                        earliest.map_or(*chapter_number, |current| current.min(*chapter_number)),
                    )
                });
        }
    }
    Ok(earliest_by_profile)
}
