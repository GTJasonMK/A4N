use a4n_db_models::models::{
    chapter::{chapter, chapter_outline, part_outline},
    project::novel_blueprint,
};
use a4n_domain::{
    chapter::{ChapterRecord, PartOutlineRecord},
    project::BlueprintRecord,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
};
use serde_json::Value;

use crate::outline::record_mapping::part_outline_record_from_model;
use crate::project::blueprint_record;

#[derive(Clone, Debug, PartialEq, FromQueryResult)]
pub struct RagChapterOutlineRow {
    pub id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub beats: Option<Value>,
    pub key_scenes: Option<Value>,
}

pub async fn chapters_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<ChapterRecord>, DbErr> {
    chapter::Entity::find_current()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .all(db)
        .await
        .map(|chapters| {
            chapters
                .into_iter()
                .map(crate::project::chapter_record)
                .collect()
        })
}

pub async fn chapter_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
    chapter_id: i64,
) -> Result<Option<ChapterRecord>, DbErr> {
    chapter::Entity::find_current()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Id.eq(chapter_id))
        .one(db)
        .await
        .map(|chapter| chapter.map(crate::project::chapter_record))
}

pub async fn chapter_outlines_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<RagChapterOutlineRow>, DbErr> {
    chapter_outline::Entity::find()
        .select_only()
        .column(chapter_outline::Column::Id)
        .column(chapter_outline::Column::ChapterNumber)
        .column(chapter_outline::Column::Title)
        .column(chapter_outline::Column::Summary)
        .column(chapter_outline::Column::Beats)
        .column(chapter_outline::Column::KeyScenes)
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .into_model::<RagChapterOutlineRow>()
        .all(db)
        .await
}

pub async fn part_outlines_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<PartOutlineRecord>, DbErr> {
    let models = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(db)
        .await?;
    Ok(models
        .into_iter()
        .map(part_outline_record_from_model)
        .collect())
}

pub async fn novel_blueprints_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<BlueprintRecord>, DbErr> {
    novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .all(db)
        .await
        .map(|blueprints| blueprints.into_iter().map(blueprint_record).collect())
}
