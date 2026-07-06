use crate::state::AppState;
use a4n_core::models::{chapter::chapter, character::protagonist_profile};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use super::sync_scope::{
    CharacterSyncRangeMode, analysis_profile_sync_skip_reason, character_sync_range,
    validate_character_sync_target_chapter,
};
use super::sync_validate::validate_contiguous_sync_range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CharacterSyncPreflightFailure {
    pub(super) name: String,
    pub(super) error: String,
}

pub(crate) async fn preflight_analysis_character_profile_sync_to_chapter(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<(), String> {
    let chapters = crate::workflow::character_sync_chapters(state, project_id).await?;
    let target_chapter =
        validate_character_sync_target_chapter(&chapters, project_id, chapter_number)?;
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("查询角色档案失败: {err}"))?;
    report_analysis_character_profile_sync_preflight_failures(
        analysis_character_profile_sync_preflight_failures(&profiles, &chapters, target_chapter),
    )
}

pub(super) fn analysis_character_profile_sync_preflight_failures(
    profiles: &[protagonist_profile::Model],
    chapters: &[chapter::Model],
    target_chapter: i32,
) -> Vec<CharacterSyncPreflightFailure> {
    profiles
        .iter()
        .filter_map(|profile| {
            analysis_character_profile_sync_preflight_failure(profile, chapters, target_chapter)
        })
        .collect()
}

fn analysis_character_profile_sync_preflight_failure(
    profile: &protagonist_profile::Model,
    chapters: &[chapter::Model],
    target_chapter: i32,
) -> Option<CharacterSyncPreflightFailure> {
    if analysis_profile_sync_skip_reason(profile, target_chapter).is_some() {
        return None;
    }
    let range = character_sync_range(
        profile,
        target_chapter,
        CharacterSyncRangeMode::AnalysisResyncFromTarget,
    )?;
    let selected_chapter_numbers = chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .filter(|chapter| *chapter >= range.start_chapter && *chapter <= target_chapter)
        .collect::<Vec<_>>();
    let result = if selected_chapter_numbers.is_empty() {
        Err(format!(
            "第 {} 到第 {} 章没有可同步内容",
            range.start_chapter, target_chapter
        ))
    } else {
        validate_contiguous_sync_range(
            range.start_chapter,
            target_chapter,
            &selected_chapter_numbers,
        )
    };
    result.err().map(|error| CharacterSyncPreflightFailure {
        name: profile.name.clone(),
        error,
    })
}

pub(super) fn report_analysis_character_profile_sync_preflight_failures(
    failures: Vec<CharacterSyncPreflightFailure>,
) -> Result<(), String> {
    if failures.is_empty() {
        return Ok(());
    }

    let failed = failures.len();
    let details = failures
        .iter()
        .take(5)
        .map(|failure| format!("{}: {}", failure.name, failure.error))
        .collect::<Vec<_>>()
        .join("；");
    Err(format!(
        "章节分析前角色档案同步预检失败：失败 {failed} 个：{details}；请先补齐缺失章节或调整角色同步断点后再生成章节分析"
    ))
}
