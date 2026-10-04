use a4n_db_models::models::character::protagonist_profile;
use a4n_domain::character::CharacterProfileRecord;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};

use crate::character::CharacterDbError;

use super::types::CharacterProfileCurrentStateUpdate;

pub(crate) fn character_profile_record(
    model: protagonist_profile::Model,
) -> CharacterProfileRecord {
    CharacterProfileRecord {
        id: model.id,
        project_id: model.project_id,
        name: model.name,
        gender: model.gender,
        age: model.age,
        base_attributes: model.base_attributes,
        current_state: model.current_state,
        character_arc: model.character_arc,
        voice_fingerprint: model.voice_fingerprint,
        appearance: model.appearance,
        background: model.background,
        personality: model.personality,
        last_synced_chapter: model.last_synced_chapter,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

pub async fn profile_records_by_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<CharacterProfileRecord>, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    profiles_by_project(db, project_id)
        .await
        .map(|profiles| profiles.into_iter().map(character_profile_record).collect())
}

pub async fn profile_record_by_id<C>(
    db: &C,
    profile_id: i64,
) -> Result<Option<CharacterProfileRecord>, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    profile_by_id(db, profile_id)
        .await
        .map(|row| row.map(character_profile_record))
}

pub async fn profile_record_by_project_name<C>(
    db: &C,
    project_id: i64,
    name: &str,
) -> Result<Option<CharacterProfileRecord>, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    profile_by_project_name(db, project_id, name)
        .await
        .map(|row| row.map(character_profile_record))
}

pub async fn profiles_with_timeline_state_records<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<(CharacterProfileRecord, bool, bool)>, crate::character::CharacterDbError>
where
    C: ConnectionTrait,
{
    crate::character::profiles_with_timeline_state(db, project_id)
        .await
        .map(|profiles| {
            profiles
                .into_iter()
                .map(|(profile, timeline_started, editable)| {
                    (
                        character_profile_record(profile),
                        timeline_started,
                        editable,
                    )
                })
                .collect()
        })
}

pub async fn update_initial_profile_record<C>(
    db: &C,
    project_id: i64,
    profile_id: i64,
    expected_current_state: serde_json::Value,
    base_attributes: serde_json::Value,
) -> Result<CharacterProfileRecord, crate::character::CharacterDbError>
where
    C: ConnectionTrait + sea_orm::TransactionTrait,
{
    super::update_initial_profile(
        db,
        project_id,
        profile_id,
        expected_current_state,
        base_attributes,
    )
    .await
    .map(character_profile_record)
}

pub(crate) async fn referenced_profile_ids<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    profile_ids: &[i64],
) -> Result<std::collections::BTreeSet<i64>, DbErr> {
    if profile_ids.is_empty() {
        return Ok(std::collections::BTreeSet::new());
    }
    db.query_all(sea_orm::Statement::from_sql_and_values(DbBackend::Postgres, r#"
        SELECT profile.id FROM protagonist_profile profile
        WHERE profile.project_id = $1
          AND profile.id::text IN (SELECT jsonb_array_elements_text($2::jsonb))
          AND (
            EXISTS (SELECT 1 FROM source_snapshot_profile snapshot
                WHERE snapshot.root_project_id = profile.project_id AND snapshot.profile_kind = 'character'
                  AND snapshot.source_profile_id = profile.id)
            OR EXISTS (SELECT 1 FROM galgame_scene scene JOIN galgame_script game ON game.id = scene.script_id
                WHERE game.root_project_id = profile.project_id AND (
                    scene.participants_json::jsonb @> jsonb_build_array(jsonb_build_object('character_profile_id', profile.id))
                    OR EXISTS (SELECT 1 FROM galgame_line line WHERE line.scene_id = scene.id AND (
                        line.speaker_profile_id = profile.id
                        OR line.presentation_json::jsonb @> jsonb_build_object('visible_cast', jsonb_build_array(jsonb_build_object('character_profile_id', profile.id)))
                        OR line.presentation_json::jsonb @> jsonb_build_object('enter', jsonb_build_array(profile.id))
                        OR line.presentation_json::jsonb @> jsonb_build_object('exit', jsonb_build_array(profile.id))
                        OR line.presentation_json ->> 'focus_character_profile_id' = profile.id::text
                    ))
                ))
          )
    "#, [project_id.into(), serde_json::json!(profile_ids).into()])).await?
        .into_iter().map(|row| row.try_get("", "id")).collect()
}

pub(in crate::character) async fn require_profile<C>(
    db: &C,
    profile_id: i64,
) -> Result<protagonist_profile::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    profile_by_id(db, profile_id)
        .await?
        .ok_or_else(|| CharacterDbError::NotFound(format!("角色档案不存在: {profile_id}")))
}

pub(in crate::character) async fn require_profile_for_update<C>(
    db: &C,
    profile_id: i64,
) -> Result<protagonist_profile::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    let mut query = protagonist_profile::Entity::find_by_id(profile_id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    query
        .one(db)
        .await?
        .ok_or_else(|| CharacterDbError::NotFound(format!("角色档案不存在: {profile_id}")))
}

pub async fn profiles_by_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<protagonist_profile::Model>, DbErr>
where
    C: ConnectionTrait,
{
    protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(db)
        .await
}

pub async fn profile_by_id<C>(
    db: &C,
    profile_id: i64,
) -> Result<Option<protagonist_profile::Model>, DbErr>
where
    C: ConnectionTrait,
{
    protagonist_profile::Entity::find_by_id(profile_id)
        .one(db)
        .await
}

pub async fn profile_by_project_name<C>(
    db: &C,
    project_id: i64,
    name: &str,
) -> Result<Option<protagonist_profile::Model>, DbErr>
where
    C: ConnectionTrait,
{
    protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Name.eq(name))
        .one(db)
        .await
}

pub(in crate::character) async fn update_profile_current_state<C>(
    db: &C,
    profile: protagonist_profile::Model,
    input: CharacterProfileCurrentStateUpdate,
) -> Result<protagonist_profile::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    let mut active: protagonist_profile::ActiveModel = profile.into();
    active.gender = Set(input.gender);
    active.age = Set(input.age);
    active.appearance = Set(input.appearance);
    active.background = Set(input.background);
    active.personality = Set(input.personality);
    active.current_state = Set(input.current_state);
    if let Some(last_synced_chapter) = input.last_synced_chapter {
        active.last_synced_chapter = Set(last_synced_chapter);
    }
    active.updated_at = Set(chrono::Utc::now().naive_utc());
    active.update(db).await.map_err(CharacterDbError::Database)
}
