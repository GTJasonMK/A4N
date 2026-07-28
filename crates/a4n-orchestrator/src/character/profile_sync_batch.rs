use std::collections::BTreeSet;

use a4n_core::{error::AppError, models::character::protagonist_profile};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

use crate::OrchestratorContext;

use super::{
    CharacterProfileSyncAnchorMode, CharacterProfileSyncApplicationInput,
    CharacterProfileSyncPreviewStep, CharacterProfileSyncProgress,
    CharacterProfileSyncStageTracePort, CharacterProfileSyncSubject, CharacterSyncRangeMode,
    analysis_profile_sync_skip_reason, character_identity_key, profile_character_keys,
    profile_sync_skip_reason, sync_skip_summary, sync_skip_summary_with_reason,
    validate_character_sync_target_chapter,
};

pub const CHARACTER_PROFILE_SYNC_STEPS: u32 = 10;

pub type CharacterProfileBatchProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Clone, Copy, Default)]
pub struct CharacterProfileBatchProgress<'a> {
    pub reporter: Option<CharacterProfileBatchProgressReporter<'a>>,
}

impl CharacterProfileBatchProgress<'_> {
    fn report(&self, current: u32, total: u32, message: &str) -> Result<(), AppError> {
        if let Some(reporter) = self.reporter {
            reporter(current, total.max(1), message)?;
        }
        Ok(())
    }
}

impl OrchestratorContext {
    pub async fn preflight_analysis_character_profile_sync_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<(), AppError> {
        let chapters = self.character_sync_chapters(project_id).await?;
        let target_chapter =
            validate_character_sync_target_chapter(&chapters, project_id, chapter_number)
                .map_err(AppError::InvalidArgument)?;
        let profiles = self.protagonist_profiles_for_project(project_id).await?;
        let failures = analysis_character_profile_sync_preflight_failures(
            &profiles,
            &chapters,
            target_chapter,
        );
        if failures.is_empty() {
            return Ok(());
        }
        let details = failures
            .iter()
            .take(5)
            .map(|(name, error)| format!("{name}: {error}"))
            .collect::<Vec<_>>()
            .join("；");
        Err(AppError::InvalidState {
            current: format!("角色档案同步预检失败：{} 个", failures.len()),
            operation: format!("生成章节分析前请先补齐缺失章节或调整角色同步断点：{details}"),
        })
    }

    pub async fn run_all_character_profile_sync_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
        prompt_renderer: &dyn PromptRenderer,
        progress: CharacterProfileBatchProgress<'_>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<Value, AppError> {
        progress.report(0, 1, "正在校验同步目标章节")?;
        let target_chapter = self
            .resolve_character_profile_sync_target_chapter(project_id, chapter_number)
            .await?;
        let profiles = self.protagonist_profiles_for_project(project_id).await?;

        if profiles.is_empty() {
            progress.report(1, 1, "当前没有可同步的角色档案")?;
            return Ok(json!({
                "status": "skipped",
                "target_chapter": target_chapter,
                "total": 0,
                "synced": [],
                "skipped": [],
                "failed": [],
            }));
        }

        let profile_count = profiles.len();
        let (synced, skipped, failed) = self
            .run_character_profile_sync_batch(
                profiles,
                target_chapter,
                CharacterSyncRangeMode::Incremental,
                CharacterProfileSyncAnchorMode::EnsureCurrent,
                false,
                prompt_renderer,
                progress,
                trace_port,
            )
            .await?;
        Ok(json!({
            "status": batch_status(&synced, &skipped, &failed),
            "target_chapter": target_chapter,
            "total": profile_count,
            "synced": synced,
            "skipped": skipped,
            "failed": failed,
        }))
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
            steps.extend(
                self.prepare_character_profile_sync_preview_validated(
                    CharacterProfileSyncApplicationInput {
                        subject: CharacterProfileSyncSubject::Profile(profile),
                        target_chapter,
                        range_mode: CharacterSyncRangeMode::Incremental,
                        anchor_mode: CharacterProfileSyncAnchorMode::RequireExistingCurrent,
                    },
                    prompt_renderer,
                )
                .await?,
            );
        }
        Ok(steps)
    }

    pub async fn run_analysis_character_profile_sync_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
        analysis: Option<&Value>,
        discovery_result: Option<&Value>,
        prompt_renderer: &dyn PromptRenderer,
        progress: CharacterProfileBatchProgress<'_>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<Value, AppError> {
        progress.report(0, 1, "正在按章节分析结果选择待同步角色")?;
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
            return Ok(analysis_sync_summary(
                target_chapter,
                &analysis_character_names,
                analysis_characters,
                &discovery_character_names,
                discovery_characters,
                &sync_character_names,
                sync_characters,
                0,
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
        let matched_count = matched_profiles.len();

        if matched_profiles.is_empty() {
            return Ok(analysis_sync_summary(
                target_chapter,
                &analysis_character_names,
                analysis_characters,
                &discovery_character_names,
                discovery_characters,
                &sync_character_names,
                sync_characters,
                0,
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
                true,
                prompt_renderer,
                progress,
                trace_port,
            )
            .await?;
        Ok(analysis_sync_summary(
            target_chapter,
            &analysis_character_names,
            analysis_characters,
            &discovery_character_names,
            discovery_characters,
            &sync_character_names,
            sync_characters,
            matched_count,
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
        analysis_mode: bool,
        prompt_renderer: &dyn PromptRenderer,
        progress: CharacterProfileBatchProgress<'_>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<(Vec<Value>, Vec<Value>, Vec<Value>), AppError> {
        let mut synced = Vec::new();
        let mut skipped = Vec::new();
        let mut failed = Vec::new();
        let total = batch_progress_total(profiles.len())?;

        for (index, profile) in profiles.into_iter().enumerate() {
            let base = batch_progress_base(index)?;
            let name = profile.name.clone();
            progress.report(
                base,
                total,
                &format!("角色 {}/{} · {name}：准备同步", index + 1, total / 10),
            )?;
            let skip_reason = if analysis_mode {
                analysis_profile_sync_skip_reason(&profile, target_chapter)
            } else {
                profile_sync_skip_reason(&profile, target_chapter)
            };
            if let Some(reason) = skip_reason {
                skipped.push(if analysis_mode {
                    sync_skip_summary_with_reason(&profile, target_chapter, reason)
                } else {
                    sync_skip_summary(&profile, target_chapter)
                });
                progress.report(
                    base.saturating_add(CHARACTER_PROFILE_SYNC_STEPS),
                    total,
                    &format!("角色 {} · {name}：无需同步，跳过", index + 1),
                )?;
                continue;
            }

            let profile_reporter = |current: u32, local_total: u32, message: &str| {
                let local_total = local_total.max(1);
                let offset = u64::from(current.min(local_total))
                    * u64::from(CHARACTER_PROFILE_SYNC_STEPS)
                    / u64::from(local_total);
                let mapped = base.saturating_add(u32::try_from(offset).map_err(|_| {
                    AppError::InvalidArgument("角色同步进度超出 u32 范围".to_string())
                })?);
                progress.report(mapped.min(total), total, message)
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
                    }),
                    trace_port,
                )
                .await;
            match result {
                Ok(result) => synced.push(json!({ "name": name, "result": result })),
                Err(AppError::Cancelled) => return Err(AppError::Cancelled),
                Err(error) => failed.push(json!({ "name": name, "error": error.to_string() })),
            }
            progress.report(
                base.saturating_add(CHARACTER_PROFILE_SYNC_STEPS),
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

fn analysis_character_profile_sync_preflight_failures(
    profiles: &[protagonist_profile::Model],
    chapters: &[a4n_core::models::chapter::chapter::Model],
    target_chapter: i32,
) -> Vec<(String, String)> {
    profiles
        .iter()
        .filter_map(|profile| {
            if analysis_profile_sync_skip_reason(profile, target_chapter).is_some() {
                return None;
            }
            let range = super::character_sync_range(
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
                super::validate_contiguous_character_sync_range(
                    range.start_chapter,
                    target_chapter,
                    &selected_chapter_numbers,
                )
            };
            result.err().map(|error| (profile.name.clone(), error))
        })
        .collect()
}

fn batch_progress_base(index: usize) -> Result<u32, AppError> {
    u32::try_from(index)
        .map_err(|_| AppError::InvalidArgument("角色序号超出 u32 范围".to_string()))?
        .checked_mul(CHARACTER_PROFILE_SYNC_STEPS)
        .ok_or_else(|| AppError::InvalidArgument("角色同步进度基数超出 u32 范围".to_string()))
}

fn batch_status(synced: &[Value], skipped: &[Value], failed: &[Value]) -> &'static str {
    if failed.is_empty() {
        if synced.is_empty() {
            "skipped"
        } else {
            "synced"
        }
    } else if synced.is_empty() && skipped.is_empty() {
        "failed"
    } else {
        "completed_with_errors"
    }
}

#[allow(clippy::too_many_arguments)]
fn analysis_sync_summary(
    target_chapter: i32,
    analysis_character_names: &BTreeSet<String>,
    analysis_characters: Vec<String>,
    discovery_character_names: &BTreeSet<String>,
    discovery_characters: Vec<String>,
    sync_character_names: &BTreeSet<String>,
    sync_characters: Vec<String>,
    matched: usize,
    unmatched_analysis_characters: Vec<String>,
    unmatched_sync_characters: Vec<String>,
    synced: Vec<Value>,
    skipped: Vec<Value>,
    failed: Vec<Value>,
) -> Value {
    json!({
        "status": batch_status(&synced, &skipped, &failed),
        "scope": "analysis_characters",
        "target_chapter": target_chapter,
        "analysis_character_count": analysis_character_names.len(),
        "analysis_characters": analysis_characters,
        "discovery_character_count": discovery_character_names.len(),
        "discovery_characters": discovery_characters,
        "sync_character_count": sync_character_names.len(),
        "sync_characters": sync_characters,
        "matched": matched,
        "unmatched_analysis_characters": unmatched_analysis_characters,
        "unmatched_sync_characters": unmatched_sync_characters,
        "total": matched,
        "synced": synced,
        "skipped": skipped,
        "failed": failed,
    })
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
