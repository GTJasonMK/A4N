use std::collections::HashMap;

use a4n_api_contract::outline::{
    DeleteLatestOutlineRecordsInput, SaveChapterOutlineInput, SavePartOutlineInput,
    UpdateChapterOutlineInput,
};
use a4n_core::{
    error::AppError,
    models::chapter::{chapter_outline, part_outline},
};
use a4n_db::{DbHandle, project::OutlineGenerationProgressRows};

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
    pub async fn chapter_outline_by_project_number(
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

    pub async fn chapter_outline_source_kinds_for_ids(
        &self,
        outline_ids: &[i64],
    ) -> Result<HashMap<i64, String>, AppError> {
        self.project_outline_records
            .chapter_outline_source_kinds_for_ids(outline_ids)
            .await
    }

    pub async fn chapter_outline_by_id(
        &self,
        outline_id: i64,
    ) -> Result<Option<chapter_outline::Model>, AppError> {
        self.project_outline_records
            .chapter_outline_by_id(outline_id)
            .await
    }

    /// 更新章节大纲：ID 门禁 + 可选字段规范化后落库。
    pub async fn update_chapter_outline_validated(
        &self,
        input: UpdateChapterOutlineInput,
    ) -> Result<chapter_outline::Model, AppError> {
        self.project_outline_records
            .update_chapter_outline_validated(input)
            .await
    }

    pub async fn save_manual_chapter_outline_validated(
        &self,
        input: SaveChapterOutlineInput,
    ) -> Result<chapter_outline::Model, AppError> {
        self.project_outline_records
            .save_manual_chapter_outline_validated(input)
            .await
    }

    pub async fn save_manual_part_outline_validated(
        &self,
        input: SavePartOutlineInput,
    ) -> Result<part_outline::Model, AppError> {
        self.project_outline_records
            .save_manual_part_outline_validated(input)
            .await
    }

    pub async fn delete_latest_part_outline_records_validated(
        &self,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<(), AppError> {
        self.project_outline_records
            .delete_latest_part_outline_records_validated(input)
            .await
    }

    pub async fn delete_latest_chapter_outline_records_validated(
        &self,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<(), AppError> {
        self.project_outline_records
            .delete_latest_chapter_outline_records_validated(input)
            .await
    }

    pub async fn chapter_outlines_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        self.project_outline_records
            .chapter_outlines_for_project(project_id)
            .await
    }

    pub async fn part_outlines_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<part_outline::Model>, AppError> {
        self.project_outline_records
            .part_outlines_for_project(project_id)
            .await
    }

    pub async fn part_outline_by_id(
        &self,
        part_id: i64,
    ) -> Result<Option<part_outline::Model>, AppError> {
        self.project_outline_records
            .part_outline_by_id(part_id)
            .await
    }

    pub async fn outline_generation_progress_rows(
        &self,
        project_id: i64,
    ) -> Result<OutlineGenerationProgressRows, AppError> {
        self.project_outline_records
            .outline_generation_progress_rows(project_id)
            .await
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
