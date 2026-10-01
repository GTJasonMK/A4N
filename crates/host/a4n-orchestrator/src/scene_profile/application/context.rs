use super::types::{
    ChapterAnchorMode, SceneProfileSyncRequestContext, SceneProfileSyncStableContext,
};
use crate::OrchestratorContext;
use a4n_db_models::models::asset::object_profile;
use a4n_db_models::models::chapter::chapter;
use a4n_db_models::models::character::protagonist_profile;
use a4n_db_models::models::project::text_anchor;
use a4n_domain::scene_profile::SceneProfileRecord;
use a4n_error::AppError;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(super) async fn load_scene_profile_sync_stable_context(
    state: &OrchestratorContext,
    project_id: i64,
) -> Result<SceneProfileSyncStableContext, AppError> {
    let (object_profiles, character_profiles) = futures::join!(
        state.object_profiles_by_project(project_id),
        state.protagonist_profiles_for_project(project_id),
    );
    let object_profiles = Arc::new(object_profiles?);
    let character_profiles = Arc::new(character_profiles?);
    let object_ids = Arc::new(object_profiles.iter().map(|profile| profile.id).collect());
    let character_ids = Arc::new(
        character_profiles
            .iter()
            .map(|profile| profile.id)
            .collect(),
    );
    let object_names = Arc::new(
        object_profiles
            .iter()
            .map(|profile| (profile.id, profile.name.clone()))
            .collect(),
    );
    let character_names = Arc::new(
        character_profiles
            .iter()
            .map(|profile| (profile.id, profile.name.clone()))
            .collect(),
    );

    Ok(SceneProfileSyncStableContext {
        character_profiles,
        object_profiles,
        character_ids,
        object_ids,
        character_names,
        object_names,
    })
}

pub(super) async fn build_scene_profile_sync_request(
    state: &OrchestratorContext,
    project_id: i64,
    chapter: chapter::Model,
    text_anchors: Vec<text_anchor::Model>,
    stable: &SceneProfileSyncStableContext,
) -> Result<SceneProfileSyncRequestContext, AppError> {
    let existing_scene_profiles = state.scene_profiles_by_project(project_id).await?;
    Ok(build_scene_profile_sync_request_with_profiles(
        chapter,
        text_anchors,
        stable,
        Arc::new(existing_scene_profiles),
    ))
}

pub(super) fn build_scene_profile_sync_request_with_profiles(
    chapter: chapter::Model,
    text_anchors: Vec<text_anchor::Model>,
    stable: &SceneProfileSyncStableContext,
    existing_scene_profiles: Arc<Vec<SceneProfileRecord>>,
) -> SceneProfileSyncRequestContext {
    let existing_scene_ids = existing_scene_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<BTreeSet<_>>();
    SceneProfileSyncRequestContext {
        chapter,
        text_anchors,
        existing_scene_profiles,
        character_profiles: Arc::clone(&stable.character_profiles),
        object_profiles: Arc::clone(&stable.object_profiles),
        existing_scene_ids,
        character_ids: Arc::clone(&stable.character_ids),
        object_ids: Arc::clone(&stable.object_ids),
        character_names: Arc::clone(&stable.character_names),
        object_names: Arc::clone(&stable.object_names),
    }
}

pub(super) async fn load_scene_profile_sync_anchor_context(
    state: &OrchestratorContext,
    project_id: i64,
    chapters: &[chapter::Model],
    anchor_mode: ChapterAnchorMode,
) -> Result<BTreeMap<i64, Vec<text_anchor::Model>>, AppError> {
    let chapter_ids = chapters
        .iter()
        .map(|chapter| chapter.id)
        .collect::<Vec<_>>();
    let groups = match anchor_mode {
        ChapterAnchorMode::EnsureCurrent => {
            state
                .ensure_chapter_content_anchor_records_batch(project_id, &chapter_ids)
                .await?
        }
        ChapterAnchorMode::RequireExistingCurrent => {
            state
                .require_current_chapter_content_anchors_for_preview_batch(project_id, &chapter_ids)
                .await?
        }
    };
    Ok(chapter_ids.into_iter().zip(groups).collect())
}

pub(super) async fn select_scene_profile_sync_chapters(
    state: &OrchestratorContext,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<Vec<chapter::Model>, AppError> {
    if project_id <= 0 {
        return Err(AppError::InvalidArgument(
            "场景档案同步 project_id 必须大于 0".to_string(),
        ));
    }
    if chapter_number.is_some_and(|value| value <= 0) {
        return Err(AppError::InvalidArgument(
            "场景档案同步章节号必须大于 0".to_string(),
        ));
    }
    let mut chapters = state.character_sync_chapters(project_id).await?;
    if let Some(chapter_number) = chapter_number {
        chapters.retain(|chapter| chapter.chapter_number == chapter_number);
        if chapters.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "第 {chapter_number} 章没有可用于场景档案同步的有效正文"
            )));
        }
        return Ok(chapters);
    }
    if chapters.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "项目还没有可用于场景档案同步的章节: project_id={project_id}"
        )));
    }
    Ok(chapters)
}

pub(super) fn existing_scene_profile_for_prompt(profile: &SceneProfileRecord) -> Value {
    json!({"id": profile.id, "name": profile.name, "aliases": profile.aliases_json, "scene_type": profile.scene_type, "importance": profile.importance, "base_description": profile.base_description, "visual_description": profile.visual_description, "spatial_layout": profile.spatial_layout, "atmosphere": profile.atmosphere, "current_state": profile.current_state_json, "first_seen_chapter": profile.first_seen_chapter, "last_seen_chapter": profile.last_seen_chapter})
}

pub(super) fn character_profile_for_prompt(profile: &protagonist_profile::Model) -> Value {
    json!({"profile_id": profile.id, "name": profile.name, "appearance": profile.appearance})
}

pub(super) fn object_profile_for_prompt(profile: &object_profile::Model) -> Value {
    json!({"object_profile_id": profile.id, "name": profile.name, "object_type": profile.object_type, "visual_description": profile.visual_description, "material_and_state": profile.material_and_state, "location_context": profile.location_context, "last_seen_chapter": profile.last_seen_chapter})
}

pub(super) fn scene_profile_chapter_context(
    chapter: &chapter::Model,
    anchors: &[text_anchor::Model],
) -> Result<Value, AppError> {
    Ok(json!({
        "chapter_number": chapter.chapter_number,
        "chapter_title": chapter.title,
        "content": a4n_domain::source_spans::text_anchor_prompt_text(chapter, anchors).map_err(AppError::InvalidArgument)?,
    }))
}
