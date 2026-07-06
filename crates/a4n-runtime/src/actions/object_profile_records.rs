use crate::AppState;
use crate::actions::access;
use crate::actions::dependency_conflict::{
    DownstreamConflictStrategy, delete_downstream_images_if_requested,
    resolve_downstream_id_conflict,
};
use crate::actions::profile_materials;
use a4n_core::models::asset::object_profile;
use a4n_services::asset::ObjectProfileDraft;
use a4n_services::profile_material::PROFILE_KIND_OBJECT;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListObjectProfilesInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectProfileInput {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub object_type: String,
    pub importance: String,
    pub visual_description: String,
    pub material_and_state: String,
    pub story_usage: String,
    pub visual_usage: String,
    pub generation_usage: String,
    #[serde(default)]
    pub location_context: Option<String>,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    #[serde(default)]
    pub evidence: Vec<String>,
    pub image_prompt_seed: String,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectProfileSummary {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub normalized_name: String,
    pub aliases_json: Value,
    pub object_type: String,
    pub importance: String,
    pub visual_description: String,
    pub material_and_state: String,
    pub story_usage: String,
    pub visual_usage: String,
    pub generation_usage: String,
    pub location_context: Option<String>,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    pub evidence_json: Value,
    pub image_prompt_seed: String,
    pub generated_image_count: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

pub async fn list_object_profiles(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ObjectProfileSummary>, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let profiles = state
        .object_profile
        .list_by_project(project_id)
        .await
        .map_err(|err| err.to_string())?;
    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let image_counts = profile_materials::count_profile_material_images_by_profile(
        state,
        project_id,
        PROFILE_KIND_OBJECT,
        &profile_ids,
    )
    .await?;
    let mut summaries = Vec::with_capacity(profiles.len());
    for profile in profiles {
        summaries.push(to_summary_with_image_count(
            &profile,
            image_counts.get(&profile.id).copied().unwrap_or(0),
        ));
    }
    Ok(summaries)
}

pub async fn create_object_profile(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: ObjectProfileInput,
) -> Result<ObjectProfileSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let profile = state
        .object_profile
        .create_one(project_id, input.into_draft())
        .await
        .map_err(|err| err.to_string())?;
    to_summary(state, &profile).await
}

pub async fn update_object_profile(
    state: &AppState,
    user_id: i64,
    object_profile_id: i64,
    input: ObjectProfileInput,
) -> Result<ObjectProfileSummary, String> {
    let project_id = object_profile_project_id(state, object_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream = resolve_object_profile_downstream_conflict(
        state,
        project_id,
        object_profile_id,
        input.downstream_strategy,
        "修改",
    )
    .await?;
    let profile = state
        .object_profile
        .update_one(object_profile_id, input.into_draft())
        .await
        .map_err(|err| err.to_string())?;
    delete_downstream_images_if_requested(state, user_id, downstream).await?;
    to_summary(state, &profile).await
}

pub async fn delete_object_profile(
    state: &AppState,
    user_id: i64,
    object_profile_id: i64,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<(), String> {
    let project_id = object_profile_project_id(state, object_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream = resolve_object_profile_downstream_conflict(
        state,
        project_id,
        object_profile_id,
        downstream_strategy,
        "删除",
    )
    .await?;
    state
        .object_profile
        .delete(object_profile_id)
        .await
        .map_err(|err| err.to_string())?;
    delete_downstream_images_if_requested(state, user_id, downstream).await
}

impl ObjectProfileInput {
    fn into_draft(self) -> ObjectProfileDraft {
        ObjectProfileDraft {
            name: self.name,
            aliases: self.aliases,
            object_type: self.object_type,
            importance: self.importance,
            visual_description: self.visual_description,
            material_and_state: self.material_and_state,
            story_usage: self.story_usage,
            visual_usage: self.visual_usage,
            generation_usage: self.generation_usage,
            location_context: self.location_context,
            first_seen_chapter: self.first_seen_chapter,
            last_seen_chapter: self.last_seen_chapter,
            evidence: self.evidence,
            image_prompt_seed: self.image_prompt_seed,
        }
    }
}

async fn object_profile_project_id(
    state: &AppState,
    object_profile_id: i64,
) -> Result<i64, String> {
    let profile = state
        .object_profile
        .get(object_profile_id)
        .await
        .map_err(|err| err.to_string())?;
    Ok(profile.project_id)
}

async fn resolve_object_profile_downstream_conflict(
    state: &AppState,
    project_id: i64,
    object_profile_id: i64,
    strategy: DownstreamConflictStrategy,
    operation: &str,
) -> Result<Vec<i64>, String> {
    let image_ids = profile_materials::list_profile_material_image_ids(
        state,
        project_id,
        PROFILE_KIND_OBJECT,
        object_profile_id,
    )
    .await?;
    resolve_downstream_id_conflict(
        image_ids,
        strategy,
        &format!("物体档案 #{object_profile_id} "),
        "张素材图",
        operation,
        "源档案",
    )
}

async fn to_summary(
    state: &AppState,
    profile: &object_profile::Model,
) -> Result<ObjectProfileSummary, String> {
    let generated_image_count = profile_materials::count_profile_material_images(
        state,
        profile.project_id,
        PROFILE_KIND_OBJECT,
        profile.id,
    )
    .await?;
    Ok(to_summary_with_image_count(profile, generated_image_count))
}

fn to_summary_with_image_count(
    profile: &object_profile::Model,
    generated_image_count: i64,
) -> ObjectProfileSummary {
    ObjectProfileSummary {
        id: profile.id,
        project_id: profile.project_id,
        name: profile.name.clone(),
        normalized_name: profile.normalized_name.clone(),
        aliases_json: profile.aliases_json.clone(),
        object_type: profile.object_type.clone(),
        importance: profile.importance.clone(),
        visual_description: profile.visual_description.clone(),
        material_and_state: profile.material_and_state.clone(),
        story_usage: profile.story_usage.clone(),
        visual_usage: profile.visual_usage.clone(),
        generation_usage: profile.generation_usage.clone(),
        location_context: profile.location_context.clone(),
        first_seen_chapter: profile.first_seen_chapter,
        last_seen_chapter: profile.last_seen_chapter,
        evidence_json: profile.evidence_json.clone(),
        image_prompt_seed: profile.image_prompt_seed.clone(),
        generated_image_count,
        created_at: Some(profile.created_at.to_string()),
        updated_at: Some(profile.updated_at.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn object_profile_summary_shape_keeps_generated_image_count_field() {
        let value = json!({
            "id": 1,
            "project_id": 2,
            "name": "玉佩",
            "normalized_name": "玉佩",
            "aliases_json": [],
            "object_type": "prop",
            "importance": "high",
            "visual_description": "青白色玉佩",
            "material_and_state": "温润完整",
            "story_usage": "信物",
            "visual_usage": "锁定角色手中信物",
            "generation_usage": "约束玉佩外观",
            "location_context": null,
            "first_seen_chapter": 1,
            "last_seen_chapter": 3,
            "evidence_json": [],
            "image_prompt_seed": "jade pendant",
            "generated_image_count": 2,
            "created_at": "2026-01-01 00:00:00",
            "updated_at": "2026-01-01 00:00:00"
        });
        assert_eq!(value["generated_image_count"], json!(2));
    }
}
