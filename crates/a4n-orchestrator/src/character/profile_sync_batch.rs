use std::collections::BTreeSet;

use a4n_api_contract::character::{
    CharacterProfileSkippedResult, CharacterProfileSyncBatchStatus, CharacterProfileSyncFailure,
    CharacterProfileSyncedResult, SyncAllCharacterProfilesResult, SyncCharacterProfileResult,
};
use a4n_core::{error::AppError, models::character::protagonist_profile};
use a4n_prompt::rendering::PromptRenderer;
use serde::Serialize;
use serde_json::Value;

use crate::OrchestratorContext;

use super::{
    CHARACTER_PROFILE_SYNC_STEPS, CharacterProfileSyncAnchorMode,
    CharacterProfileSyncApplicationInput, CharacterProfileSyncPreviewStep,
    CharacterProfileSyncProgress, CharacterProfileSyncStageTracePort, CharacterProfileSyncSubject,
    CharacterSyncRangeMode, character_identity_key, character_profile_skipped_result,
    profile_character_keys, sync_rules::profile_sync_skip_reason_for_mode,
    validate_character_sync_target_chapter,
};

impl OrchestratorContext {
    pub async fn run_all_character_profile_sync_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
        prompt_renderer: &dyn PromptRenderer,
        progress: CharacterProfileSyncProgress<'_>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<SyncAllCharacterProfilesResult, AppError> {
        progress.report_with_total(0, 1, "正在校验同步目标章节")?;
        let target_chapter = self
            .resolve_character_profile_sync_target_chapter(project_id, chapter_number)
            .await?;
        let profiles = self.protagonist_profiles_for_project(project_id).await?;

        if profiles.is_empty() {
            progress.report_with_total(1, 1, "当前没有可同步的角色档案")?;
            return Ok(character_profile_sync_batch_result(
                target_chapter,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ));
        }

        let (synced, skipped, failed) = self
            .run_character_profile_sync_batch(
                profiles,
                target_chapter,
                CharacterSyncRangeMode::Incremental,
                CharacterProfileSyncAnchorMode::EnsureCurrent,
                prompt_renderer,
                progress,
                trace_port,
            )
            .await?;
        Ok(character_profile_sync_batch_result(
            target_chapter,
            synced,
            skipped,
            failed,
        ))
    }

    pub async fn prepare_all_character_profile_sync_preview_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<CharacterProfileSyncPreviewStep>, AppError> {
        let target_chapter = self
            .resolve_character_profile_sync_target_chapter(project_id, chapter_number)
            .await?;
        let profiles = self.protagonist_profiles_for_project(project_id).await?;
        let mut steps = Vec::new();
        for profile in profiles {
            if let Some(step) = self
                .prepare_character_profile_sync_preview_validated(
                    CharacterProfileSyncApplicationInput {
                        subject: CharacterProfileSyncSubject::Profile(profile),
                        target_chapter,
                        range_mode: CharacterSyncRangeMode::Incremental,
                        anchor_mode: CharacterProfileSyncAnchorMode::RequireExistingCurrent,
                    },
                    prompt_renderer,
                )
                .await?
            {
                steps.push(step);
            }
        }
        Ok(steps)
    }

    pub(crate) async fn run_analysis_character_profile_sync_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
        analysis: Option<&Value>,
        discovery_result: Option<&Value>,
        prompt_renderer: &dyn PromptRenderer,
        progress: CharacterProfileSyncProgress<'_>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<AnalysisCharacterProfileSyncResult, AppError> {
        progress.report_with_total(0, 1, "正在按章节分析结果选择待同步角色")?;
        let target_chapter = self
            .resolve_character_profile_sync_target_chapter(project_id, chapter_number)
            .await?;
        let analysis_character_names = analysis
            .map(character_names_from_chapter_analysis)
            .unwrap_or_default();
        let discovery_character_names = discovery_result
            .map(character_names_from_discovery_result)
            .unwrap_or_default();
        let sync_character_names =
            analysis_sync_character_names(&analysis_character_names, &discovery_character_names);
        let sync_character_keys = character_keys_from_names(&sync_character_names);
        let analysis_characters = analysis_character_names.iter().cloned().collect::<Vec<_>>();
        let discovery_characters = discovery_character_names
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let sync_characters = sync_character_names.iter().cloned().collect::<Vec<_>>();

        if sync_character_keys.is_empty() {
            return Ok(analysis_sync_result(
                target_chapter,
                analysis_characters,
                discovery_characters,
                sync_characters,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ));
        }

        let profiles = self.protagonist_profiles_for_project(project_id).await?;
        let all_profile_keys = profiles
            .iter()
            .flat_map(|profile| profile_character_keys(profile).into_iter())
            .collect::<BTreeSet<_>>();
        let matched_profiles = profiles
            .into_iter()
            .filter(|profile| {
                profile_character_keys(profile)
                    .iter()
                    .any(|key| sync_character_keys.contains(key))
            })
            .collect::<Vec<_>>();
        let unmatched_analysis_characters = analysis_character_names
            .iter()
            .filter(|name| !all_profile_keys.contains(&character_identity_key(name)))
            .cloned()
            .collect::<Vec<_>>();
        let unmatched_sync_characters = sync_character_names
            .iter()
            .filter(|name| !all_profile_keys.contains(&character_identity_key(name)))
            .cloned()
            .collect::<Vec<_>>();

        if matched_profiles.is_empty() {
            return Ok(analysis_sync_result(
                target_chapter,
                analysis_characters,
                discovery_characters,
                sync_characters,
                unmatched_analysis_characters,
                unmatched_sync_characters,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ));
        }

        let (synced, skipped, failed) = self
            .run_character_profile_sync_batch(
                matched_profiles,
                target_chapter,
                CharacterSyncRangeMode::AnalysisResyncFromTarget,
                CharacterProfileSyncAnchorMode::EnsureCurrent,
                prompt_renderer,
                progress,
                trace_port,
            )
            .await?;
        Ok(analysis_sync_result(
            target_chapter,
            analysis_characters,
            discovery_characters,
            sync_characters,
            unmatched_analysis_characters,
            unmatched_sync_characters,
            synced,
            skipped,
            failed,
        ))
    }

    async fn resolve_character_profile_sync_target_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<i32, AppError> {
        let chapters = self.character_sync_chapters(project_id).await?;
        validate_character_sync_target_chapter(&chapters, project_id, chapter_number)
            .map_err(AppError::InvalidArgument)
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_character_profile_sync_batch(
        &self,
        profiles: Vec<protagonist_profile::Model>,
        target_chapter: i32,
        range_mode: CharacterSyncRangeMode,
        anchor_mode: CharacterProfileSyncAnchorMode,
        prompt_renderer: &dyn PromptRenderer,
        progress: CharacterProfileSyncProgress<'_>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<
        (
            Vec<CharacterProfileSyncedResult>,
            Vec<CharacterProfileSkippedResult>,
            Vec<CharacterProfileSyncFailure>,
        ),
        AppError,
    > {
        let mut synced = Vec::new();
        let mut skipped = Vec::new();
        let mut failed = Vec::new();
        let total = batch_progress_total(profiles.len())?;

        for (index, profile) in profiles.into_iter().enumerate() {
            let base = batch_progress_base(index)?;
            let name = profile.name.clone();
            progress.report_with_total(
                base,
                total,
                &format!(
                    "角色 {}/{} · {name}：准备同步",
                    index + 1,
                    total / CHARACTER_PROFILE_SYNC_STEPS
                ),
            )?;
            let skip_reason =
                profile_sync_skip_reason_for_mode(&profile, target_chapter, range_mode);
            if let Some(reason) = skip_reason {
                skipped.push(character_profile_skipped_result(
                    &profile,
                    target_chapter,
                    reason,
                ));
                progress.report_with_total(
                    base + CHARACTER_PROFILE_SYNC_STEPS,
                    total,
                    &format!("角色 {} · {name}：无需同步，跳过", index + 1),
                )?;
                continue;
            }

            let profile_reporter = |current: u32, local_total: u32, message: &str| {
                let offset = u64::from(current.min(local_total))
                    * u64::from(CHARACTER_PROFILE_SYNC_STEPS)
                    / u64::from(local_total);
                let mapped = base
                    + u32::try_from(offset).map_err(|_| {
                        AppError::InvalidArgument("角色同步进度超出 u32 范围".to_string())
                    })?;
                progress.report_with_total(mapped, total, message)
            };
            let result = self
                .run_character_profile_sync_validated(
                    CharacterProfileSyncApplicationInput {
                        subject: CharacterProfileSyncSubject::Profile(profile),
                        target_chapter,
                        range_mode,
                        anchor_mode,
                    },
                    prompt_renderer,
                    Some(CharacterProfileSyncProgress {
                        reporter: Some(&profile_reporter),
                        active_task_id: progress.active_task_id,
                    }),
                    trace_port,
                )
                .await;
            match result {
                Ok(SyncCharacterProfileResult::Synced(result)) => synced.push(result),
                Ok(SyncCharacterProfileResult::Skipped(result)) => skipped.push(result),
                Err(AppError::Cancelled) => return Err(AppError::Cancelled),
                Err(error) => failed.push(CharacterProfileSyncFailure {
                    name: name.clone(),
                    error: error.to_string(),
                }),
            }
            progress.report_with_total(
                base + CHARACTER_PROFILE_SYNC_STEPS,
                total,
                &format!("角色 {} · {name}：处理完成", index + 1),
            )?;
        }
        Ok((synced, skipped, failed))
    }
}

fn batch_progress_total(profile_count: usize) -> Result<u32, AppError> {
    u32::try_from(profile_count)
        .map_err(|_| AppError::InvalidArgument("角色数量超出 u32 范围".to_string()))?
        .checked_mul(CHARACTER_PROFILE_SYNC_STEPS)
        .ok_or_else(|| AppError::InvalidArgument("批量角色同步进度超出 u32 范围".to_string()))
}

fn batch_progress_base(index: usize) -> Result<u32, AppError> {
    u32::try_from(index)
        .map_err(|_| AppError::InvalidArgument("角色序号超出 u32 范围".to_string()))?
        .checked_mul(CHARACTER_PROFILE_SYNC_STEPS)
        .ok_or_else(|| AppError::InvalidArgument("角色同步进度基数超出 u32 范围".to_string()))
}

fn batch_status(
    synced: &[CharacterProfileSyncedResult],
    skipped: &[CharacterProfileSkippedResult],
    failed: &[CharacterProfileSyncFailure],
) -> CharacterProfileSyncBatchStatus {
    if failed.is_empty() {
        if synced.is_empty() {
            CharacterProfileSyncBatchStatus::Skipped
        } else {
            CharacterProfileSyncBatchStatus::Synced
        }
    } else if synced.is_empty() && skipped.is_empty() {
        CharacterProfileSyncBatchStatus::Failed
    } else {
        CharacterProfileSyncBatchStatus::CompletedWithErrors
    }
}

fn character_profile_sync_batch_result(
    target_chapter: i32,
    synced: Vec<CharacterProfileSyncedResult>,
    skipped: Vec<CharacterProfileSkippedResult>,
    failed: Vec<CharacterProfileSyncFailure>,
) -> SyncAllCharacterProfilesResult {
    SyncAllCharacterProfilesResult {
        status: batch_status(&synced, &skipped, &failed),
        target_chapter,
        total: synced.len() + skipped.len() + failed.len(),
        synced,
        skipped,
        failed,
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct AnalysisCharacterProfileSyncResult {
    pub(crate) analysis_characters: Vec<String>,
    pub(crate) discovery_characters: Vec<String>,
    pub(crate) sync_characters: Vec<String>,
    pub(crate) unmatched_analysis_characters: Vec<String>,
    pub(crate) unmatched_sync_characters: Vec<String>,
    pub(crate) preserved: Vec<AnalysisPreservedCharacterAttribute>,
    #[serde(flatten)]
    pub(crate) batch: SyncAllCharacterProfilesResult,
}

#[derive(Debug, Serialize)]
pub(crate) struct AnalysisPreservedCharacterAttribute {
    pub(crate) name: String,
    pub(crate) character_name: String,
    pub(crate) chapter_number: i32,
    pub(crate) category: String,
    pub(crate) key: String,
    pub(crate) operation: String,
}

#[allow(clippy::too_many_arguments)]
fn analysis_sync_result(
    target_chapter: i32,
    analysis_characters: Vec<String>,
    discovery_characters: Vec<String>,
    sync_characters: Vec<String>,
    unmatched_analysis_characters: Vec<String>,
    unmatched_sync_characters: Vec<String>,
    synced: Vec<CharacterProfileSyncedResult>,
    skipped: Vec<CharacterProfileSkippedResult>,
    failed: Vec<CharacterProfileSyncFailure>,
) -> AnalysisCharacterProfileSyncResult {
    let preserved = synced
        .iter()
        .flat_map(|profile| {
            profile.protected_attributes.iter().map(|attribute| {
                AnalysisPreservedCharacterAttribute {
                    name: format!(
                        "{} · {}「{}」",
                        profile.name,
                        protected_attribute_category_label(&attribute.category),
                        attribute.key
                    ),
                    character_name: profile.name.clone(),
                    chapter_number: attribute.chapter_number,
                    category: attribute.category.clone(),
                    key: attribute.key.clone(),
                    operation: attribute.operation.clone(),
                }
            })
        })
        .collect();
    AnalysisCharacterProfileSyncResult {
        analysis_characters,
        discovery_characters,
        sync_characters,
        unmatched_analysis_characters,
        unmatched_sync_characters,
        preserved,
        batch: character_profile_sync_batch_result(target_chapter, synced, skipped, failed),
    }
}

fn protected_attribute_category_label(category: &str) -> &str {
    match category {
        "social" => "关系",
        "implicit" => "隐性属性",
        _ => "显性属性",
    }
}

pub fn analysis_sync_character_names(
    analysis_character_names: &BTreeSet<String>,
    discovery_character_names: &BTreeSet<String>,
) -> BTreeSet<String> {
    analysis_character_names
        .iter()
        .chain(discovery_character_names.iter())
        .cloned()
        .collect()
}

pub fn character_keys_from_names(names: &BTreeSet<String>) -> BTreeSet<String> {
    names
        .iter()
        .map(|name| character_identity_key(name))
        .filter(|key| !key.is_empty())
        .collect()
}

pub fn character_names_from_chapter_analysis(analysis: &Value) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    if let Some(metadata) = analysis.get("metadata") {
        collect_character_name_values(metadata.get("characters"), &mut names);
    }
    if let Some(states) = analysis.get("character_states").and_then(Value::as_object) {
        for name in states.keys() {
            insert_analysis_character_name(name, &mut names);
        }
    }
    if let Some(events) = analysis.get("key_events").and_then(Value::as_array) {
        for event in events {
            collect_character_name_values(event.get("involved_characters"), &mut names);
        }
    }
    names
}

pub fn character_names_from_discovery_result(result: &Value) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for field in ["created", "existing"] {
        let Some(items) = result.get(field).and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            collect_character_name_values(item.get("name"), &mut names);
            collect_character_name_values(item.get("aliases"), &mut names);
            if let Some(base_attributes) = item.get("base_attributes") {
                collect_character_attribute_aliases(base_attributes, &mut names);
            }
            if let Some(current_state) = item.get("current_state") {
                collect_character_attribute_aliases(current_state, &mut names);
            }
        }
    }
    names
}

fn collect_character_attribute_aliases(attributes: &Value, names: &mut BTreeSet<String>) {
    if let Some(explicit) = attributes.get("explicit_attributes") {
        collect_character_name_values(explicit.get("aliases"), names);
    }
}

fn collect_character_name_values(value: Option<&Value>, names: &mut BTreeSet<String>) {
    match value {
        Some(Value::String(text)) => insert_analysis_character_name(text, names),
        Some(Value::Array(items)) => {
            for item in items {
                collect_character_name_values(Some(item), names);
            }
        }
        _ => {}
    }
}

fn insert_analysis_character_name(name: &str, names: &mut BTreeSet<String>) {
    let name = name.trim();
    if !name.is_empty() {
        names.insert(name.to_string());
    }
}
