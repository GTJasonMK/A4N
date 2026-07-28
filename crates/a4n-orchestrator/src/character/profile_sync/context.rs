use a4n_core::{
    error::AppError,
    models::{chapter::chapter, character::protagonist_profile, project::text_anchor},
    source_spans::text_anchor_prompt_values,
};
use serde_json::Value;

use crate::OrchestratorContext;

use super::{
    super::{
        CharacterProfileSyncStageInput, CharacterSyncRangeMode, character_llm_chapter_context,
        character_sync_range, profile_first_seen_chapter, snapshot_attributes_from_data,
        validate_character_sync_target_chapter, validate_contiguous_character_sync_range,
    },
    contracts::{
        CharacterProfileSyncAnchorMode, CharacterProfileSyncApplicationInput,
        CharacterProfileSyncSubject,
    },
};

pub(super) struct PreparedCharacterProfileSync {
    pub(super) project_id: i64,
    pub(super) profile: protagonist_profile::Model,
    pub(super) start_chapter: i32,
    pub(super) target_chapter: i32,
    pub(super) clear_to_chapter: i32,
    pub(super) selected_chapter_numbers: Vec<i32>,
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
            last_synced_chapter: self.profile.last_synced_chapter,
            start_chapter: self.start_chapter,
            target_chapter: self.target_chapter,
            selected_chapter_numbers: self.selected_chapter_numbers.clone(),
            chapter_data: self.chapter_data.clone(),
            baseline_state: self.baseline_state.clone(),
            base_attributes: self.profile.base_attributes.clone(),
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
        let target_chapter =
            validate_character_sync_target_chapter(&chapters, project_id, input.target_chapter)
                .map_err(AppError::InvalidArgument)?;
        self.prepare_character_profile_sync_for_profile(
            profile,
            &chapters,
            target_chapter,
            input.range_mode,
            input.anchor_mode,
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
    ) -> Result<PreparedCharacterProfileSync, AppError> {
        let Some(range) = character_sync_range(&profile, target_chapter, range_mode) else {
            return Ok(PreparedCharacterProfileSync {
                project_id: profile.project_id,
                baseline_state: profile.current_state.clone(),
                profile,
                start_chapter: target_chapter,
                target_chapter,
                clear_to_chapter: target_chapter,
                selected_chapter_numbers: Vec::new(),
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

        let first_seen = profile_first_seen_chapter(&profile).unwrap_or(1);
        let baseline_state = if range.start_chapter <= first_seen {
            profile.base_attributes.clone()
        } else if profile.last_synced_chapter >= range.start_chapter {
            self.character_profile_state_before_chapter(&profile, range.start_chapter)
                .await?
        } else {
            profile.current_state.clone()
        };
        let mut chapter_data = Vec::with_capacity(selected_chapters.len());
        for chapter in selected_chapters {
            let anchors = match anchor_mode {
                CharacterProfileSyncAnchorMode::EnsureCurrent => {
                    self.ensure_chapter_content_anchor_records(profile.project_id, chapter.id)
                        .await?
                }
                CharacterProfileSyncAnchorMode::RequireExistingCurrent => {
                    self.read_current_character_profile_sync_anchors(profile.project_id, chapter.id)
                        .await?
                }
            };
            chapter_data.push(character_profile_sync_chapter_context(chapter, &anchors));
        }

        Ok(PreparedCharacterProfileSync {
            project_id: profile.project_id,
            profile,
            start_chapter: range.start_chapter,
            target_chapter,
            clear_to_chapter: range.clear_to_chapter,
            selected_chapter_numbers,
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
        let snapshot_chapter = chapter_number - 1;
        let snapshot = self
            .snapshot_reference_by_chapter(profile.id, snapshot_chapter)
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: format!("第 {snapshot_chapter} 章没有角色快照"),
                operation: format!("从第 {chapter_number} 章断点重同步"),
            })?;
        snapshot_attributes_from_data(&snapshot.data, snapshot_chapter)
            .map_err(super::super::character_db_error)
    }

    async fn read_current_character_profile_sync_anchors(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<text_anchor::Model>, AppError> {
        let rows = self
            .read_current_chapter_content_anchor_records(project_id, chapter_id)
            .await?;
        if rows.anchors.is_empty() {
            return Err(AppError::InvalidState {
                current: format!("第 {} 章缺少当前正文文本锚点", rows.chapter.chapter_number),
                operation: "预览模型请求".to_string(),
            });
        }
        if rows
            .anchors
            .iter()
            .any(|anchor| anchor.source_version_hash != rows.source_version_hash)
        {
            return Err(AppError::InvalidState {
                current: format!("第 {} 章正文文本锚点已过期", rows.chapter.chapter_number),
                operation: "预览模型请求".to_string(),
            });
        }
        Ok(rows.anchors)
    }
}

fn character_profile_sync_chapter_context(
    chapter: &chapter::Model,
    anchors: &[text_anchor::Model],
) -> Value {
    let mut context = character_llm_chapter_context(chapter);
    if let Some(object) = context.as_object_mut() {
        object.insert(
            "text_anchors".to_string(),
            Value::Array(text_anchor_prompt_values(
                std::slice::from_ref(chapter),
                anchors,
            )),
        );
    }
    context
}
