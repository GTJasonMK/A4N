use a4n_core::models::{
    character::{attribute_event, character_state_index, protagonist_profile},
    project::{blueprint_character, blueprint_relationship},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

pub async fn blueprint_characters_for_rag_search(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<String>, DbErr> {
    blueprint_character::Entity::find()
        .select_only()
        .column(blueprint_character::Column::Name)
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_character::Column::Id)
        .into_tuple::<String>()
        .all(db)
        .await
}

pub async fn blueprint_characters_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<blueprint_character::Model>, DbErr> {
    blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .all(db)
        .await
}

pub async fn blueprint_relationships_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<blueprint_relationship::Model>, DbErr> {
    blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .all(db)
        .await
}

pub async fn protagonist_profiles_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<protagonist_profile::Model>, DbErr> {
    protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .all(db)
        .await
}

pub async fn character_states_for_rag_ingest(
    db: &impl ConnectionTrait,
    profile_ids: &[i64],
) -> Result<Vec<character_state_index::Model>, DbErr> {
    character_states_for_rag_ingest_with_bound(db, profile_ids, None).await
}

pub async fn character_states_for_rag_ingest_before_chapter(
    db: &impl ConnectionTrait,
    profile_ids: &[i64],
    chapter_number: i32,
) -> Result<Vec<character_state_index::Model>, DbErr> {
    character_states_for_rag_ingest_with_bound(db, profile_ids, Some(chapter_number)).await
}

async fn character_states_for_rag_ingest_with_bound(
    db: &impl ConnectionTrait,
    profile_ids: &[i64],
    chapter_number: Option<i32>,
) -> Result<Vec<character_state_index::Model>, DbErr> {
    if profile_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut query = character_state_index::Entity::find()
        .filter(character_state_index::Column::ProfileId.is_in(profile_ids.to_vec()))
        .order_by_asc(character_state_index::Column::ChapterNumber);
    if let Some(chapter_number) = chapter_number {
        query = query
            .filter(character_state_index::Column::ChapterNumber.lt(chapter_number))
            .order_by_asc(character_state_index::Column::Id);
    }
    query.all(db).await
}

pub async fn attribute_events_for_rag_ingest(
    db: &impl ConnectionTrait,
    profile_ids: &[i64],
) -> Result<Vec<attribute_event::Model>, DbErr> {
    attribute_events_for_rag_ingest_with_bound(db, profile_ids, None).await
}

pub async fn attribute_events_for_rag_ingest_before_chapter(
    db: &impl ConnectionTrait,
    profile_ids: &[i64],
    chapter_number: i32,
) -> Result<Vec<attribute_event::Model>, DbErr> {
    attribute_events_for_rag_ingest_with_bound(db, profile_ids, Some(chapter_number)).await
}

async fn attribute_events_for_rag_ingest_with_bound(
    db: &impl ConnectionTrait,
    profile_ids: &[i64],
    chapter_number: Option<i32>,
) -> Result<Vec<attribute_event::Model>, DbErr> {
    if profile_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut query = attribute_event::Entity::find()
        .filter(attribute_event::Column::ProfileId.is_in(profile_ids.to_vec()))
        .order_by_asc(attribute_event::Column::ChapterNumber)
        .order_by_asc(attribute_event::Column::Id);
    if let Some(chapter_number) = chapter_number {
        query = query.filter(attribute_event::Column::ChapterNumber.lt(chapter_number));
    }
    query.all(db).await
}
