mod formatting;

use crate::state::AppState;
use a4n_core::models::{chapter::chapter, character::protagonist_profile, project::text_anchor};

use super::state_projection::profile_state_before_chapter;
use super::sync_scope::{
    CharacterSyncRangeMode, character_sync_range, profile_first_seen_chapter,
    validate_character_sync_target_chapter,
};
use super::sync_types::CharacterSyncContext;
use super::sync_validate::validate_contiguous_sync_range;
pub(in crate::actions::character_ai) use formatting::character_llm_chapter_context;
use formatting::character_llm_chapter_context_with_anchors;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CharacterSyncAnchorMode {
    EnsureCurrent,
    RequireExistingCurrent,
}

pub(super) async fn build_character_sync_context_by_name(
    state: &AppState,
    project_id: i64,
    name: &str,
    chapter_number: i32,
    anchor_mode: CharacterSyncAnchorMode,
) -> Result<CharacterSyncContext, String> {
    let profile = state
        .orchestrator
        .protagonist_profiles_for_project(project_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .into_iter()
        .find(|profile| profile.name == name)
        .ok_or_else(|| format!("角色不存在: project_id={project_id}, name={name}"))?;
    let chapters = state
        .orchestrator
        .character_sync_chapters(project_id)
        .await
        .map_err(|err| err.to_string())?;
    if chapters.is_empty() {
        return Err(format!("项目还没有章节: project_id={project_id}"));
    }
    let target_chapter =
        validate_character_sync_target_chapter(&chapters, project_id, chapter_number)?;
    build_character_sync_context_for_profile(
        state,
        profile,
        &chapters,
        target_chapter,
        CharacterSyncRangeMode::Incremental,
        anchor_mode,
    )
    .await
}

pub(super) async fn build_character_sync_context_for_profile(
    state: &AppState,
    profile: protagonist_profile::Model,
    chapters: &[chapter::Model],
    target_chapter: i32,
    range_mode: CharacterSyncRangeMode,
    anchor_mode: CharacterSyncAnchorMode,
) -> Result<CharacterSyncContext, String> {
    let Some(range) = character_sync_range(&profile, target_chapter, range_mode) else {
        return Ok(CharacterSyncContext {
            baseline_state: profile.current_state.clone(),
            profile,
            start_chapter: target_chapter,
            target_chapter,
            clear_to_chapter: target_chapter,
            selected_chapter_numbers: Vec::new(),
            chapter_data: Vec::new(),
        });
    };
    let start_chapter = range.start_chapter;
    let clear_to_chapter = range.clear_to_chapter;
    let selected_chapters: Vec<&chapter::Model> = chapters
        .iter()
        .filter(|chapter| {
            chapter.chapter_number >= start_chapter && chapter.chapter_number <= target_chapter
        })
        .collect();
    let selected_chapter_numbers: Vec<i32> = selected_chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .collect();
    if selected_chapters.is_empty() {
        return Err(format!(
            "第 {start_chapter} 到第 {target_chapter} 章没有可同步内容"
        ));
    }
    validate_contiguous_sync_range(start_chapter, target_chapter, &selected_chapter_numbers)?;

    let first_seen = profile_first_seen_chapter(&profile).unwrap_or(1);
    let baseline_state = if start_chapter <= first_seen {
        profile.base_attributes.clone()
    } else if profile.last_synced_chapter >= start_chapter {
        profile_state_before_chapter(state, &profile, start_chapter).await?
    } else {
        profile.current_state.clone()
    };
    let mut chapter_data = Vec::with_capacity(selected_chapters.len());
    for chapter in &selected_chapters {
        let text_anchors = match anchor_mode {
            CharacterSyncAnchorMode::EnsureCurrent => state
                .orchestrator
                .ensure_chapter_content_anchor_records(profile.project_id, chapter.id)
                .await
                .map_err(crate::actions::app_error_message)?,
            CharacterSyncAnchorMode::RequireExistingCurrent => {
                read_current_chapter_content_anchors(state, profile.project_id, chapter.id).await?
            }
        };
        chapter_data.push(character_llm_chapter_context_with_anchors(
            chapter,
            &text_anchors,
        ));
    }

    Ok(CharacterSyncContext {
        profile,
        start_chapter,
        target_chapter,
        clear_to_chapter,
        selected_chapter_numbers,
        chapter_data,
        baseline_state,
    })
}

async fn read_current_chapter_content_anchors(
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
