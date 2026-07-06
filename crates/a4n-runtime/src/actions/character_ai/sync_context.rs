use crate::state::AppState;
use a4n_core::models::{chapter::chapter, character::protagonist_profile, project::text_anchor};
use a4n_services::chapter::analysis::summary_text;
use a4n_services::text_anchor::text_anchor_prompt_values;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Map, Value, json};

use super::state_projection::profile_state_before_chapter;
use super::sync_scope::{
    CharacterSyncRangeMode, character_sync_range, profile_first_seen_chapter,
    validate_character_sync_target_chapter,
};
use super::sync_types::CharacterSyncContext;
use super::sync_validate::validate_contiguous_sync_range;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CharacterSyncAnchorMode {
    EnsureCurrent,
    RequireExistingCurrent,
}

pub(super) fn character_llm_chapter_context(chapter: &chapter::Model) -> Value {
    let mut context = Map::new();
    context.insert(
        "chapter_number".to_string(),
        Value::from(chapter.chapter_number),
    );
    context.insert("title".to_string(), json!(chapter.title));

    if let Some(summary) = chapter_summary_for_character_prompt(chapter) {
        context.insert("summary".to_string(), Value::String(summary));
    }
    if let Some(analysis) = chapter
        .analysis_data
        .as_ref()
        .and_then(compact_character_prompt_analysis)
    {
        context.insert("analysis".to_string(), analysis);
    }
    context.insert(
        "content".to_string(),
        Value::String(chapter.content.clone()),
    );

    Value::Object(context)
}

fn character_llm_chapter_context_with_anchors(
    chapter: &chapter::Model,
    text_anchors: &[text_anchor::Model],
) -> Value {
    let mut context = character_llm_chapter_context(chapter);
    if let Some(object) = context.as_object_mut() {
        object.insert(
            "text_anchors".to_string(),
            Value::Array(text_anchor_prompt_values(
                std::slice::from_ref(chapter),
                text_anchors,
            )),
        );
    }
    context
}

pub(super) async fn build_character_sync_context_by_name(
    state: &AppState,
    project_id: i64,
    name: &str,
    chapter_number: i32,
    anchor_mode: CharacterSyncAnchorMode,
) -> Result<CharacterSyncContext, String> {
    let profile = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Name.eq(name))
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("角色不存在: project_id={project_id}, name={name}"))?;
    let chapters = crate::workflow::character_sync_chapters(state, project_id).await?;
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
            CharacterSyncAnchorMode::EnsureCurrent => {
                state
                    .text_anchor
                    .ensure_chapter_content_anchors(profile.project_id, chapter.id)
                    .await
            }
            CharacterSyncAnchorMode::RequireExistingCurrent => {
                state
                    .text_anchor
                    .read_current_chapter_content_anchors(profile.project_id, chapter.id)
                    .await
            }
        }
        .map_err(|err| err.to_string())?;
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

fn chapter_summary_for_character_prompt(chapter: &chapter::Model) -> Option<String> {
    chapter
        .summary
        .as_deref()
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| chapter.analysis_data.as_ref().and_then(summary_text))
}

fn compact_character_prompt_analysis(analysis: &Value) -> Option<Value> {
    let mut compact = Map::new();
    copy_analysis_field(&mut compact, analysis, "metadata");
    copy_analysis_field(&mut compact, analysis, "character_states");
    copy_analysis_field(&mut compact, analysis, "key_events");
    copy_analysis_field(&mut compact, analysis, "foreshadowing");

    if compact.is_empty() {
        None
    } else {
        Some(Value::Object(compact))
    }
}

fn copy_analysis_field(compact: &mut Map<String, Value>, analysis: &Value, key: &str) {
    if let Some(value) = analysis.get(key)
        && !value.is_null()
    {
        compact.insert(key.to_string(), value.clone());
    }
}
