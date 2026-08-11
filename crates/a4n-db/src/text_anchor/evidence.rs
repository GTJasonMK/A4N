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
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use std::collections::{BTreeSet, HashMap};

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
        .filter(text_anchor::Column::ProjectId.eq(project_id))
        .filter(text_anchor::Column::SourceKind.eq(super::SOURCE_KIND_CHAPTER_CONTENT))
        .filter(text_anchor::Column::SourceId.eq(chapter_id))
        .filter(text_anchor::Column::SourceSnapshotId.is_null())
        .all(db)
        .await?
        .into_iter()
        .map(|anchor| anchor.id)
        .collect::<Vec<_>>();
    if anchor_ids.is_empty() {
        return Ok(0);
    }
    Ok(profile_evidence::Entity::delete_many()
        .filter(profile_evidence::Column::TextAnchorId.is_in(anchor_ids))
        .exec(db)
        .await?
        .rows_affected)
}

pub async fn merge_profile_evidence_records<C>(
    db: &C,
    root_project_id: i64,
    source_snapshot_id: Option<i64>,
    profile_kind: TextAnchorProfileKind,
    profile_id: i64,
    refs: Vec<TextAnchorEvidenceInput>,
) -> TextAnchorDbResult<Vec<profile_evidence::Model>>
where
    C: ConnectionTrait,
{
    let profile_kind_value = profile_kind.as_str();
    ensure_profile_belongs_to_root(db, root_project_id, profile_kind, profile_id).await?;
    let anchor_ids = refs
        .iter()
        .map(|item| item.text_anchor_id)
        .collect::<Vec<_>>();
    validate_anchor_ref_records(db, root_project_id, source_snapshot_id, &[], &anchor_ids).await?;

    let existing = profile_evidence::Entity::find()
        .filter(profile_evidence::Column::ProfileKind.eq(profile_kind_value))
        .filter(profile_evidence::Column::ProfileId.eq(profile_id))
        .all(db)
        .await?
        .into_iter()
        .map(|item| (item.text_anchor_id, item.evidence_role))
        .collect::<BTreeSet<_>>();
    let now = Utc::now().naive_utc();
    let mut inserted = Vec::new();
    for item in refs {
        if existing.contains(&(item.text_anchor_id, item.evidence_role.clone())) {
            continue;
        }
        inserted.push(
            profile_evidence::ActiveModel {
                profile_kind: Set(profile_kind_value.to_string()),
                profile_id: Set(profile_id),
                text_anchor_id: Set(item.text_anchor_id),
                evidence_role: Set(item.evidence_role),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await?,
        );
    }
    Ok(inserted)
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
        .filter(profile_evidence::Column::ProfileKind.eq(profile_kind.as_str()))
        .filter(profile_evidence::Column::ProfileId.eq(profile_id))
        .filter(profile_evidence::Column::EvidenceRole.eq(super::EVIDENCE_ROLE_FIRST_SEEN))
        .all(db)
        .await?
        .into_iter()
        .map(|evidence| evidence.text_anchor_id)
        .collect::<Vec<_>>();
    if evidence_anchor_ids.is_empty() {
        return Ok(None);
    }

    let anchors = text_anchor::Entity::find()
        .filter(text_anchor::Column::Id.is_in(evidence_anchor_ids))
        .filter(text_anchor::Column::ProjectId.eq(root_project_id))
        .filter(text_anchor::Column::SourceKind.eq(super::SOURCE_KIND_CHAPTER_CONTENT))
        .filter(text_anchor::Column::SourceSnapshotId.is_null())
        .filter(text_anchor::Column::SourceChapterId.is_not_null())
        .all(db)
        .await?;
    let chapter_ids = anchors
        .iter()
        .filter_map(|anchor| anchor.source_chapter_id)
        .collect::<BTreeSet<_>>();
    if chapter_ids.is_empty() {
        return Ok(None);
    }

    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(root_project_id))
        .filter(chapter::Column::Id.is_in(chapter_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|chapter| (chapter.id, chapter))
        .collect::<HashMap<_, _>>();
    Ok(anchors
        .iter()
        .filter_map(|anchor| {
            let chapter = chapters.get(&anchor.source_chapter_id?)?;
            (chapter.has_available_content()
                && !chapter.is_generating()
                && anchor.source_version_hash == source_text_hash(&chapter.content))
            .then_some(chapter.chapter_number)
        })
        .min())
}
