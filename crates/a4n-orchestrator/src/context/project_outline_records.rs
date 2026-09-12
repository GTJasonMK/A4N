use std::collections::HashMap;

use a4n_api_contract::{
    collaboration::ProjectAccessRole,
    outline::{
        ChapterOutlineSummary, DeleteLatestOutlineRecordsInput, OutlineGenerationProgressSummary,
        OutlineMutationResult, PartOutlineSummary, SaveChapterOutlineInput,
        SaveChapterOutlineResult, SavePartOutlineInput, SavePartOutlineResult,
        UpdateChapterOutlineInput, UpdateChapterOutlineResult,
    },
};
use a4n_db::{DbHandle, project::OutlineGenerationProgressRows};
use a4n_db_models::models::chapter::{chapter_outline, part_outline};
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::project::{
    ensure_positive_project_id, project_chapter_outline_update_error_to_app_error,
    project_outline_record_error_to_app_error,
};
use serde_json::Value;

#[derive(Clone)]
pub(super) struct ProjectOutlineRecordsApplication {
    database: DbHandle,
}

impl ProjectOutlineRecordsApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn chapter_outline_by_project_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<chapter_outline::Model>, AppError> {
        a4n_db::project::chapter_outline_by_project_number(
            self.database.connection(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(project_outline_record_error_to_app_error)
    }

    async fn formal_chapter_outline_by_project_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<chapter_outline::Model>, AppError> {
        a4n_db::project::formal_chapter_outline_by_project_number(
            self.database.connection(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn chapter_outline_source_kinds_for_ids(
        &self,
        outline_ids: &[i64],
    ) -> Result<HashMap<i64, String>, AppError> {
        a4n_db::project::chapter_outline_source_kinds_for_ids(
            self.database.connection(),
            outline_ids,
        )
        .await
        .map_err(project_outline_record_error_to_app_error)
    }

    async fn chapter_outline_by_id(
        &self,
        outline_id: i64,
    ) -> Result<Option<chapter_outline::Model>, AppError> {
        a4n_db::project::chapter_outline_by_id(self.database.connection(), outline_id)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    /// 规范化标题/摘要/结构化字段后更新章节大纲。
    async fn update_chapter_outline_validated(
        &self,
        input: UpdateChapterOutlineInput,
    ) -> Result<chapter_outline::Model, AppError> {
        let input = normalize_chapter_outline_update_input(input)?;
        a4n_db::project::update_chapter_outline_record(self.database.connection(), input)
            .await
            .map_err(project_chapter_outline_update_error_to_app_error)
    }

    async fn save_manual_chapter_outline_validated(
        &self,
        input: SaveChapterOutlineInput,
    ) -> Result<chapter_outline::Model, AppError> {
        let input = a4n_db::project::ManualChapterOutlineSaveInput {
            project_id: input.project_id,
            outline_id: input.outline_id,
            expected_updated_at: input.expected_updated_at,
            chapter_number: input.chapter_number,
            title: input.title,
            summary: input.summary,
        };
        a4n_db::project::save_manual_chapter_outline(self.database.connection(), input)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    async fn save_manual_part_outline_validated(
        &self,
        input: SavePartOutlineInput,
    ) -> Result<part_outline::Model, AppError> {
        let input = a4n_db::project::ManualPartOutlineSaveInput {
            project_id: input.project_id,
            part_id: input.part_id,
            expected_updated_at: input.expected_updated_at,
            part_number: input.part_number,
            start_chapter: input.start_chapter,
            end_chapter: input.end_chapter,
            title: input.title,
            summary: input.summary,
            theme: input.theme,
        };
        a4n_db::project::save_manual_part_outline(self.database.connection(), input)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    async fn delete_latest_part_outline_records_validated(
        &self,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<(), AppError> {
        let input = delete_latest_outline_records_input(input);
        a4n_db::project::delete_latest_part_outline_records(self.database.connection(), input)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    async fn delete_latest_chapter_outline_records_validated(
        &self,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<(), AppError> {
        let input = delete_latest_outline_records_input(input);
        a4n_db::project::delete_latest_chapter_outline_records(self.database.connection(), input)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    async fn chapter_outlines_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        a4n_db::project::chapter_outlines_for_project(self.database.connection(), project_id)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    async fn part_outlines_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<part_outline::Model>, AppError> {
        a4n_db::project::part_outlines_for_project(self.database.connection(), project_id)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    async fn part_outline_by_id(
        &self,
        part_id: i64,
    ) -> Result<Option<part_outline::Model>, AppError> {
        a4n_db::project::part_outline_by_id(self.database.connection(), part_id)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    async fn outline_generation_progress_rows(
        &self,
        project_id: i64,
    ) -> Result<OutlineGenerationProgressRows, AppError> {
        a4n_db::project::outline_generation_progress_rows(self.database.connection(), project_id)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn list_chapter_outlines_for_user(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ChapterOutlineSummary>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        let outlines = self.chapter_outlines_for_project(project_id).await?;
        let source_kinds = self
            .chapter_outline_source_kinds_for_ids(
                &outlines
                    .iter()
                    .map(|outline| outline.id)
                    .collect::<Vec<_>>(),
            )
            .await?;
        Ok(outlines
            .iter()
            .map(|outline| chapter_outline_summary(outline, source_kinds.get(&outline.id)))
            .collect())
    }

    pub(crate) async fn list_part_outlines_for_user(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<PartOutlineSummary>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        Ok(self
            .part_outlines_for_project(project_id)
            .await?
            .iter()
            .map(part_outline_summary)
            .collect())
    }

    pub(crate) async fn outline_generation_progress_for_user(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<OutlineGenerationProgressSummary, AppError> {
        self.outline_generation_progress_for_user_validated(user_id, project_id)
            .await
    }

    pub(crate) async fn chapter_outline_by_project_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<chapter_outline::Model>, AppError> {
        self.project_outline_records
            .chapter_outline_by_project_number(project_id, chapter_number)
            .await
    }

    pub(crate) async fn formal_chapter_outline_by_project_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<chapter_outline::Model>, AppError> {
        self.project_outline_records
            .formal_chapter_outline_by_project_number(project_id, chapter_number)
            .await
    }

    pub(crate) async fn chapter_outline_source_kinds_for_ids(
        &self,
        outline_ids: &[i64],
    ) -> Result<HashMap<i64, String>, AppError> {
        self.project_outline_records
            .chapter_outline_source_kinds_for_ids(outline_ids)
            .await
    }

    pub(crate) async fn chapter_outline_by_id(
        &self,
        outline_id: i64,
    ) -> Result<Option<chapter_outline::Model>, AppError> {
        self.project_outline_records
            .chapter_outline_by_id(outline_id)
            .await
    }

    /// 更新章节大纲：ID 门禁 + 可选字段规范化后落库。
    pub(crate) async fn update_chapter_outline_validated(
        &self,
        input: UpdateChapterOutlineInput,
    ) -> Result<chapter_outline::Model, AppError> {
        self.project_outline_records
            .update_chapter_outline_validated(input)
            .await
    }

    pub(crate) async fn save_manual_chapter_outline_validated(
        &self,
        input: SaveChapterOutlineInput,
    ) -> Result<chapter_outline::Model, AppError> {
        self.project_outline_records
            .save_manual_chapter_outline_validated(input)
            .await
    }

    pub(crate) async fn save_manual_part_outline_validated(
        &self,
        input: SavePartOutlineInput,
    ) -> Result<part_outline::Model, AppError> {
        self.project_outline_records
            .save_manual_part_outline_validated(input)
            .await
    }

    pub(crate) async fn delete_latest_part_outline_records_validated(
        &self,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<(), AppError> {
        self.project_outline_records
            .delete_latest_part_outline_records_validated(input)
            .await
    }

    pub(crate) async fn delete_latest_chapter_outline_records_validated(
        &self,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<(), AppError> {
        self.project_outline_records
            .delete_latest_chapter_outline_records_validated(input)
            .await
    }

    pub(crate) async fn chapter_outlines_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        self.project_outline_records
            .chapter_outlines_for_project(project_id)
            .await
    }

    pub(crate) async fn part_outlines_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<part_outline::Model>, AppError> {
        self.project_outline_records
            .part_outlines_for_project(project_id)
            .await
    }

    pub(crate) async fn part_outline_by_id(
        &self,
        part_id: i64,
    ) -> Result<Option<part_outline::Model>, AppError> {
        self.project_outline_records
            .part_outline_by_id(part_id)
            .await
    }

    pub(crate) async fn outline_generation_progress_rows(
        &self,
        project_id: i64,
    ) -> Result<OutlineGenerationProgressRows, AppError> {
        self.project_outline_records
            .outline_generation_progress_rows(project_id)
            .await
    }

    pub(crate) async fn delete_latest_chapter_outlines_for_user(
        &self,
        user_id: i64,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<OutlineMutationResult, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.delete_latest_chapter_outline_records_validated(input)
            .await?;
        Ok(OutlineMutationResult {
            warnings: Vec::new(),
        })
    }

    pub(crate) async fn delete_latest_part_outlines_for_user(
        &self,
        user_id: i64,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<OutlineMutationResult, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.delete_latest_part_outline_records_validated(input)
            .await?;
        Ok(OutlineMutationResult {
            warnings: Vec::new(),
        })
    }

    pub(crate) async fn save_chapter_outline_for_user(
        &self,
        user_id: i64,
        input: SaveChapterOutlineInput,
    ) -> Result<SaveChapterOutlineResult, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let outline = self.save_manual_chapter_outline_validated(input).await?;
        Ok(SaveChapterOutlineResult {
            outline: chapter_outline_summary(&outline, None),
            warnings: Vec::new(),
        })
    }

    pub(crate) async fn update_chapter_outline_for_user(
        &self,
        user_id: i64,
        input: UpdateChapterOutlineInput,
    ) -> Result<UpdateChapterOutlineResult, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let outline = self.update_chapter_outline_validated(input).await?;
        Ok(UpdateChapterOutlineResult {
            outline: chapter_outline_summary(&outline, None),
            warnings: Vec::new(),
        })
    }

    pub(crate) async fn save_part_outline_for_user(
        &self,
        user_id: i64,
        input: SavePartOutlineInput,
    ) -> Result<SavePartOutlineResult, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let part = self.save_manual_part_outline_validated(input).await?;
        Ok(SavePartOutlineResult {
            part: part_outline_summary(&part),
            warnings: Vec::new(),
        })
    }
}

fn chapter_outline_summary(
    outline: &chapter_outline::Model,
    source_kind: Option<&String>,
) -> ChapterOutlineSummary {
    ChapterOutlineSummary {
        id: outline.id,
        project_id: outline.project_id,
        part_id: outline.part_id,
        chapter_number: outline.chapter_number,
        title: outline.title.clone(),
        summary: outline.summary.clone(),
        beats: outline.beats.clone(),
        key_scenes: outline.key_scenes.clone(),
        pacing_score: outline.pacing_score,
        foreshadowing_setup: outline.foreshadowing_setup.clone(),
        foreshadowing_payoff: outline.foreshadowing_payoff.clone(),
        characters_involved: outline.characters_involved.clone(),
        word_count_target: outline.word_count_target,
        source_kind: source_kind.cloned(),
        status: outline.status.clone(),
        created_at: Some(outline.created_at.to_string()),
        updated_at: Some(outline.updated_at.to_string()),
    }
}

fn part_outline_summary(part: &part_outline::Model) -> PartOutlineSummary {
    PartOutlineSummary {
        id: part.id,
        project_id: part.project_id,
        part_number: part.part_number,
        title: part.title.clone(),
        start_chapter: part.start_chapter,
        end_chapter: part.end_chapter,
        summary: part.summary.clone(),
        theme: part.theme.clone(),
        key_events: part.key_events.clone(),
        character_focus: part.character_focus.clone(),
        created_at: Some(part.created_at.to_string()),
        updated_at: Some(part.updated_at.to_string()),
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::outline::OutlineReadApplicationPort for OrchestratorContext {
    async fn list_chapters(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ChapterOutlineSummary>, AppError> {
        self.list_chapter_outlines_for_user(user_id, project_id)
            .await
    }

    async fn list_parts(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<PartOutlineSummary>, AppError> {
        self.list_part_outlines_for_user(user_id, project_id).await
    }

    async fn generation_progress(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<OutlineGenerationProgressSummary, AppError> {
        self.outline_generation_progress_for_user(user_id, project_id)
            .await
    }

    async fn ensure_chapter_outline_access(
        &self,
        user_id: i64,
        project_id: i64,
        outline_id: i64,
    ) -> Result<(), AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let outline = self
            .chapter_outline_by_id(outline_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("章节大纲不存在: {outline_id}")))?;
        if outline.project_id != project_id {
            return Err(AppError::Forbidden(format!(
                "章节大纲 {outline_id} 不属于项目 {project_id}"
            )));
        }
        Ok(())
    }

    async fn ensure_part_access(
        &self,
        user_id: i64,
        project_id: i64,
        part_id: i64,
    ) -> Result<(), AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let part = self
            .part_outline_by_id(part_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("分部大纲不存在: {part_id}")))?;
        if part.project_id != project_id {
            return Err(AppError::Forbidden(format!(
                "分部大纲 {part_id} 不属于项目 {project_id}"
            )));
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::outline::OutlineMutationApplicationPort for OrchestratorContext {
    async fn delete_latest_chapters(
        &self,
        user_id: i64,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<OutlineMutationResult, AppError> {
        self.delete_latest_chapter_outlines_for_user(user_id, input)
            .await
    }

    async fn delete_latest_parts(
        &self,
        user_id: i64,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<OutlineMutationResult, AppError> {
        self.delete_latest_part_outlines_for_user(user_id, input)
            .await
    }

    async fn save_chapter(
        &self,
        user_id: i64,
        input: SaveChapterOutlineInput,
    ) -> Result<SaveChapterOutlineResult, AppError> {
        self.save_chapter_outline_for_user(user_id, input).await
    }

    async fn update_chapter(
        &self,
        user_id: i64,
        input: UpdateChapterOutlineInput,
    ) -> Result<UpdateChapterOutlineResult, AppError> {
        self.update_chapter_outline_for_user(user_id, input).await
    }

    async fn save_part(
        &self,
        user_id: i64,
        input: SavePartOutlineInput,
    ) -> Result<SavePartOutlineResult, AppError> {
        self.save_part_outline_for_user(user_id, input).await
    }
}

fn normalize_chapter_outline_update_input(
    input: UpdateChapterOutlineInput,
) -> Result<a4n_db::project::ProjectChapterOutlineUpdateInput, AppError> {
    ensure_positive_project_id(input.project_id)?;
    if input.outline_id <= 0 {
        return Err(AppError::InvalidArgument(
            "章节大纲 ID 必须大于 0".to_string(),
        ));
    }
    let expected_updated_at = input.expected_updated_at.trim().to_string();
    if expected_updated_at.is_empty() {
        return Err(AppError::InvalidArgument(
            "缺少章节大纲资料版本，请刷新后重试".to_string(),
        ));
    }
    let title = match input.title {
        Some(value) => Some(require_non_empty_outline_text(&value, "title")?),
        None => None,
    };
    let summary = match input.summary {
        Some(value) => Some(require_non_empty_outline_text(&value, "summary")?),
        None => None,
    };
    let beats = match input.beats {
        Some(value) => Some(require_outline_json_array(value, "beats")?),
        None => None,
    };
    let key_scenes = match input.key_scenes {
        Some(value) => Some(require_outline_json_array(value, "key_scenes")?),
        None => None,
    };
    Ok(a4n_db::project::ProjectChapterOutlineUpdateInput {
        project_id: input.project_id,
        outline_id: input.outline_id,
        expected_updated_at,
        title,
        summary,
        beats,
        key_scenes,
    })
}

fn delete_latest_outline_records_input(
    input: DeleteLatestOutlineRecordsInput,
) -> a4n_db::project::DeleteLatestOutlineRecordsInput {
    a4n_db::project::DeleteLatestOutlineRecordsInput {
        project_id: input.project_id,
        targets: input
            .targets
            .into_iter()
            .map(|target| a4n_db::project::OutlineDeleteTarget {
                id: target.id,
                expected_updated_at: target.expected_updated_at,
            })
            .collect(),
    }
}

fn require_non_empty_outline_text(value: &str, field: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "章节大纲字段 {field} 不能为空"
        )));
    }
    Ok(trimmed.to_string())
}

fn require_outline_json_array(value: Value, field: &str) -> Result<Value, AppError> {
    if value.is_array() {
        Ok(value)
    } else {
        Err(AppError::InvalidArgument(format!(
            "章节大纲字段 {field} 必须是数组"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn sample_outline() -> chapter_outline::Model {
        let now = NaiveDate::from_ymd_opt(2026, 6, 6)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        chapter_outline::Model {
            id: 42,
            project_id: 1,
            part_id: None,
            chapter_number: 3,
            title: Some("旧标题".to_string()),
            summary: Some("旧概要".to_string()),
            beats: None,
            key_scenes: None,
            pacing_score: None,
            foreshadowing_setup: None,
            foreshadowing_payoff: None,
            characters_involved: None,
            word_count_target: None,
            status: "draft".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn chapter_outline_summary_exposes_derived_source() {
        let summary = chapter_outline_summary(
            &sample_outline(),
            Some(&"retrospective_analysis".to_string()),
        );

        assert_eq!(
            summary.source_kind.as_deref(),
            Some("retrospective_analysis")
        );
    }
}
