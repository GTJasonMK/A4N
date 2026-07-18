use crate::scene_profile::SceneProfileDbResult;
use a4n_core::models::scene::{
    scene_entity_state, scene_occurrence, scene_profile, scene_transition,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde_json::json;
use std::collections::BTreeSet;

async fn delete_orphan_scene_profile<C>(
    db: &C,
    project_id: i64,
    scene_profile_id: i64,
) -> SceneProfileDbResult<()>
where
    C: ConnectionTrait,
{
    let occurrence_count = scene_occurrence::Entity::find()
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .filter(scene_occurrence::Column::SceneProfileId.eq(scene_profile_id))
        .count(db)
        .await?;
    if occurrence_count > 0 {
        return Ok(());
    }
    scene_entity_state::Entity::delete_many()
        .filter(scene_entity_state::Column::ProjectId.eq(project_id))
        .filter(scene_entity_state::Column::SceneProfileId.eq(scene_profile_id))
        .exec(db)
        .await?;
    scene_transition::Entity::delete_many()
        .filter(scene_transition::Column::ProjectId.eq(project_id))
        .filter(
            scene_transition::Column::FromSceneProfileId
                .eq(scene_profile_id)
                .or(scene_transition::Column::ToSceneProfileId.eq(scene_profile_id)),
        )
        .exec(db)
        .await?;
    crate::generation::delete_generation_run_targets_for(db, "scene_profile", &[scene_profile_id])
        .await?;
    scene_profile::Entity::delete_many()
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .filter(scene_profile::Column::Id.eq(scene_profile_id))
        .exec(db)
        .await?;
    Ok(())
}

pub(in crate::scene_profile) async fn refresh_scene_profiles_after_occurrence_change<C>(
    db: &C,
    project_id: i64,
    scene_profile_ids: &BTreeSet<i64>,
) -> SceneProfileDbResult<()>
where
    C: ConnectionTrait,
{
    for scene_id in scene_profile_ids {
        if !rebuild_scene_profile_state(db, project_id, *scene_id).await? {
            delete_orphan_scene_profile(db, project_id, *scene_id).await?;
        }
    }
    Ok(())
}

async fn rebuild_scene_profile_state<C>(
    db: &C,
    project_id: i64,
    scene_profile_id: i64,
) -> SceneProfileDbResult<bool>
where
    C: ConnectionTrait,
{
    let occurrences = scene_occurrence::Entity::find()
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .filter(scene_occurrence::Column::SceneProfileId.eq(scene_profile_id))
        .order_by_asc(scene_occurrence::Column::ChapterNumber)
        .order_by_asc(scene_occurrence::Column::OccurrenceOrder)
        .all(db)
        .await?;
    if occurrences.is_empty() {
        return Ok(false);
    }
    let first = occurrences
        .iter()
        .map(|item| item.chapter_number)
        .min()
        .unwrap_or(1);
    let last = occurrences
        .iter()
        .map(|item| item.chapter_number)
        .max()
        .unwrap_or(first);
    let Some(latest) = occurrences.last() else {
        return Ok(false);
    };
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
    let Some(profile) = scene_profile::Entity::find_by_id(scene_profile_id)
        .one(db)
        .await?
    else {
        return Ok(false);
    };
    let mut active: scene_profile::ActiveModel = profile.into();
    active.first_seen_chapter = Set(first);
    active.last_seen_chapter = Set(last);
    active.current_state_json = Set(current_state);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await?;
    Ok(true)
}
