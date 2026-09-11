use a4n_db_models::models::character::foreshadowing_index;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
};

#[derive(Clone, Debug, PartialEq, FromQueryResult)]
pub struct RagForeshadowingRow {
    pub id: i64,
    pub description: String,
    pub setup_chapter: i32,
    pub payoff_chapter: Option<i32>,
    pub status: String,
    pub related_characters: Option<serde_json::Value>,
    pub importance: i32,
}

#[derive(Clone, Debug, PartialEq, FromQueryResult)]
pub struct PendingForeshadowingSearchRow {
    pub id: i64,
    pub description: String,
    pub related_characters: Option<serde_json::Value>,
    pub importance: i32,
}

pub async fn pending_foreshadowing_search_rows(
    db: &impl ConnectionTrait,
    project_id: i64,
    chapter_number: Option<i32>,
    limit: u64,
) -> Result<Vec<PendingForeshadowingSearchRow>, DbErr> {
    let mut query = foreshadowing_index::Entity::find()
        .select_only()
        .column(foreshadowing_index::Column::Id)
        .column(foreshadowing_index::Column::Description)
        .column(foreshadowing_index::Column::RelatedCharacters)
        .column(foreshadowing_index::Column::Importance)
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(foreshadowing_index::Column::Status.ne("resolved"))
        .filter(foreshadowing_index::Column::Status.ne("abandoned"));
    if let Some(chapter_number) = chapter_number {
        query = query.filter(foreshadowing_index::Column::SetupChapter.lt(chapter_number));
    }
    query
        .order_by_desc(foreshadowing_index::Column::Importance)
        .order_by_desc(foreshadowing_index::Column::SetupChapter)
        .order_by_asc(foreshadowing_index::Column::Id)
        .limit(limit)
        .into_model::<PendingForeshadowingSearchRow>()
        .all(db)
        .await
}

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
) -> Result<Vec<RagForeshadowingRow>, DbErr> {
    foreshadowing_index::Entity::find()
        .select_only()
        .column(foreshadowing_index::Column::Id)
        .column(foreshadowing_index::Column::Description)
        .column(foreshadowing_index::Column::SetupChapter)
        .column(foreshadowing_index::Column::PayoffChapter)
        .column(foreshadowing_index::Column::Status)
        .column(foreshadowing_index::Column::RelatedCharacters)
        .column(foreshadowing_index::Column::Importance)
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .order_by_asc(foreshadowing_index::Column::SetupChapter)
        .order_by_asc(foreshadowing_index::Column::Id)
        .into_model::<RagForeshadowingRow>()
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
