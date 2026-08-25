use a4n_core::models::scene::{scene_occurrence, scene_profile, scene_transition};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};
use sea_orm::{FromQueryResult, QuerySelect};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, FromQueryResult)]
pub struct RagSceneProfileRow {
    pub id: i64,
    pub name: String,
    pub aliases_json: Value,
    pub scene_type: String,
    pub importance: String,
    pub base_description: String,
    pub visual_description: String,
    pub spatial_layout: String,
    pub atmosphere: String,
    pub entry_points_json: Value,
    pub rules_json: Value,
    pub current_state_json: Value,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    pub evidence_json: Value,
}

#[derive(Clone, Debug, PartialEq, FromQueryResult)]
pub struct RagSceneOccurrenceRow {
    pub id: i64,
    pub scene_profile_id: i64,
    pub chapter_number: i32,
    pub occurrence_order: i32,
    pub timeline_position: Option<String>,
    pub summary: String,
    pub mood: Option<String>,
    pub weather_or_lighting: Option<String>,
    pub characters_json: Value,
    pub objects_json: Value,
    pub key_events_json: Value,
    pub entry_state_json: Value,
    pub exit_state_json: Value,
}

#[derive(Clone, Debug, PartialEq, FromQueryResult)]
pub struct RagSceneTransitionRow {
    pub id: i64,
    pub chapter_number: i32,
    pub from_scene_profile_id: Option<i64>,
    pub to_scene_profile_id: i64,
    pub transition_order: i32,
    pub movement_type: String,
    pub route_description: String,
    pub time_cost: Option<String>,
    pub distance_hint: Option<String>,
    pub participants_json: Value,
    pub object_refs_json: Value,
}

pub async fn scene_profiles_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<RagSceneProfileRow>, DbErr> {
    scene_profile::Entity::find()
        .select_only()
        .column(scene_profile::Column::Id)
        .column(scene_profile::Column::Name)
        .column(scene_profile::Column::AliasesJson)
        .column(scene_profile::Column::SceneType)
        .column(scene_profile::Column::Importance)
        .column(scene_profile::Column::BaseDescription)
        .column(scene_profile::Column::VisualDescription)
        .column(scene_profile::Column::SpatialLayout)
        .column(scene_profile::Column::Atmosphere)
        .column(scene_profile::Column::EntryPointsJson)
        .column(scene_profile::Column::RulesJson)
        .column(scene_profile::Column::CurrentStateJson)
        .column(scene_profile::Column::FirstSeenChapter)
        .column(scene_profile::Column::LastSeenChapter)
        .column(scene_profile::Column::EvidenceJson)
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(scene_profile::Column::FirstSeenChapter)
        .order_by_asc(scene_profile::Column::Id)
        .into_model::<RagSceneProfileRow>()
        .all(db)
        .await
}

pub async fn scene_occurrences_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<RagSceneOccurrenceRow>, DbErr> {
    scene_occurrence::Entity::find()
        .select_only()
        .column(scene_occurrence::Column::Id)
        .column(scene_occurrence::Column::SceneProfileId)
        .column(scene_occurrence::Column::ChapterNumber)
        .column(scene_occurrence::Column::OccurrenceOrder)
        .column(scene_occurrence::Column::TimelinePosition)
        .column(scene_occurrence::Column::Summary)
        .column(scene_occurrence::Column::Mood)
        .column(scene_occurrence::Column::WeatherOrLighting)
        .column(scene_occurrence::Column::CharactersJson)
        .column(scene_occurrence::Column::ObjectsJson)
        .column(scene_occurrence::Column::KeyEventsJson)
        .column(scene_occurrence::Column::EntryStateJson)
        .column(scene_occurrence::Column::ExitStateJson)
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .order_by_asc(scene_occurrence::Column::ChapterNumber)
        .order_by_asc(scene_occurrence::Column::OccurrenceOrder)
        .into_model::<RagSceneOccurrenceRow>()
        .all(db)
        .await
}

pub async fn scene_transitions_for_rag_ingest(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<RagSceneTransitionRow>, DbErr> {
    scene_transition::Entity::find()
        .select_only()
        .column(scene_transition::Column::Id)
        .column(scene_transition::Column::ChapterNumber)
        .column(scene_transition::Column::FromSceneProfileId)
        .column(scene_transition::Column::ToSceneProfileId)
        .column(scene_transition::Column::TransitionOrder)
        .column(scene_transition::Column::MovementType)
        .column(scene_transition::Column::RouteDescription)
        .column(scene_transition::Column::TimeCost)
        .column(scene_transition::Column::DistanceHint)
        .column(scene_transition::Column::ParticipantsJson)
        .column(scene_transition::Column::ObjectRefsJson)
        .filter(scene_transition::Column::ProjectId.eq(project_id))
        .order_by_asc(scene_transition::Column::ChapterNumber)
        .order_by_asc(scene_transition::Column::TransitionOrder)
        .into_model::<RagSceneTransitionRow>()
        .all(db)
        .await
}
