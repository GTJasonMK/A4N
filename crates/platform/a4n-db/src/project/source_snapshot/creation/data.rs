use a4n_db_models::models::{chapter::chapter, project::blueprint_version};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::json;

use super::rules::chapter_scope_json;
use crate::project::{
    ProjectMutationError,
    source_snapshot::{
        DerivedChapterScope,
        hashing::{hash_json, hash_text},
        profiles::load_snapshot_profiles,
        types::SourceSnapshotBuildData,
    },
};
use std::collections::{HashMap, HashSet};

pub(crate) async fn source_snapshot_version_hash<C: ConnectionTrait>(
    db: &C,
    root_project_id: i64,
    scope: DerivedChapterScope,
) -> Result<String, ProjectMutationError> {
    Ok(build_source_snapshot_data(db, root_project_id, scope)
        .await?
        .source_version_hash)
}

pub(super) async fn build_source_snapshot_data<C>(
    db: &C,
    root_project_id: i64,
    scope: DerivedChapterScope,
) -> Result<SourceSnapshotBuildData, ProjectMutationError>
where
    C: ConnectionTrait,
{
    let chapters = chapter::Entity::find_current()
        .filter(chapter::Column::ProjectId.eq(root_project_id))
        .filter(chapter::Column::ChapterNumber.gte(scope.start_chapter))
        .filter(chapter::Column::ChapterNumber.lte(scope.end_chapter))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?;
    if chapters.is_empty() {
        return Err(ProjectMutationError::InvalidArgument(
            "源文本快照范围内没有章节".to_string(),
        ));
    }
    let expected_numbers = (scope.start_chapter..=scope.end_chapter).collect::<Vec<_>>();
    let actual_numbers = chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .collect::<Vec<_>>();
    if actual_numbers != expected_numbers {
        let actual_number_set = actual_numbers.iter().copied().collect::<HashSet<_>>();
        let missing = expected_numbers
            .into_iter()
            .filter(|number| !actual_number_set.contains(number))
            .collect::<Vec<_>>();
        return Err(ProjectMutationError::InvalidArgument(format!(
            "源文本快照范围内缺少章节: {:?}",
            missing
        )));
    }

    let content_hashes = chapters
        .iter()
        .map(|chapter| (chapter.id, hash_text(&chapter.content)))
        .collect::<HashMap<_, _>>();
    let chapter_hashes_json = chapters
        .iter()
        .map(|chapter| {
            json!({
                "source_chapter_id": chapter.id,
                "chapter_number": chapter.chapter_number,
                "content_hash": content_hashes[&chapter.id],
                "analysis_hash": chapter.analysis_data.as_ref().map(hash_json),
            })
        })
        .collect::<Vec<_>>();
    let profiles = load_snapshot_profiles(db, root_project_id).await?;
    let profile_hashes_json = json!(
        profiles
            .iter()
            .map(|profile| json!({
                "kind": profile.kind,
                "id": profile.id,
                "hash": hash_json(&profile.payload),
            }))
            .collect::<Vec<_>>()
    );
    let latest_blueprint_version_id = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(root_project_id))
        .order_by_desc(blueprint_version::Column::CreatedAt)
        .order_by_desc(blueprint_version::Column::Id)
        .one(db)
        .await
        .map_err(ProjectMutationError::Database)?
        .map(|version| version.id);
    let chapter_scope_json = chapter_scope_json(scope);
    let source_version_hash = hash_json(&json!({
        "root_project_id": root_project_id,
        "chapter_scope": chapter_scope_json,
        "chapter_hashes": chapter_hashes_json,
        "profile_hashes": profile_hashes_json,
        "blueprint_version_id": latest_blueprint_version_id,
    }));

    Ok(SourceSnapshotBuildData {
        chapters,
        profiles,
        chapter_hashes_json,
        profile_hashes_json,
        latest_blueprint_version_id,
        chapter_scope_json,
        source_version_hash,
    })
}
