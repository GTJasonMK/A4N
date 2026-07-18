use super::types::{ChapterAnchorMode, SceneProfileSyncRequestContext};
use crate::actions::app_error_message;
use crate::state::AppState;
use a4n_core::models::asset::object_profile;
use a4n_core::models::chapter::chapter;
use a4n_core::models::character::protagonist_profile;
use a4n_core::models::project::text_anchor;
use a4n_core::models::scene::scene_profile;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) async fn build_scene_profile_sync_request(
    state: &AppState,
    project_id: i64,
    chapter: chapter::Model,
    anchor_mode: ChapterAnchorMode,
) -> Result<SceneProfileSyncRequestContext, String> {
    let existing_scene_profiles = state
        .orchestrator
        .scene_profiles_by_project(project_id)
        .await
        .map_err(|error| error.to_string())?;
    let existing_scene_ids = existing_scene_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<BTreeSet<_>>();
    let object_profiles = state
        .orchestrator
        .object_profiles_by_project(project_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    let object_ids = object_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<BTreeSet<_>>();
    let object_names = object_profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<BTreeMap<_, _>>();
    let character_profiles = state
        .orchestrator
        .protagonist_profiles_for_project(project_id)
        .await
        .map_err(app_error_message)?;
    let character_ids = character_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<BTreeSet<_>>();
    let character_names = character_profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<BTreeMap<_, _>>();
    let text_anchors = match anchor_mode {
        ChapterAnchorMode::EnsureCurrent => state
            .orchestrator
            .ensure_chapter_content_anchor_records(project_id, chapter.id)
            .await
            .map_err(crate::actions::app_error_message)?,
        ChapterAnchorMode::RequireExistingCurrent => {
            read_current_chapter_content_anchors(state, project_id, chapter.id).await?
        }
    };
    Ok(SceneProfileSyncRequestContext {
        chapter,
        text_anchors,
        existing_scene_profiles,
        character_profiles,
        object_profiles,
        existing_scene_ids,
        character_ids,
        object_ids,
        character_names,
        object_names,
    })
}

pub(super) async fn select_scene_profile_sync_chapters(
    state: &AppState,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<Vec<chapter::Model>, String> {
    if project_id <= 0 {
        return Err("场景档案同步 project_id 必须大于 0".to_string());
    }
    if chapter_number.is_some_and(|value| value <= 0) {
        return Err("场景档案同步章节号必须大于 0".to_string());
    }
    let mut chapters = state
        .orchestrator
        .character_sync_chapters(project_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    if let Some(chapter_number) = chapter_number {
        chapters.retain(|chapter| chapter.chapter_number == chapter_number);
        if chapters.is_empty() {
            return Err(format!(
                "第 {chapter_number} 章没有可用于场景档案同步的有效正文"
            ));
        }
        return Ok(chapters);
    }
    if chapters.is_empty() {
        return Err(format!(
            "项目还没有可用于场景档案同步的章节: project_id={project_id}"
        ));
    }
    Ok(chapters)
}

pub(super) fn existing_scene_profile_for_prompt(profile: &scene_profile::Model) -> Value {
    json!({"id": profile.id, "name": profile.name, "aliases": profile.aliases_json, "scene_type": profile.scene_type, "importance": profile.importance, "base_description": profile.base_description, "visual_description": profile.visual_description, "spatial_layout": profile.spatial_layout, "atmosphere": profile.atmosphere, "current_state": profile.current_state_json, "first_seen_chapter": profile.first_seen_chapter, "last_seen_chapter": profile.last_seen_chapter})
}

pub(super) fn character_profile_for_prompt(profile: &protagonist_profile::Model) -> Value {
    json!({"profile_id": profile.id, "name": profile.name, "appearance": profile.appearance, "current_state": profile.current_state, "last_synced_chapter": profile.last_synced_chapter})
}

pub(super) fn object_profile_for_prompt(profile: &object_profile::Model) -> Value {
    json!({"object_profile_id": profile.id, "name": profile.name, "object_type": profile.object_type, "visual_description": profile.visual_description, "material_and_state": profile.material_and_state, "location_context": profile.location_context, "last_seen_chapter": profile.last_seen_chapter})
}

pub(super) fn scene_profile_chapter_context(chapter: &chapter::Model) -> Value {
    json!({"chapter_id": chapter.id, "chapter_number": chapter.chapter_number, "chapter_title": chapter.title, "content": chapter.content, "analysis_data": chapter.analysis_data})
}

pub(super) async fn read_current_chapter_content_anchors(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<text_anchor::Model>, String> {
    let rows = state
        .orchestrator
        .read_current_chapter_content_anchor_records(project_id, chapter_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    if rows.anchors.is_empty() {
        return Err(format!(
            "状态无效：当前状态 第 {} 章缺少当前正文文本锚点，不允许操作 预览模型请求",
            rows.chapter.chapter_number
        ));
    }
    if rows
        .anchors
        .iter()
        .any(|anchor| anchor.source_version_hash != rows.source_version_hash)
    {
        return Err(format!(
            "状态无效：当前状态 第 {} 章正文文本锚点已过期，不允许操作 预览模型请求",
            rows.chapter.chapter_number
        ));
    }
    Ok(rows.anchors)
}
