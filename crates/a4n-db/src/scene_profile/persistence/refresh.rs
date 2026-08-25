use crate::scene_profile::SceneProfileDbResult;
use a4n_core::models::scene::{
    scene_entity_state, scene_occurrence, scene_profile, scene_transition,
};
use a4n_core::profile_ownership::PROFILE_OWNERSHIP_AI_SYNC;
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
    sea_query::{Alias, CaseStatement, Expr},
};
use serde_json::json;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, FromQueryResult)]
struct OccurrenceRefreshRow {
    scene_profile_id: i64,
    chapter_number: i32,
    occurrence_order: i32,
    summary: String,
    mood: Option<String>,
    weather_or_lighting: Option<String>,
    characters_json: serde_json::Value,
    objects_json: serde_json::Value,
    key_events_json: serde_json::Value,
    entry_state_json: serde_json::Value,
    exit_state_json: serde_json::Value,
}

#[derive(Debug, FromQueryResult)]
struct ProfileRefreshRow {
    id: i64,
    ownership_kind: String,
    current_state_json: serde_json::Value,
}

pub(in crate::scene_profile) async fn refresh_scene_profiles_after_occurrence_change<C>(
    db: &C,
    project_id: i64,
    scene_profile_ids: &BTreeSet<i64>,
    prune_ai_orphans: bool,
) -> SceneProfileDbResult<()>
where
    C: ConnectionTrait,
{
    if scene_profile_ids.is_empty() {
        return Ok(());
    }
    let occurrences_query = scene_occurrence::Entity::find()
        .select_only()
        .column(scene_occurrence::Column::SceneProfileId)
        .column(scene_occurrence::Column::ChapterNumber)
        .column(scene_occurrence::Column::OccurrenceOrder)
        .column(scene_occurrence::Column::Summary)
        .column(scene_occurrence::Column::Mood)
        .column(scene_occurrence::Column::WeatherOrLighting)
        .column(scene_occurrence::Column::CharactersJson)
        .column(scene_occurrence::Column::ObjectsJson)
        .column(scene_occurrence::Column::KeyEventsJson)
        .column(scene_occurrence::Column::EntryStateJson)
        .column(scene_occurrence::Column::ExitStateJson)
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .filter(scene_occurrence::Column::SceneProfileId.is_in(scene_profile_ids.iter().copied()))
        .order_by_asc(scene_occurrence::Column::SceneProfileId)
        .order_by_asc(scene_occurrence::Column::ChapterNumber)
        .order_by_asc(scene_occurrence::Column::OccurrenceOrder)
        .into_model::<OccurrenceRefreshRow>()
        .all(db);
    let profiles_query = scene_profile::Entity::find()
        .select_only()
        .column(scene_profile::Column::Id)
        .column(scene_profile::Column::OwnershipKind)
        .column(scene_profile::Column::CurrentStateJson)
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .filter(scene_profile::Column::Id.is_in(scene_profile_ids.iter().copied()))
        .into_model::<ProfileRefreshRow>()
        .all(db);
    let (occurrences_result, profiles_result) = futures::join!(occurrences_query, profiles_query);
    let occurrences = occurrences_result?;
    let mut occurrences_by_profile = HashMap::<i64, Vec<OccurrenceRefreshRow>>::new();
    for occurrence in occurrences {
        occurrences_by_profile
            .entry(occurrence.scene_profile_id)
            .or_default()
            .push(occurrence);
    }
    let profiles = profiles_result?;
    let profiles_by_id = profiles
        .into_iter()
        .map(|profile| (profile.id, profile))
        .collect::<HashMap<_, _>>();

    let mut refreshed = Vec::new();
    let mut reset_orphan_ids = Vec::new();
    let mut prune_orphan_ids = Vec::new();
    for scene_id in scene_profile_ids {
        let Some(occurrences) = occurrences_by_profile.get(scene_id) else {
            let Some(profile) = profiles_by_id.get(scene_id) else {
                continue;
            };
            if prune_ai_orphans && profile.ownership_kind == PROFILE_OWNERSHIP_AI_SYNC {
                prune_orphan_ids.push(*scene_id);
            } else if profile.current_state_json != json!({}) {
                reset_orphan_ids.push(*scene_id);
            }
            continue;
        };
        let Some(profile) = profiles_by_id.get(scene_id) else {
            continue;
        };
        let first = occurrences
            .first()
            .expect("non-empty scene occurrence group");
        let latest = occurrences
            .last()
            .expect("non-empty scene occurrence group");
        let current_state = json!({
            "chapter_number": latest.chapter_number,
            "occurrence_order": latest.occurrence_order,
            "summary": latest.summary,
            "mood": latest.mood,
            "weather_or_lighting": latest.weather_or_lighting,
            "entry_state": latest.entry_state_json,
            "exit_state": latest.exit_state_json,
            "characters": latest.characters_json,
            "objects": latest.objects_json,
            "key_events": latest.key_events_json,
        });
        refreshed.push((
            profile.id,
            first.chapter_number,
            latest.chapter_number,
            current_state,
        ));
    }

    if !reset_orphan_ids.is_empty() {
        let reset_now = Utc::now().naive_utc();
        scene_profile::Entity::update_many()
            .col_expr(
                scene_profile::Column::CurrentStateJson,
                Expr::value(json!({})).cast_as(Alias::new("json")),
            )
            .col_expr(scene_profile::Column::UpdatedAt, Expr::value(reset_now))
            .filter(scene_profile::Column::ProjectId.eq(project_id))
            .filter(scene_profile::Column::Id.is_in(reset_orphan_ids))
            .exec(db)
            .await?;
    }
    if !prune_orphan_ids.is_empty() {
        scene_entity_state::Entity::delete_many()
            .filter(scene_entity_state::Column::ProjectId.eq(project_id))
            .filter(
                scene_entity_state::Column::SceneProfileId.is_in(prune_orphan_ids.iter().copied()),
            )
            .exec(db)
            .await?;
        scene_transition::Entity::delete_many()
            .filter(scene_transition::Column::ProjectId.eq(project_id))
            .filter(
                scene_transition::Column::FromSceneProfileId
                    .is_in(prune_orphan_ids.iter().copied())
                    .or(scene_transition::Column::ToSceneProfileId
                        .is_in(prune_orphan_ids.iter().copied())),
            )
            .exec(db)
            .await?;
        crate::generation::delete_generation_run_targets_for(
            db,
            "scene_profile",
            &prune_orphan_ids,
        )
        .await?;
        scene_profile::Entity::delete_many()
            .filter(scene_profile::Column::ProjectId.eq(project_id))
            .filter(scene_profile::Column::Id.is_in(prune_orphan_ids))
            .exec(db)
            .await?;
    }
    if refreshed.is_empty() {
        return Ok(());
    }

    let mut first_seen_case = CaseStatement::new();
    let mut last_seen_case = CaseStatement::new();
    let mut current_state_case = CaseStatement::new();
    for (profile_id, first, last, current_state) in &refreshed {
        first_seen_case = first_seen_case.case(
            scene_profile::Column::Id.eq(*profile_id),
            Expr::value(*first),
        );
        last_seen_case = last_seen_case.case(
            scene_profile::Column::Id.eq(*profile_id),
            Expr::value(*last),
        );
        current_state_case = current_state_case.case(
            scene_profile::Column::Id.eq(*profile_id),
            Expr::value(current_state.clone()).cast_as(Alias::new("json")),
        );
    }
    let refreshed_ids = refreshed.iter().map(|(id, ..)| *id).collect::<Vec<_>>();
    let now = Utc::now().naive_utc();
    let updated = scene_profile::Entity::update_many()
        .col_expr(
            scene_profile::Column::FirstSeenChapter,
            first_seen_case
                .finally(Expr::col(scene_profile::Column::FirstSeenChapter))
                .into(),
        )
        .col_expr(
            scene_profile::Column::LastSeenChapter,
            last_seen_case
                .finally(Expr::col(scene_profile::Column::LastSeenChapter))
                .into(),
        )
        .col_expr(
            scene_profile::Column::CurrentStateJson,
            current_state_case
                .finally(Expr::col(scene_profile::Column::CurrentStateJson))
                .into(),
        )
        .col_expr(scene_profile::Column::UpdatedAt, Expr::value(now))
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .filter(scene_profile::Column::Id.is_in(refreshed_ids))
        .exec(db)
        .await?;
    if updated.rows_affected != refreshed.len() as u64 {
        return Err(crate::scene_profile::SceneProfileDbError::InvalidState {
            current: format!(
                "场景档案刷新影响 {} 行，预期 {} 行",
                updated.rows_affected,
                refreshed.len()
            ),
            operation: "批量刷新场景档案状态".to_string(),
        });
    }
    Ok(())
}
