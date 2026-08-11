use a4n_core::models::character::foreshadowing_index;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

pub async fn foreshadowing_items_by_ids(
    db: &impl ConnectionTrait,
    project_id: i64,
    foreshadowing_ids: &[i64],
) -> Result<Vec<foreshadowing_index::Model>, DbErr> {
    if foreshadowing_ids.is_empty() {
        return Ok(Vec::new());
    }
    foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(foreshadowing_index::Column::Id.is_in(foreshadowing_ids.to_vec()))
        .all(db)
        .await
}

pub async fn foreshadowing_items_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<foreshadowing_index::Model>, DbErr> {
    foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .order_by_asc(foreshadowing_index::Column::SetupChapter)
        .order_by_asc(foreshadowing_index::Column::Id)
        .all(db)
        .await
}

pub async fn pending_foreshadowing_before_chapter(
    db: &impl ConnectionTrait,
    project_id: i64,
    chapter_number: i32,
    limit: u64,
) -> Result<Vec<foreshadowing_index::Model>, DbErr> {
    foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(foreshadowing_index::Column::SetupChapter.lt(chapter_number))
        .filter(foreshadowing_index::Column::Status.ne("resolved"))
        .filter(foreshadowing_index::Column::Status.ne("abandoned"))
        .order_by_desc(foreshadowing_index::Column::Importance)
        .order_by_desc(foreshadowing_index::Column::SetupChapter)
        .order_by_asc(foreshadowing_index::Column::Id)
        .limit(limit)
        .all(db)
        .await
}

pub async fn pending_foreshadowing_for_project(
    db: &impl ConnectionTrait,
    project_id: i64,
    limit: u64,
) -> Result<Vec<foreshadowing_index::Model>, DbErr> {
    foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(foreshadowing_index::Column::Status.ne("resolved"))
        .filter(foreshadowing_index::Column::Status.ne("abandoned"))
        .order_by_desc(foreshadowing_index::Column::Importance)
        .order_by_desc(foreshadowing_index::Column::SetupChapter)
        .order_by_asc(foreshadowing_index::Column::Id)
        .limit(limit)
        .all(db)
        .await
}
