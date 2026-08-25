use super::contracts::ProjectChapterContextError;
use a4n_core::models::{
    asset::object_profile,
    chapter::chapter,
    character::{character_state_index, protagonist_profile, snapshot_reference},
    project::{profile_evidence, text_anchor},
    scene::{scene_occurrence, scene_profile},
};
use a4n_core::profile_ownership::PROFILE_OWNERSHIP_AI_SYNC;
use a4n_core::source_spans::source_text_hash;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait, FromQueryResult,
    QueryFilter, QueryOrder, QuerySelect,
};
use std::collections::HashMap;

#[derive(Debug, FromQueryResult)]
struct ChapterContentRow {
    id: i64,
    content: String,
}

#[derive(Debug, FromQueryResult)]
struct ChapterContentAnchorRow {
    id: i64,
    source_chapter_id: Option<i64>,
    source_version_hash: String,
}

pub async fn object_profiles_before_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<object_profile::Model>, ProjectChapterContextError> {
    let current_ai_profile_ids =
        current_object_profile_ids_before_chapter(db, project_id, chapter_number).await?;
    let mut available =
        Condition::any().add(object_profile::Column::OwnershipKind.ne(PROFILE_OWNERSHIP_AI_SYNC));
    if !current_ai_profile_ids.is_empty() {
        available = available.add(object_profile::Column::Id.is_in(current_ai_profile_ids));
    }
    object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.eq(project_id))
        .filter(available)
        .order_by_desc(object_profile::Column::LastSeenChapter)
        .order_by_asc(object_profile::Column::Id)
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)
}

pub async fn scene_profiles_before_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<scene_profile::Model>, ProjectChapterContextError> {
    let current_ai_profile_ids = scene_occurrence::Entity::find()
        .select_only()
        .column(scene_occurrence::Column::SceneProfileId)
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .filter(scene_occurrence::Column::ChapterNumber.lt(chapter_number))
        .distinct()
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)?;
    let mut available =
        Condition::any().add(scene_profile::Column::OwnershipKind.ne(PROFILE_OWNERSHIP_AI_SYNC));
    if !current_ai_profile_ids.is_empty() {
        available = available.add(scene_profile::Column::Id.is_in(current_ai_profile_ids));
    }
    scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .filter(available)
        .order_by_desc(scene_profile::Column::LastSeenChapter)
        .order_by_asc(scene_profile::Column::Id)
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)
}

async fn current_object_profile_ids_before_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<i64>, ProjectChapterContextError> {
    let chapters = chapter::Entity::find()
        .select_only()
        .column(chapter::Column::Id)
        .column(chapter::Column::Content)
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.lt(chapter_number))
        .into_model::<ChapterContentRow>()
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)?;
    if chapters.is_empty() {
        return Ok(Vec::new());
    }
    let chapter_versions = chapters
        .iter()
        .map(|chapter| (chapter.id, source_text_hash(&chapter.content)))
        .collect::<HashMap<_, _>>();
    let anchors = text_anchor::Entity::find()
        .select_only()
        .column(text_anchor::Column::Id)
        .column(text_anchor::Column::SourceChapterId)
        .column(text_anchor::Column::SourceVersionHash)
        .filter(text_anchor::Column::ProjectId.eq(project_id))
        .filter(text_anchor::Column::SourceKind.eq(crate::text_anchor::SOURCE_KIND_CHAPTER_CONTENT))
        .filter(text_anchor::Column::SourceChapterId.is_in(chapter_versions.keys().copied()))
        .filter(text_anchor::Column::SourceSnapshotId.is_null())
        .into_model::<ChapterContentAnchorRow>()
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)?;
    let current_anchor_ids = anchors
        .into_iter()
        .filter(|anchor| {
            anchor
                .source_chapter_id
                .and_then(|chapter_id| chapter_versions.get(&chapter_id))
                .is_some_and(|hash| hash == &anchor.source_version_hash)
        })
        .map(|anchor| anchor.id)
        .collect::<Vec<_>>();
    if current_anchor_ids.is_empty() {
        return Ok(Vec::new());
    }
    profile_evidence::Entity::find()
        .select_only()
        .column(profile_evidence::Column::ProfileId)
        .filter(profile_evidence::Column::ProfileKind.eq("object"))
        .filter(profile_evidence::Column::TextAnchorId.is_in(current_anchor_ids))
        .distinct()
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)
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

pub async fn snapshot_reference_for_profile_chapter<C>(
    db: &C,
    profile_id: i64,
    chapter_number: i32,
) -> Result<Option<snapshot_reference::Model>, ProjectChapterContextError>
where
    C: ConnectionTrait,
{
    snapshot_reference::Entity::find()
        .filter(snapshot_reference::Column::ProfileId.eq(profile_id))
        .filter(snapshot_reference::Column::ChapterNumber.eq(chapter_number))
        .one(db)
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
