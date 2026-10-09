use std::collections::{BTreeSet, HashMap, HashSet};

use a4n_db_models::models::{
    asset::object_profile,
    chapter::chapter,
    project::{profile_evidence, text_anchor},
};
use a4n_domain::{
    profile_ownership::PROFILE_OWNERSHIP_AI_SYNC, source_text_hash,
    text_anchor::TextAnchorProfileKind,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;

pub(crate) async fn reconcile_object_profile_evidence_from_anchors<C>(
    db: &C,
    project_id: i64,
    profile_ids: &[i64],
) -> Result<(), sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    if profile_ids.is_empty() {
        return Ok(());
    }

    let requested_ids = profile_ids.iter().copied().collect::<HashSet<_>>();
    let profiles = object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.eq(project_id))
        .filter(object_profile::Column::Id.is_in(requested_ids))
        .filter(object_profile::Column::OwnershipKind.eq(PROFILE_OWNERSHIP_AI_SYNC))
        .all(db)
        .await?;
    if profiles.is_empty() {
        return Ok(());
    }

    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let references = profile_evidence::Entity::find()
        .filter(profile_evidence::Column::ProfileKind.eq(TextAnchorProfileKind::Object.as_str()))
        .filter(profile_evidence::Column::ProfileId.is_in(profile_ids.iter().copied()))
        .all(db)
        .await?;
    let anchor_ids = references
        .iter()
        .map(|reference| reference.text_anchor_id)
        .collect::<HashSet<_>>();
    let anchors = if anchor_ids.is_empty() {
        Vec::new()
    } else {
        text_anchor::Entity::find()
            .filter(text_anchor::Column::ProjectId.eq(project_id))
            .filter(text_anchor::Column::Id.is_in(anchor_ids.iter().copied()))
            .filter(
                text_anchor::Column::SourceKind.eq(crate::text_anchor::SOURCE_KIND_CHAPTER_CONTENT),
            )
            .filter(text_anchor::Column::SourceSnapshotId.is_null())
            .filter(text_anchor::Column::SourceChapterId.is_not_null())
            .all(db)
            .await?
    };
    let chapter_ids = anchors
        .iter()
        .filter_map(|anchor| anchor.source_chapter_id)
        .collect::<HashSet<_>>();
    let chapters = if chapter_ids.is_empty() {
        Vec::new()
    } else {
        chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(project_id))
            .filter(chapter::Column::Id.is_in(chapter_ids))
            .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
            .filter(chapter::Column::Status.ne(chapter::STATUS_GENERATING))
            .all(db)
            .await?
    };
    let current_chapters = chapters
        .into_iter()
        .filter(|chapter| !chapter.content.trim().is_empty())
        .map(|chapter| {
            (
                chapter.id,
                (chapter.chapter_number, source_text_hash(&chapter.content)),
            )
        })
        .collect::<HashMap<_, _>>();
    let current_anchors = anchors
        .into_iter()
        .filter_map(|anchor| {
            let chapter_id = anchor.source_chapter_id?;
            let (chapter_number, source_hash) = current_chapters.get(&chapter_id)?;
            (anchor.source_version_hash == *source_hash).then_some((
                anchor.id,
                (
                    *chapter_number,
                    anchor.paragraph_index,
                    anchor.start_offset,
                    anchor.quote,
                ),
            ))
        })
        .collect::<HashMap<_, _>>();

    let mut evidence_by_profile = HashMap::<i64, Vec<(i32, i32, i32, i64, String)>>::new();
    let mut seen_anchors = HashSet::new();
    for reference in references {
        if !seen_anchors.insert((reference.profile_id, reference.text_anchor_id)) {
            continue;
        }
        if let Some((chapter_number, paragraph, offset, quote)) =
            current_anchors.get(&reference.text_anchor_id)
        {
            evidence_by_profile
                .entry(reference.profile_id)
                .or_default()
                .push((
                    *chapter_number,
                    *paragraph,
                    *offset,
                    reference.text_anchor_id,
                    quote.clone(),
                ));
        }
    }

    let now = Utc::now().naive_utc();
    for profile in profiles {
        let mut evidence = evidence_by_profile.remove(&profile.id).unwrap_or_default();
        evidence.sort_by_key(|(chapter, paragraph, offset, anchor_id, _)| {
            (*chapter, *paragraph, *offset, *anchor_id)
        });
        let first_seen_chapter = evidence.first().map_or(0, |item| item.0);
        let last_seen_chapter = evidence.last().map_or(0, |item| item.0);
        let quotes = evidence
            .into_iter()
            .map(|item| item.4)
            .collect::<BTreeSet<_>>();

        let mut active: object_profile::ActiveModel = profile.into();
        active.first_seen_chapter = Set(first_seen_chapter);
        active.last_seen_chapter = Set(last_seen_chapter);
        active.evidence_json = Set(json!(quotes));
        active.updated_at = Set(now);
        active.update(db).await?;
    }
    Ok(())
}
