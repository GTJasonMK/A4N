use a4n_core::models::{
    chapter::{chapter, chapter_outline, part_outline},
    project::novel_blueprint,
};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};

pub async fn chapters_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<chapter::Model>, DbErr> {
    chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .all(db)
        .await
}

pub async fn chapter_outlines_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<chapter_outline::Model>, DbErr> {
    chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
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
