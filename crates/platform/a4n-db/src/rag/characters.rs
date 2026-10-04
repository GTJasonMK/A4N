use a4n_db_models::models::{
    character::{attribute_event, character_state_index, protagonist_profile},
    project::{blueprint_character, blueprint_relationship},
};
use a4n_domain::character::{CharacterAttributeEventRecord, CharacterStateIndexRecord};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, FromQueryResult)]
pub struct RagBlueprintCharacterRow {
    pub id: i64,
    pub name: String,
    pub character_type: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub personality: Option<String>,
    pub background: Option<String>,
    pub motivation: Option<String>,
    pub arc_summary: Option<String>,
    pub relationship_to_protagonist: Option<String>,
    pub arc_stages: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, FromQueryResult)]
pub struct RagBlueprintRelationshipRow {
    pub id: i64,
    pub relationship_type: String,
    pub character_a: String,
    pub character_b: String,
    pub description: Option<String>,
    pub strength: Option<i32>,
    pub evolution: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, FromQueryResult)]
pub struct RagProtagonistProfileRow {
    pub id: i64,
    pub name: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub base_attributes: Value,
    pub current_state: Value,
    pub character_arc: Option<Value>,
    pub voice_fingerprint: Option<Value>,
    pub appearance: Option<String>,
    pub background: Option<String>,
    pub personality: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, FromQueryResult)]
pub struct RagCharacterStateRow {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub state_summary: String,
    pub key_attributes: Value,
    pub emotional_state: Option<String>,
    pub current_location: Option<String>,
    pub current_goal: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

impl From<&character_state_index::Model> for RagCharacterStateRow {
    fn from(model: &character_state_index::Model) -> Self {
        Self {
            id: model.id,
            profile_id: model.profile_id,
            chapter_number: model.chapter_number,
            state_summary: model.state_summary.clone(),
            key_attributes: model.key_attributes.clone(),
            emotional_state: model.emotional_state.clone(),
            current_location: model.current_location.clone(),
            current_goal: model.current_goal.clone(),
            created_at: model.created_at,
        }
    }
}

impl From<&CharacterStateIndexRecord> for RagCharacterStateRow {
    fn from(record: &CharacterStateIndexRecord) -> Self {
        Self {
            id: record.id,
            profile_id: record.profile_id,
            chapter_number: record.chapter_number,
            state_summary: record.state_summary.clone(),
            key_attributes: record.key_attributes.clone(),
            emotional_state: record.emotional_state.clone(),
            current_location: record.current_location.clone(),
            current_goal: record.current_goal.clone(),
            created_at: record.created_at,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, FromQueryResult)]
pub struct RagAttributeEventRow {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub source_kind: String,
    pub timestamp: chrono::NaiveDateTime,
    pub category: String,
    pub key: String,
    pub operation: String,
    pub change_description: String,
    pub old_value: Value,
    pub new_value: Value,
    pub cause: String,
    pub evidence: String,
}

impl From<&attribute_event::Model> for RagAttributeEventRow {
    fn from(model: &attribute_event::Model) -> Self {
        Self {
            id: model.id,
            profile_id: model.profile_id,
            chapter_number: model.chapter_number,
            source_kind: model.source_kind.clone(),
            timestamp: model.timestamp,
            category: model.category.clone(),
            key: model.key.clone(),
            operation: model.operation.clone(),
            change_description: model.change_description.clone(),
            old_value: model.old_value.clone(),
            new_value: model.new_value.clone(),
            cause: model.cause.clone(),
            evidence: model.evidence.clone(),
        }
    }
}

impl From<&CharacterAttributeEventRecord> for RagAttributeEventRow {
    fn from(record: &CharacterAttributeEventRecord) -> Self {
        Self {
            id: record.id,
            profile_id: record.profile_id,
            chapter_number: record.chapter_number,
            source_kind: record.source_kind.clone(),
            timestamp: record.timestamp,
            category: record.category.clone(),
            key: record.key.clone(),
            operation: record.operation.clone(),
            change_description: record.change_description.clone(),
            old_value: record.old_value.clone(),
            new_value: record.new_value.clone(),
            cause: record.cause.clone(),
            evidence: record.evidence.clone(),
        }
    }
}

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
) -> Result<Vec<RagBlueprintCharacterRow>, DbErr> {
    blueprint_character::Entity::find()
        .select_only()
        .column(blueprint_character::Column::Id)
        .column(blueprint_character::Column::Name)
        .column(blueprint_character::Column::CharacterType)
        .column(blueprint_character::Column::Gender)
        .column(blueprint_character::Column::Age)
        .column(blueprint_character::Column::Appearance)
        .column(blueprint_character::Column::Personality)
        .column(blueprint_character::Column::Background)
        .column(blueprint_character::Column::Motivation)
        .column(blueprint_character::Column::ArcSummary)
        .column(blueprint_character::Column::RelationshipToProtagonist)
        .column(blueprint_character::Column::ArcStages)
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_character::Column::Id)
        .into_model::<RagBlueprintCharacterRow>()
        .all(db)
        .await
}

pub async fn blueprint_relationships_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<RagBlueprintRelationshipRow>, DbErr> {
    blueprint_relationship::Entity::find()
        .select_only()
        .column(blueprint_relationship::Column::Id)
        .column(blueprint_relationship::Column::RelationshipType)
        .column(blueprint_relationship::Column::CharacterA)
        .column(blueprint_relationship::Column::CharacterB)
        .column(blueprint_relationship::Column::Description)
        .column(blueprint_relationship::Column::Strength)
        .column(blueprint_relationship::Column::Evolution)
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_relationship::Column::Id)
        .into_model::<RagBlueprintRelationshipRow>()
        .all(db)
        .await
}

pub async fn protagonist_profiles_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<RagProtagonistProfileRow>, DbErr> {
    protagonist_profile::Entity::find()
        .select_only()
        .column(protagonist_profile::Column::Id)
        .column(protagonist_profile::Column::Name)
        .column(protagonist_profile::Column::Gender)
        .column(protagonist_profile::Column::Age)
        .column(protagonist_profile::Column::BaseAttributes)
        .column(protagonist_profile::Column::CurrentState)
        .column(protagonist_profile::Column::CharacterArc)
        .column(protagonist_profile::Column::VoiceFingerprint)
        .column(protagonist_profile::Column::Appearance)
        .column(protagonist_profile::Column::Background)
        .column(protagonist_profile::Column::Personality)
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .into_model::<RagProtagonistProfileRow>()
        .all(db)
        .await
}

pub async fn character_states_for_rag_ingest(
    db: &impl ConnectionTrait,
    profile_ids: &[i64],
) -> Result<Vec<RagCharacterStateRow>, DbErr> {
    character_states_for_rag_ingest_with_bound(db, profile_ids, None).await
}

pub async fn character_states_for_rag_ingest_before_chapter(
    db: &impl ConnectionTrait,
    profile_ids: &[i64],
    chapter_number: i32,
) -> Result<Vec<RagCharacterStateRow>, DbErr> {
    character_states_for_rag_ingest_with_bound(db, profile_ids, Some(chapter_number)).await
}

async fn character_states_for_rag_ingest_with_bound(
    db: &impl ConnectionTrait,
    profile_ids: &[i64],
    chapter_number: Option<i32>,
) -> Result<Vec<RagCharacterStateRow>, DbErr> {
    if profile_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut query = character_state_index::Entity::find()
        .select_only()
        .column(character_state_index::Column::Id)
        .column(character_state_index::Column::ProfileId)
        .column(character_state_index::Column::ChapterNumber)
        .column(character_state_index::Column::StateSummary)
        .column(character_state_index::Column::KeyAttributes)
        .column(character_state_index::Column::EmotionalState)
        .column(character_state_index::Column::CurrentLocation)
        .column(character_state_index::Column::CurrentGoal)
        .column(character_state_index::Column::CreatedAt)
        .filter(character_state_index::Column::ProfileId.is_in(profile_ids.to_vec()))
        .order_by_asc(character_state_index::Column::ChapterNumber);
    if let Some(chapter_number) = chapter_number {
        query = query
            .filter(character_state_index::Column::ChapterNumber.lt(chapter_number))
            .order_by_asc(character_state_index::Column::Id);
    }
    query.into_model::<RagCharacterStateRow>().all(db).await
}

pub async fn attribute_events_for_rag_ingest(
    db: &impl ConnectionTrait,
    profile_ids: &[i64],
) -> Result<Vec<RagAttributeEventRow>, DbErr> {
    attribute_events_for_rag_ingest_with_bound(db, profile_ids, None).await
}

pub async fn attribute_events_for_rag_ingest_before_chapter(
    db: &impl ConnectionTrait,
    profile_ids: &[i64],
    chapter_number: i32,
) -> Result<Vec<RagAttributeEventRow>, DbErr> {
    attribute_events_for_rag_ingest_with_bound(db, profile_ids, Some(chapter_number)).await
}

async fn attribute_events_for_rag_ingest_with_bound(
    db: &impl ConnectionTrait,
    profile_ids: &[i64],
    chapter_number: Option<i32>,
) -> Result<Vec<RagAttributeEventRow>, DbErr> {
    if profile_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut query = attribute_event::Entity::find()
        .select_only()
        .column(attribute_event::Column::Id)
        .column(attribute_event::Column::ProfileId)
        .column(attribute_event::Column::ChapterNumber)
        .column(attribute_event::Column::SourceKind)
        .column(attribute_event::Column::Timestamp)
        .column(attribute_event::Column::Category)
        .column(attribute_event::Column::Key)
        .column(attribute_event::Column::Operation)
        .column(attribute_event::Column::ChangeDescription)
        .column(attribute_event::Column::OldValue)
        .column(attribute_event::Column::NewValue)
        .column(attribute_event::Column::Cause)
        .column(attribute_event::Column::Evidence)
        .filter(attribute_event::Column::ProfileId.is_in(profile_ids.to_vec()))
        .order_by_asc(attribute_event::Column::ChapterNumber)
        .order_by_asc(attribute_event::Column::Id);
    if let Some(chapter_number) = chapter_number {
        query = query.filter(attribute_event::Column::ChapterNumber.lt(chapter_number));
    }
    query.into_model::<RagAttributeEventRow>().all(db).await
}
