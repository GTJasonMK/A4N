use a4n_db_models::models::{
    chapter::{chapter, chapter_outline, part_outline},
    project::novel_blueprint,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
};
use serde_json::Value;

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
) -> Result<Vec<chapter::Model>, DbErr> {
    chapter::Entity::find_current()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .all(db)
        .await
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
) -> Result<Vec<part_outline::Model>, DbErr> {
    part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(db)
        .await
}

pub async fn novel_blueprints_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<novel_blueprint::Model>, DbErr> {
    novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .all(db)
        .await
}
