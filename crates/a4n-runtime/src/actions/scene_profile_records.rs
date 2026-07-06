use crate::AppState;
use crate::actions::access;
use crate::actions::dependency_conflict::{
    DownstreamConflictStrategy, delete_downstream_images_if_requested,
    resolve_downstream_id_conflict,
};
use crate::actions::profile_materials;
use a4n_core::models::scene::{scene_occurrence, scene_profile};
use a4n_services::profile_material::PROFILE_KIND_SCENE;
use a4n_services::scene::SceneProfileDraft;
use sea_orm::{
    ColumnTrait, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter, QuerySelect,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListSceneProfilesInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneProfileInput {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub scene_type: String,
    pub importance: String,
    #[serde(default)]
    pub parent_scene_id: Option<i64>,
    pub base_description: String,
    pub visual_description: String,
    pub spatial_layout: String,
    pub atmosphere: String,
    #[serde(default)]
    pub entry_points_json: Option<Value>,
    #[serde(default)]
    pub rules_json: Option<Value>,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    #[serde(default)]
    pub evidence: Vec<String>,
    pub image_prompt_seed: String,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneProfileSummary {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub normalized_name: String,
    pub aliases_json: Value,
    pub scene_type: String,
    pub importance: String,
    pub parent_scene_id: Option<i64>,
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
    pub image_prompt_seed: String,
    pub occurrence_count: u64,
    pub generated_image_count: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, FromQueryResult)]
struct SceneOccurrenceCountRow {
    scene_profile_id: i64,
    count: i64,
}

pub async fn list_scene_profiles(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<SceneProfileSummary>, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let profiles = state
        .scene_profile
        .list_by_project(project_id)
        .await
        .map_err(|err| err.to_string())?;
    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let occurrence_counts = occurrence_counts_by_profile(state, project_id, &profile_ids).await?;
    let image_counts = profile_materials::count_profile_material_images_by_profile(
        state,
        project_id,
        PROFILE_KIND_SCENE,
        &profile_ids,
    )
    .await?;
    let mut summaries = Vec::with_capacity(profiles.len());
    for profile in profiles {
        summaries.push(to_summary_with_counts(
            &profile,
            occurrence_counts.get(&profile.id).copied().unwrap_or(0),
            image_counts.get(&profile.id).copied().unwrap_or(0),
        ));
    }
    Ok(summaries)
}

pub async fn create_scene_profile(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: SceneProfileInput,
) -> Result<SceneProfileSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let profile = state
        .scene_profile
        .create_one(project_id, input.into_draft())
        .await
        .map_err(|err| err.to_string())?;
    to_summary(state, &profile).await
}

pub async fn update_scene_profile(
    state: &AppState,
    user_id: i64,
    scene_profile_id: i64,
    input: SceneProfileInput,
) -> Result<SceneProfileSummary, String> {
    let project_id = scene_profile_project_id(state, scene_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream = resolve_scene_profile_downstream_conflict(
        state,
        project_id,
        scene_profile_id,
        input.downstream_strategy,
        "修改",
    )
    .await?;
    let profile = state
        .scene_profile
        .update_one(scene_profile_id, input.into_draft())
        .await
        .map_err(|err| err.to_string())?;
    delete_downstream_images_if_requested(state, user_id, downstream).await?;
    to_summary(state, &profile).await
}

pub async fn delete_scene_profile(
    state: &AppState,
    user_id: i64,
    scene_profile_id: i64,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<(), String> {
    let project_id = scene_profile_project_id(state, scene_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream = resolve_scene_profile_downstream_conflict(
        state,
        project_id,
        scene_profile_id,
        downstream_strategy,
        "删除",
    )
    .await?;
    state
        .scene_profile
        .delete(scene_profile_id)
        .await
        .map_err(|err| err.to_string())?;
    delete_downstream_images_if_requested(state, user_id, downstream).await
}

impl SceneProfileInput {
    fn into_draft(self) -> SceneProfileDraft {
        SceneProfileDraft {
            scene_ref: self.name.clone(),
            scene_profile_id: None,
            name: self.name,
            aliases: self.aliases,
            scene_type: self.scene_type,
            importance: self.importance,
            parent_scene_id: self.parent_scene_id,
            base_description: self.base_description,
            visual_description: self.visual_description,
            spatial_layout: self.spatial_layout,
            atmosphere: self.atmosphere,
            entry_points: self
                .entry_points_json
                .unwrap_or_else(|| Value::Array(Vec::new())),
            rules: self.rules_json.unwrap_or_else(|| Value::Array(Vec::new())),
            first_seen_chapter: self.first_seen_chapter,
            last_seen_chapter: self.last_seen_chapter,
            evidence: self.evidence,
            image_prompt_seed: self.image_prompt_seed,
        }
    }
}

async fn scene_profile_project_id(state: &AppState, scene_profile_id: i64) -> Result<i64, String> {
    let profile = state
        .scene_profile
        .get(scene_profile_id)
        .await
        .map_err(|err| err.to_string())?;
    Ok(profile.project_id)
}

async fn resolve_scene_profile_downstream_conflict(
    state: &AppState,
    project_id: i64,
    scene_profile_id: i64,
    strategy: DownstreamConflictStrategy,
    operation: &str,
) -> Result<Vec<i64>, String> {
    let image_ids = profile_materials::list_profile_material_image_ids(
        state,
        project_id,
        PROFILE_KIND_SCENE,
        scene_profile_id,
    )
    .await?;
    resolve_downstream_id_conflict(
        image_ids,
        strategy,
        &format!("场景档案 #{scene_profile_id} "),
        "张素材图",
        operation,
        "源档案",
    )
}

async fn to_summary(
    state: &AppState,
    profile: &scene_profile::Model,
) -> Result<SceneProfileSummary, String> {
    let occurrence_count = occurrence_count(state, profile).await?;
    let generated_image_count = profile_materials::count_profile_material_images(
        state,
        profile.project_id,
        PROFILE_KIND_SCENE,
        profile.id,
    )
    .await?;
    Ok(to_summary_with_counts(
        profile,
        occurrence_count,
        generated_image_count,
    ))
}

fn to_summary_with_counts(
    profile: &scene_profile::Model,
    occurrence_count: u64,
    generated_image_count: i64,
) -> SceneProfileSummary {
    SceneProfileSummary {
        id: profile.id,
        project_id: profile.project_id,
        name: profile.name.clone(),
        normalized_name: profile.normalized_name.clone(),
        aliases_json: profile.aliases_json.clone(),
        scene_type: profile.scene_type.clone(),
        importance: profile.importance.clone(),
        parent_scene_id: profile.parent_scene_id,
        base_description: profile.base_description.clone(),
        visual_description: profile.visual_description.clone(),
        spatial_layout: profile.spatial_layout.clone(),
        atmosphere: profile.atmosphere.clone(),
        entry_points_json: profile.entry_points_json.clone(),
        rules_json: profile.rules_json.clone(),
        current_state_json: profile.current_state_json.clone(),
        first_seen_chapter: profile.first_seen_chapter,
        last_seen_chapter: profile.last_seen_chapter,
        evidence_json: profile.evidence_json.clone(),
        image_prompt_seed: profile.image_prompt_seed.clone(),
        occurrence_count,
        generated_image_count,
        created_at: Some(profile.created_at.to_string()),
        updated_at: Some(profile.updated_at.to_string()),
    }
}

async fn occurrence_count(state: &AppState, profile: &scene_profile::Model) -> Result<u64, String> {
    scene_occurrence::Entity::find()
        .filter(scene_occurrence::Column::ProjectId.eq(profile.project_id))
        .filter(scene_occurrence::Column::SceneProfileId.eq(profile.id))
        .count(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())
}

async fn occurrence_counts_by_profile(
    state: &AppState,
    project_id: i64,
    profile_ids: &[i64],
) -> Result<BTreeMap<i64, u64>, String> {
    if profile_ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let rows = scene_occurrence::Entity::find()
        .select_only()
        .column(scene_occurrence::Column::SceneProfileId)
        .column_as(scene_occurrence::Column::Id.count(), "count")
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .filter(scene_occurrence::Column::SceneProfileId.is_in(profile_ids.iter().copied()))
        .group_by(scene_occurrence::Column::SceneProfileId)
        .into_model::<SceneOccurrenceCountRow>()
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    Ok(rows
        .into_iter()
        .map(|row| (row.scene_profile_id, row.count.max(0) as u64))
        .collect())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn scene_profile_summary_shape_keeps_occurrence_and_image_counts() {
        let value = json!({
            "id": 1,
            "project_id": 2,
            "name": "长安东市",
            "normalized_name": "长安东市",
            "aliases_json": [],
            "scene_type": "market",
            "importance": "high",
            "parent_scene_id": null,
            "base_description": "繁华市井",
            "visual_description": "灯火与摊贩",
            "spatial_layout": "街巷纵横",
            "atmosphere": "喧闹",
            "entry_points_json": [],
            "rules_json": [],
            "current_state_json": {},
            "first_seen_chapter": 1,
            "last_seen_chapter": 3,
            "evidence_json": [],
            "image_prompt_seed": "ancient market",
            "occurrence_count": 5,
            "generated_image_count": 2,
            "created_at": "2026-01-01 00:00:00",
            "updated_at": "2026-01-01 00:00:00"
        });
        assert_eq!(value["occurrence_count"], json!(5));
        assert_eq!(value["generated_image_count"], json!(2));
    }
}
