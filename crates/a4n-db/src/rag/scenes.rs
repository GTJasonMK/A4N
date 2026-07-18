use a4n_core::models::scene::{scene_occurrence, scene_profile, scene_transition};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};

pub async fn scene_profiles_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<scene_profile::Model>, DbErr> {
    scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(scene_profile::Column::FirstSeenChapter)
        .order_by_asc(scene_profile::Column::Id)
        .all(db)
        .await
}

pub async fn scene_occurrences_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<scene_occurrence::Model>, DbErr> {
    scene_occurrence::Entity::find()
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .order_by_asc(scene_occurrence::Column::ChapterNumber)
        .order_by_asc(scene_occurrence::Column::OccurrenceOrder)
        .all(db)
        .await
}

pub async fn scene_transitions_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<scene_transition::Model>, DbErr> {
    scene_transition::Entity::find()
        .filter(scene_transition::Column::ProjectId.eq(project_id))
        .order_by_asc(scene_transition::Column::ChapterNumber)
        .order_by_asc(scene_transition::Column::TransitionOrder)
        .all(db)
        .await
}
