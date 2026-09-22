use super::contracts::{
    ChapterObjectProfileContext, ChapterProfileEvidence, ChapterSceneProfileContext,
    ProjectChapterContextError,
};
use a4n_db_models::models::{
    asset::object_profile,
    chapter::chapter,
    character::{character_state_index, protagonist_profile, snapshot_reference},
    project::{profile_evidence, text_anchor},
    scene::{scene_occurrence, scene_profile},
};
use a4n_domain::profile_ownership::PROFILE_OWNERSHIP_USER;
use a4n_domain::source_text_hash;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
};
use std::collections::HashMap;

pub async fn object_profiles_before_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<ChapterObjectProfileContext>, ProjectChapterContextError> {
    let profiles = object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(object_profile::Column::Id)
        .all(db)
        .await?;
    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let mut evidence =
        object_evidence_before_chapter(db, project_id, chapter_number, &profile_ids).await?;
    Ok(profiles
        .into_iter()
        .filter_map(|profile| {
            if let Some(evidence) = evidence.remove(&profile.id) {
                Some(ChapterObjectProfileContext::Observed {
                    profile_id: profile.id,
                    name: profile.name,
                    evidence,
                })
            } else if profile.ownership_kind == PROFILE_OWNERSHIP_USER {
                Some(ChapterObjectProfileContext::Authored(profile))
            } else {
                None
            }
        })
        .collect())
}

pub async fn scene_profiles_before_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<ChapterSceneProfileContext>, ProjectChapterContextError> {
    let occurrences = scene_occurrence::Entity::find()
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .filter(scene_occurrence::Column::ChapterNumber.lt(chapter_number))
        .order_by_desc(scene_occurrence::Column::ChapterNumber)
        .order_by_desc(scene_occurrence::Column::OccurrenceOrder)
        .order_by_desc(scene_occurrence::Column::Id)
        .all(db)
        .await?;
    let mut latest = HashMap::new();
    for occurrence in occurrences {
        latest
            .entry(occurrence.scene_profile_id)
            .or_insert(occurrence);
    }
    let profiles = scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(scene_profile::Column::Id)
        .all(db)
        .await?;
    Ok(profiles
        .into_iter()
        .filter_map(|profile| {
            if let Some(occurrence) = latest.remove(&profile.id) {
                Some(ChapterSceneProfileContext::Observed {
                    profile_id: profile.id,
                    name: profile.name,
                    occurrence,
                })
            } else if profile.ownership_kind == PROFILE_OWNERSHIP_USER {
                Some(ChapterSceneProfileContext::Authored(profile))
            } else {
                None
            }
        })
        .collect())
}

async fn object_evidence_before_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
    profile_ids: &[i64],
) -> Result<HashMap<i64, Vec<ChapterProfileEvidence>>, ProjectChapterContextError> {
    if profile_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let chapters = chapter::Entity::find()
        .select_only()
        .column(chapter::Column::Id)
        .column(chapter::Column::ChapterNumber)
        .column(chapter::Column::Content)
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.lt(chapter_number))
        .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
        .into_tuple::<(i64, i32, String)>()
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)?;
    if chapters.is_empty() {
        return Ok(HashMap::new());
    }
    let chapter_versions = chapters
        .iter()
        .map(|(id, number, content)| (*id, (*number, source_text_hash(content))))
        .collect::<HashMap<_, _>>();
    let references = profile_evidence::Entity::find()
        .filter(profile_evidence::Column::ProfileKind.eq("object"))
        .filter(profile_evidence::Column::ProfileId.is_in(profile_ids.iter().copied()))
        .all(db)
        .await?;
    if references.is_empty() {
        return Ok(HashMap::new());
    }
    let anchors = text_anchor::Entity::find()
        .filter(
            text_anchor::Column::Id
                .is_in(references.iter().map(|reference| reference.text_anchor_id)),
        )
        .filter(text_anchor::Column::ProjectId.eq(project_id))
        .filter(text_anchor::Column::SourceKind.eq(crate::text_anchor::SOURCE_KIND_CHAPTER_CONTENT))
        .filter(text_anchor::Column::SourceChapterId.is_in(chapter_versions.keys().copied()))
        .filter(text_anchor::Column::SourceSnapshotId.is_null())
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)?;
    let current_anchors = anchors
        .into_iter()
        .filter(|anchor| {
            anchor
                .source_chapter_id
                .and_then(|chapter_id| chapter_versions.get(&chapter_id))
                .is_some_and(|(_, hash)| hash == &anchor.source_version_hash)
        })
        .map(|anchor| (anchor.id, anchor))
        .collect::<HashMap<_, _>>();
    let mut evidence: HashMap<i64, Vec<ChapterProfileEvidence>> = HashMap::new();
    for reference in references {
        if let Some(anchor) = current_anchors.get(&reference.text_anchor_id) {
            let chapter_id = anchor
                .source_chapter_id
                .expect("current chapter anchor has a chapter");
            evidence
                .entry(reference.profile_id)
                .or_default()
                .push(ChapterProfileEvidence {
                    anchor_id: anchor.id,
                    chapter_number: chapter_versions[&chapter_id].0,
                    quote: anchor.quote.clone(),
                });
        }
    }
    for entries in evidence.values_mut() {
        entries.sort_by_key(|entry| (entry.chapter_number, entry.anchor_id));
        entries.dedup_by_key(|entry| entry.anchor_id);
    }
    Ok(evidence)
}

pub async fn protagonist_profiles_for_project(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<protagonist_profile::Model>, ProjectChapterContextError> {
    protagonist_profiles_for_project_in(db, project_id).await
}

pub async fn protagonist_profiles_for_project_in<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<protagonist_profile::Model>, ProjectChapterContextError>
where
    C: ConnectionTrait,
{
    protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)
}

pub async fn character_states_for_profiles_before_chapter(
    db: &DatabaseConnection,
    profile_ids: &[i64],
    chapter_number: i32,
    limit: u64,
) -> Result<Vec<character_state_index::Model>, ProjectChapterContextError> {
    if profile_ids.is_empty() {
        return Ok(Vec::new());
    }
    character_state_index::Entity::find()
        .filter(character_state_index::Column::ProfileId.is_in(profile_ids.iter().copied()))
        .filter(character_state_index::Column::ChapterNumber.lt(chapter_number))
        .order_by_desc(character_state_index::Column::ChapterNumber)
        .limit(limit)
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)
}

pub async fn snapshot_profile_ids_for_chapter(
    db: &DatabaseConnection,
    profile_ids: &[i64],
    chapter_number: i32,
) -> Result<Vec<i64>, ProjectChapterContextError> {
    if profile_ids.is_empty() {
        return Ok(Vec::new());
    }
    snapshot_reference::Entity::find()
        .select_only()
        .column(snapshot_reference::Column::ProfileId)
        .filter(snapshot_reference::Column::ProfileId.is_in(profile_ids.iter().copied()))
        .filter(snapshot_reference::Column::ChapterNumber.eq(chapter_number))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)
}

pub async fn snapshot_references_for_profiles_chapter<C>(
    db: &C,
    profile_ids: &[i64],
    chapter_number: i32,
) -> Result<Vec<snapshot_reference::Model>, ProjectChapterContextError>
where
    C: ConnectionTrait,
{
    if profile_ids.is_empty() {
        return Ok(Vec::new());
    }
    snapshot_reference::Entity::find()
        .filter(snapshot_reference::Column::ProfileId.is_in(profile_ids.iter().copied()))
        .filter(snapshot_reference::Column::ChapterNumber.eq(chapter_number))
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)
}
