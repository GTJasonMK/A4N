use std::collections::HashMap;

use a4n_db_models::models::{
    chapter::chapter, character::protagonist_profile, project::text_anchor,
};
use a4n_domain::chapter::ChapterSourceSnapshot;
use a4n_domain::source_spans::{text_anchor_prompt_text, text_anchor_prompt_values};
use a4n_domain::text_anchor::TextAnchorProfileKind;
use a4n_error::AppError;
use serde_json::Value;

use crate::OrchestratorContext;

use super::{
    super::{
        CharacterProfileSyncStageInput, CharacterSyncRangeMode, character_llm_chapter_context,
        character_sync_range_with_first_seen, profile_first_seen_chapter,
        validate_character_sync_target_chapter, validate_contiguous_character_sync_range,
    },
    contracts::{
        CharacterProfileSyncAnchorMode, CharacterProfileSyncApplicationInput,
        CharacterProfileSyncSubject,
    },
};

pub(crate) type CharacterProfileSyncAnchorCache = HashMap<i64, Vec<text_anchor::Model>>;

pub(crate) struct PreparedCharacterProfileSync {
    pub(super) project_id: i64,
    pub(super) profile: protagonist_profile::Model,
    /// 档案声明的 first_seen 与正文证据 first_seen 取 min；跳过判定与区间计算共用此值。
    pub(super) effective_first_seen: Option<i32>,
    pub(super) start_chapter: i32,
    pub(super) target_chapter: i32,
    pub(super) clear_to_chapter: i32,
    pub(super) selected_chapter_numbers: Vec<i32>,
    pub(super) source_chapters: Vec<ChapterSourceSnapshot>,
    pub(super) chapter_data: Vec<Value>,
    pub(super) baseline_state: Value,
}

impl PreparedCharacterProfileSync {
    pub(super) fn stage_input(
        &self,
        workflow_run_id: Option<i64>,
    ) -> CharacterProfileSyncStageInput {
        CharacterProfileSyncStageInput {
            project_id: self.project_id,
            workflow_run_id,
            profile_id: self.profile.id,
            profile_name: self.profile.name.clone(),
            start_chapter: self.start_chapter,
            target_chapter: self.target_chapter,
            selected_chapter_numbers: self.selected_chapter_numbers.clone(),
            chapter_data: self.chapter_data.clone(),
            baseline_state: self.baseline_state.clone(),
        }
    }
}

impl OrchestratorContext {
    pub(super) async fn prepare_character_profile_sync_context(
        &self,
        input: CharacterProfileSyncApplicationInput,
    ) -> Result<PreparedCharacterProfileSync, AppError> {
        let profile = self
            .resolve_character_profile_sync_subject(input.subject)
            .await?;
        let project_id = profile.project_id;
        let chapters = self.character_sync_chapters(project_id).await?;
        if chapters.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "项目还没有章节: project_id={project_id}"
            )));
        }
        let mut anchor_cache = CharacterProfileSyncAnchorCache::new();
        let target_chapter =
            validate_character_sync_target_chapter(&chapters, project_id, input.target_chapter)
                .map_err(AppError::InvalidArgument)?;
        self.prepare_character_profile_sync_for_profile(
            profile,
            &chapters,
            target_chapter,
            input.range_mode,
            input.anchor_mode,
            &mut anchor_cache,
        )
        .await
    }

    pub(crate) async fn prepare_character_profile_sync_context_with_chapters(
        &self,
        input: CharacterProfileSyncApplicationInput,
        chapters: &[chapter::Model],
        anchor_cache: &mut CharacterProfileSyncAnchorCache,
    ) -> Result<PreparedCharacterProfileSync, AppError> {
        let profile = self
            .resolve_character_profile_sync_subject(input.subject)
            .await?;
        if chapters.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "项目还没有章节: project_id={}",
                profile.project_id
            )));
        }
        let target_chapter = validate_character_sync_target_chapter(
            chapters,
            profile.project_id,
            input.target_chapter,
        )
        .map_err(AppError::InvalidArgument)?;
        self.prepare_character_profile_sync_for_profile(
            profile,
            chapters,
            target_chapter,
            input.range_mode,
            input.anchor_mode,
            anchor_cache,
        )
        .await
    }

    async fn resolve_character_profile_sync_subject(
        &self,
        subject: CharacterProfileSyncSubject,
    ) -> Result<protagonist_profile::Model, AppError> {
        match subject {
            CharacterProfileSyncSubject::ByName { project_id, name } => self
                .profile_by_project_name(project_id, &name)
                .await?
                .ok_or_else(|| {
                    AppError::NotFound(format!("角色不存在: project_id={project_id}, name={name}"))
                }),
            CharacterProfileSyncSubject::Profile(profile) => Ok(profile),
        }
    }

    async fn prepare_character_profile_sync_for_profile(
        &self,
        profile: protagonist_profile::Model,
        chapters: &[chapter::Model],
        target_chapter: i32,
        range_mode: CharacterSyncRangeMode,
        anchor_mode: CharacterProfileSyncAnchorMode,
        anchor_cache: &mut CharacterProfileSyncAnchorCache,
    ) -> Result<PreparedCharacterProfileSync, AppError> {
        let evidence_first_seen = self
            .earliest_current_profile_evidence_chapter(
                profile.project_id,
                TextAnchorProfileKind::Character,
                profile.id,
            )
            .await?;
        let effective_first_seen = profile_first_seen_chapter(&profile)
            .into_iter()
            .chain(evidence_first_seen)
            .min();
        let Some(range) = character_sync_range_with_first_seen(
            &profile,
            target_chapter,
            range_mode,
            effective_first_seen,
        ) else {
            return Ok(PreparedCharacterProfileSync {
                project_id: profile.project_id,
                baseline_state: profile.current_state.clone(),
                profile,
                effective_first_seen,
                start_chapter: target_chapter,
                target_chapter,
                clear_to_chapter: target_chapter,
                selected_chapter_numbers: Vec::new(),
                source_chapters: Vec::new(),
                chapter_data: Vec::new(),
            });
        };
        let selected_chapters = chapters
            .iter()
            .filter(|chapter| {
                chapter.chapter_number >= range.start_chapter
                    && chapter.chapter_number <= target_chapter
            })
            .collect::<Vec<_>>();
        let selected_chapter_numbers = selected_chapters
            .iter()
            .map(|chapter| chapter.chapter_number)
            .collect::<Vec<_>>();
        if selected_chapters.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "第 {} 到第 {target_chapter} 章没有可同步内容",
                range.start_chapter
            )));
        }
        validate_contiguous_character_sync_range(
            range.start_chapter,
            target_chapter,
            &selected_chapter_numbers,
        )
        .map_err(AppError::InvalidArgument)?;
        let source_chapters = selected_chapters
            .iter()
            .map(|chapter| a4n_db::project::chapter_source_snapshot_from_model(chapter))
            .collect::<Vec<_>>();

        let first_seen = effective_first_seen.unwrap_or(1);
        let baseline_state = if range.start_chapter <= first_seen {
            profile.base_attributes.clone()
        } else if profile.last_synced_chapter >= range.start_chapter {
            self.character_profile_state_before_chapter(&profile, range.start_chapter)
                .await?
        } else {
            profile.current_state.clone()
        };
        let chapter_ids = selected_chapters
            .iter()
            .map(|chapter| chapter.id)
            .collect::<Vec<_>>();
        let missing_chapter_ids = chapter_ids
            .iter()
            .copied()
            .filter(|chapter_id| !anchor_cache.contains_key(chapter_id))
            .collect::<Vec<_>>();
        if !missing_chapter_ids.is_empty() {
            let anchors = match anchor_mode {
                CharacterProfileSyncAnchorMode::EnsureCurrent => {
                    self.ensure_chapter_content_anchor_records_batch(
                        profile.project_id,
                        &missing_chapter_ids,
                    )
                    .await?
                }
                CharacterProfileSyncAnchorMode::RequireExistingCurrent => {
                    self.require_current_chapter_content_anchors_for_preview_batch(
                        profile.project_id,
                        &missing_chapter_ids,
                    )
                    .await?
                }
            };
            for (chapter_id, anchors) in missing_chapter_ids.into_iter().zip(anchors) {
                anchor_cache.insert(chapter_id, anchors);
            }
        }
        let anchor_groups = chapter_ids
            .iter()
            .map(|chapter_id| {
                anchor_cache.get(chapter_id).cloned().ok_or_else(|| {
                    AppError::Internal(anyhow::anyhow!(
                        "角色同步锚点缓存缺少章节: chapter_id={chapter_id}"
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut chapter_data = Vec::with_capacity(selected_chapters.len());
        for (chapter, anchors) in selected_chapters.iter().zip(anchor_groups) {
            chapter_data.push(character_profile_sync_chapter_context(chapter, &anchors)?);
        }

        Ok(PreparedCharacterProfileSync {
            project_id: profile.project_id,
            profile,
            effective_first_seen,
            start_chapter: range.start_chapter,
            target_chapter,
            clear_to_chapter: range.clear_to_chapter,
            selected_chapter_numbers,
            source_chapters,
            chapter_data,
            baseline_state,
        })
    }

    async fn character_profile_state_before_chapter(
        &self,
        profile: &protagonist_profile::Model,
        chapter_number: i32,
    ) -> Result<Value, AppError> {
        if chapter_number <= 1 {
            return Ok(profile.base_attributes.clone());
        }
        self.character_profile_state_through_chapter(profile, chapter_number - 1)
            .await
    }
}

fn character_profile_sync_chapter_context(
    chapter: &chapter::Model,
    anchors: &[text_anchor::Model],
) -> Result<Value, AppError> {
    let mut context = character_llm_chapter_context(chapter);
    if let Some(object) = context.as_object_mut() {
        object.insert(
            "content".to_string(),
            Value::String(
                text_anchor_prompt_text(chapter, anchors).map_err(AppError::InvalidArgument)?,
            ),
        );
        object.insert(
            "text_anchors".to_string(),
            Value::Array(text_anchor_prompt_values(
                std::slice::from_ref(chapter),
                anchors,
            )),
        );
    }
    Ok(context)
}
