use std::collections::{BTreeSet, HashMap};

use a4n_api_contract::character::{
    CharacterProfileSkippedResult, CharacterProfileSyncBatchStatus, CharacterProfileSyncFailure,
    CharacterProfileSyncedResult, SyncAllCharacterProfilesResult, SyncCharacterProfileResult,
};
use a4n_db_models::models::{chapter::chapter, character::protagonist_profile};
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use serde::Serialize;
use serde_json::Value;

use crate::OrchestratorContext;

use super::profile_sync::CharacterProfileSyncAnchorCache;
use super::{
    CHARACTER_PROFILE_SYNC_STEPS, CharacterProfileSyncAnchorMode,
    CharacterProfileSyncApplicationInput, CharacterProfileSyncPreviewStep,
    CharacterProfileSyncProgress, CharacterProfileSyncStageTracePort, CharacterProfileSyncSubject,
    CharacterSyncRangeMode, character_identity_key, profile_character_keys,
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
        let (chapters_result, profiles_result) = futures::join!(
            self.character_sync_chapters(project_id),
            self.protagonist_profiles_for_project(project_id),
        );
        let chapters = chapters_result?;
        let target_chapter =
            validate_character_sync_target_chapter(&chapters, project_id, chapter_number)
                .map_err(AppError::InvalidArgument)?;
        let profiles = profiles_result?;

        if profiles.is_empty() {
            progress.report_with_total(1, 1, "当前没有可同步的角色档案")?;
            return Ok(character_profile_sync_batch_result(
                target_chapter,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ));
        }

        let mut anchor_cache = HashMap::new();
        let (synced, skipped, failed) = self
            .run_character_profile_sync_batch(
                profiles,
                &chapters,
                target_chapter,
                CharacterSyncRangeMode::Incremental,
                CharacterProfileSyncAnchorMode::EnsureCurrent,
                prompt_renderer,
                progress,
                trace_port,
                &mut anchor_cache,
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
        let (chapters_result, profiles_result) = futures::join!(
            self.character_sync_chapters(project_id),
            self.protagonist_profiles_for_project(project_id),
        );
        let chapters = chapters_result?;
        let target_chapter =
            validate_character_sync_target_chapter(&chapters, project_id, chapter_number)
                .map_err(AppError::InvalidArgument)?;
        let profiles = profiles_result?;
        let mut anchor_cache = HashMap::new();
        let mut steps = Vec::new();
        for profile in profiles {
            let prepared = self
                .prepare_character_profile_sync_context_with_chapters(
                    CharacterProfileSyncApplicationInput {
                        subject: CharacterProfileSyncSubject::Profile(profile),
                        target_chapter,
                        range_mode: CharacterSyncRangeMode::Incremental,
                        anchor_mode: CharacterProfileSyncAnchorMode::RequireExistingCurrent,
                    },
                    &chapters,
                    &mut anchor_cache,
                )
                .await?;
            if let Some(step) = self
                .prepare_character_profile_sync_preview_from_prepared(
                    prepared,
                    CharacterSyncRangeMode::Incremental,
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
        let chapters = self.character_sync_chapters(project_id).await?;
        let target_chapter =
            validate_character_sync_target_chapter(&chapters, project_id, chapter_number)
                .map_err(AppError::InvalidArgument)?;
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

        let mut anchor_cache = HashMap::new();
        let (synced, skipped, failed) = self
            .run_character_profile_sync_batch(
                matched_profiles,
                &chapters,
                target_chapter,
                CharacterSyncRangeMode::AnalysisResyncFromTarget,
                CharacterProfileSyncAnchorMode::EnsureCurrent,
                prompt_renderer,
                progress,
                trace_port,
                &mut anchor_cache,
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

    #[allow(clippy::too_many_arguments)]
    async fn run_character_profile_sync_batch(
        &self,
        profiles: Vec<protagonist_profile::Model>,
        chapters: &[chapter::Model],
        target_chapter: i32,
        range_mode: CharacterSyncRangeMode,
        anchor_mode: CharacterProfileSyncAnchorMode,
        prompt_renderer: &dyn PromptRenderer,
        progress: CharacterProfileSyncProgress<'_>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
        anchor_cache: &mut CharacterProfileSyncAnchorCache,
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

            // 跳过判定统一由 run_character_profile_sync_validated 在 prepare 之后做：
            // 只有那里才拿得到正文证据 first_seen，批量层预判会漏掉它。
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
            let result = async {
                let prepared = self
                    .prepare_character_profile_sync_context_with_chapters(
                        CharacterProfileSyncApplicationInput {
                            subject: CharacterProfileSyncSubject::Profile(profile),
                            target_chapter,
                            range_mode,
                            anchor_mode,
                        },
                        chapters,
                        anchor_cache,
                    )
                    .await?;
                self.run_prepared_character_profile_sync_validated(
                    prepared,
                    range_mode,
                    prompt_renderer,
                    Some(CharacterProfileSyncProgress {
                        reporter: Some(&profile_reporter),
                        active_task_id: progress.active_task_id,
                    }),
                    trace_port,
                )
                .await
            }
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
    a4n_domain::character::analysis_sync_character_names(
        analysis_character_names,
        discovery_character_names,
    )
}

pub fn character_keys_from_names(names: &BTreeSet<String>) -> BTreeSet<String> {
    a4n_domain::character::character_keys_from_names(names)
}

pub fn character_names_from_chapter_analysis(analysis: &Value) -> BTreeSet<String> {
    a4n_domain::character::character_names_from_chapter_analysis(analysis)
}

pub fn character_names_from_discovery_result(result: &Value) -> BTreeSet<String> {
    a4n_domain::character::character_names_from_discovery_result(result)
}
