use std::collections::{HashMap, HashSet};

use a4n_api_contract::outline::{
    ApplyBeatMappingInput, BatchGeneratePartOutlinesInput, ContinuePartOutlinesInput,
    GenerateChapterOutlinesByCountInput, GeneratePartChapterOutlinesInput, GeneratePartsInput,
    OutlineGenerationProgressSummary, RegenerateChapterOutlineInput, RegenerateLastPartInput,
    RegeneratePartOutlineInput, RegeneratePartsInput,
};
use a4n_core::{
    error::AppError,
    models::chapter::{chapter_outline, part_outline},
};
use a4n_domain::project::ProjectStatus;
use a4n_llm::provider::ChatRequest;
use a4n_prompt::variables::parse_template_variables_app_error;
use serde_json::json;

use crate::OrchestratorContext;
use crate::collaboration::ProjectAccessRole;

use super::{
    BeatMappingApplyInput, ChapterOutlineCountGenerationInput, ChapterOutlineGenerationInput,
    ChapterOutlineRegenerationInput, OutlineRequestPreviewStep, PartOutlineContinuationInput,
    PartOutlineGenerationInput, PartOutlineRegenerationInput, ProgressReporter,
    batch_parts_progress_total, order_parts_for_batch_generation, part_chapter_count,
    progress_total,
};

impl OrchestratorContext {
    pub async fn outline_generation_progress_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<OutlineGenerationProgressSummary, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        let rows = self.outline_generation_progress_rows(project_id).await?;
        let completed_parts = completed_part_count(&rows.parts, &rows.chapter_outlines)?;
        let completed_chapter_outlines = rows
            .chapter_outlines
            .iter()
            .filter(|outline| outline.status == chapter_outline::STATUS_COMPLETED)
            .count() as u64;

        Ok(OutlineGenerationProgressSummary {
            total_parts: rows.parts.len() as u64,
            completed_parts,
            total_chapter_outlines: rows.chapter_outlines.len() as u64,
            completed_chapter_outlines,
        })
    }

    pub async fn generate_parts_validated(
        &self,
        input: GeneratePartsInput,
        progress: ProgressReporter<'_>,
    ) -> Result<Vec<part_outline::Model>, AppError> {
        report_application_progress(progress, 0, 3, "启动分部大纲生成")?;
        let variables = parse_template_variables_app_error(&input.variables)?;
        let workflow_run_id = self
            .begin_outline_workflow_validated(
                input.project_id,
                "part_outline_generation",
                "generate",
                "part_1",
                input.task_id,
                json!({
                    "schema_version": 1,
                    "workflow": "part_outline_generation",
                    "mode": "generate",
                    "project_id": input.project_id,
                    "part_count": input.part_count,
                    "chapters_per_part": input.chapters_per_part,
                    "variables": &variables,
                }),
            )
            .await?
            .id;
        let result = async {
            let parts = self
                .generate_parts(PartOutlineGenerationInput {
                    project_id: input.project_id,
                    part_count: input.part_count,
                    chapters_per_part: input.chapters_per_part,
                    prompt_renderer: self,
                    variables: &variables,
                    progress,
                    workflow_run_id: Some(workflow_run_id),
                })
                .await?;
            self.advance_project_status_if_allowed(
                input.project_id,
                ProjectStatus::PartOutlinesReady,
            )
            .await?;
            report_application_progress(progress, 3, 3, "分部大纲已生成")?;
            Ok(parts)
        }
        .await;
        self.finalize_outline_application(workflow_run_id, result)
            .await
    }

    pub async fn regenerate_parts_validated(
        &self,
        input: RegeneratePartsInput,
        progress: ProgressReporter<'_>,
    ) -> Result<Vec<part_outline::Model>, AppError> {
        report_application_progress(progress, 0, 3, "启动分部大纲重生成")?;
        let variables = parse_template_variables_app_error(&input.variables)?;
        let workflow_run_id = self
            .begin_outline_workflow_validated(
                input.project_id,
                "part_outline_generation",
                "regenerate_all",
                "part_1",
                input.task_id,
                json!({
                    "schema_version": 1,
                    "workflow": "part_outline_generation",
                    "mode": "regenerate_all",
                    "project_id": input.project_id,
                    "expected_parts": &input.expected_parts,
                    "chapters_per_part": input.chapters_per_part,
                    "variables": &variables,
                }),
            )
            .await?
            .id;
        let result = async {
            let parts = self
                .regenerate_parts(PartOutlineRegenerationInput {
                    project_id: input.project_id,
                    expected_parts: &input.expected_parts,
                    chapters_per_part: input.chapters_per_part,
                    prompt_renderer: self,
                    variables: &variables,
                    progress,
                    workflow_run_id: Some(workflow_run_id),
                })
                .await?;
            self.advance_project_status_if_allowed(
                input.project_id,
                ProjectStatus::PartOutlinesReady,
            )
            .await?;
            report_application_progress(progress, 3, 3, "已重新生成所有部")?;
            Ok(parts)
        }
        .await;
        self.finalize_outline_application(workflow_run_id, result)
            .await
    }

    pub async fn regenerate_last_part_validated(
        &self,
        input: RegenerateLastPartInput,
        progress: ProgressReporter<'_>,
    ) -> Result<part_outline::Model, AppError> {
        report_application_progress(progress, 0, 3, "正在重新生成最后一部")?;
        let variables = parse_template_variables_app_error(&input.variables)?;
        let workflow_run_id = self
            .begin_outline_workflow_validated(
                input.project_id,
                "part_outline_generation",
                "regenerate_last",
                "part_latest",
                input.task_id,
                json!({
                    "schema_version": 1,
                    "workflow": "part_outline_generation",
                    "mode": "regenerate_last",
                    "project_id": input.project_id,
                    "variables": &variables,
                }),
            )
            .await?
            .id;
        let result = async {
            let part = self
                .regenerate_last_part(input.project_id, self, &variables, Some(workflow_run_id))
                .await?;
            report_application_progress(progress, 2, 3, "正在刷新项目状态")?;
            self.advance_project_status_if_allowed(
                input.project_id,
                ProjectStatus::PartOutlinesReady,
            )
            .await?;
            report_application_progress(progress, 3, 3, "最后一部已重新生成")?;
            Ok(part)
        }
        .await;
        self.finalize_outline_application(workflow_run_id, result)
            .await
    }

    pub async fn continue_part_outlines_validated(
        &self,
        input: ContinuePartOutlinesInput,
        progress: ProgressReporter<'_>,
    ) -> Result<part_outline::Model, AppError> {
        report_application_progress(
            progress,
            0,
            3,
            &format!("正在续作第 {} 部", input.part_number),
        )?;
        let variables = parse_template_variables_app_error(&input.variables)?;
        let workflow_run_id = self
            .begin_outline_workflow_validated(
                input.project_id,
                "part_outline_generation",
                "continue",
                &format!("part_{}", input.part_number),
                input.task_id,
                json!({
                    "schema_version": 1,
                    "workflow": "part_outline_generation",
                    "mode": "continue",
                    "project_id": input.project_id,
                    "part_number": input.part_number,
                    "start_chapter": input.start_chapter,
                    "end_chapter": input.end_chapter,
                    "variables": &variables,
                }),
            )
            .await?
            .id;
        let result = async {
            let part = self
                .continue_part_outline(PartOutlineContinuationInput {
                    project_id: input.project_id,
                    part_number: input.part_number,
                    start_chapter: input.start_chapter,
                    end_chapter: input.end_chapter,
                    prompt_renderer: self,
                    variables: &variables,
                    workflow_run_id: Some(workflow_run_id),
                })
                .await?;
            report_application_progress(progress, 2, 3, "正在刷新项目状态")?;
            self.advance_project_status_if_allowed(
                input.project_id,
                ProjectStatus::PartOutlinesReady,
            )
            .await?;
            report_application_progress(progress, 3, 3, "分部续作完成")?;
            Ok(part)
        }
        .await;
        self.finalize_outline_application(workflow_run_id, result)
            .await
    }

    pub async fn regenerate_part_outline_validated(
        &self,
        input: RegeneratePartOutlineInput,
        progress: ProgressReporter<'_>,
    ) -> Result<part_outline::Model, AppError> {
        report_application_progress(progress, 0, 3, "正在重新生成指定分部")?;
        let variables = parse_template_variables_app_error(&input.variables)?;
        let workflow_run_id = self
            .begin_outline_workflow_validated(
                input.project_id,
                "part_outline_generation",
                "regenerate_one",
                &format!("part_id_{}", input.part_id),
                input.task_id,
                json!({
                    "schema_version": 1,
                    "workflow": "part_outline_generation",
                    "mode": "regenerate_one",
                    "project_id": input.project_id,
                    "part_id": input.part_id,
                    "variables": &variables,
                }),
            )
            .await?
            .id;
        let result = async {
            let part = self
                .regenerate_part_outline(
                    input.project_id,
                    input.part_id,
                    self,
                    &variables,
                    Some(workflow_run_id),
                )
                .await?;
            report_application_progress(progress, 2, 3, "正在刷新项目状态")?;
            self.advance_project_status_if_allowed(
                input.project_id,
                ProjectStatus::PartOutlinesReady,
            )
            .await?;
            report_application_progress(progress, 3, 3, "指定分部已重新生成")?;
            Ok(part)
        }
        .await;
        self.finalize_outline_application(workflow_run_id, result)
            .await
    }

    pub async fn generate_part_chapter_outlines_validated(
        &self,
        input: GeneratePartChapterOutlinesInput,
        progress: ProgressReporter<'_>,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        let part = self.get_part(input.part_id).await?;
        ensure_part_project(&part, input.project_id)?;
        let count = part_chapter_count(part.start_chapter, part.end_chapter)?;
        report_application_progress(
            progress,
            0,
            progress_total(count)?,
            &format!("启动第 {} 部章节大纲生成", part.part_number),
        )?;
        let workflow_run_id = self
            .begin_outline_workflow_validated(
                input.project_id,
                "chapter_outline_generation",
                "generate_part",
                &format!("chapter_{}", part.start_chapter),
                input.task_id,
                json!({
                    "schema_version": 1,
                    "workflow": "chapter_outline_generation",
                    "mode": "generate_part",
                    "project_id": input.project_id,
                    "part_id": input.part_id,
                    "start_chapter": part.start_chapter,
                    "count": count,
                }),
            )
            .await?
            .id;
        let variables = HashMap::new();
        let result = async {
            let outlines = self
                .generate_chapter_outlines(ChapterOutlineGenerationInput {
                    project_id: input.project_id,
                    part_id: part.id,
                    start_chapter: part.start_chapter,
                    count,
                    prompt_renderer: self,
                    variables: &variables,
                    progress,
                    workflow_run_id: Some(workflow_run_id),
                })
                .await?;
            self.advance_project_status_if_allowed(
                input.project_id,
                ProjectStatus::ChapterOutlinesReady,
            )
            .await?;
            Ok(outlines)
        }
        .await;
        self.finalize_outline_application(workflow_run_id, result)
            .await
    }

    pub async fn batch_generate_part_outlines_validated(
        &self,
        input: BatchGeneratePartOutlinesInput,
        progress: ProgressReporter<'_>,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        let parts = self
            .load_unique_outline_parts(input.project_id, &input.part_ids)
            .await?;
        let total = batch_parts_progress_total(&parts)?;
        report_application_progress(
            progress,
            0,
            total,
            &format!("启动 {} 个分部的章节大纲生成", parts.len()),
        )?;
        let part_fingerprint_json = parts
            .iter()
            .map(|part| {
                json!({
                    "part_id": part.id,
                    "part_number": part.part_number,
                    "start_chapter": part.start_chapter,
                    "end_chapter": part.end_chapter,
                })
            })
            .collect::<Vec<_>>();
        let workflow_run_id = self
            .begin_outline_workflow_validated(
                input.project_id,
                "chapter_outline_generation",
                "generate_batch_parts",
                "chapter_batch",
                input.task_id,
                json!({
                    "schema_version": 1,
                    "workflow": "chapter_outline_generation",
                    "mode": "generate_batch_parts",
                    "project_id": input.project_id,
                    "parts": part_fingerprint_json,
                }),
            )
            .await?
            .id;
        let variables = HashMap::new();
        let result = async {
            let mut results = Vec::new();
            let mut completed = 0_u32;
            for part in parts {
                let count = part_chapter_count(part.start_chapter, part.end_chapter)?;
                let part_total = progress_total(count)?;
                let offset_progress = |current: u32, child_total: u32, message: &str| {
                    if child_total != part_total || current > child_total {
                        return Err(AppError::InvalidArgument(format!(
                            "分部章节大纲进度不一致：current={current}, total={child_total}, expected_total={part_total}"
                        )));
                    }
                    let batch_current = completed.checked_add(current).ok_or_else(|| {
                        AppError::InvalidArgument("批量章节大纲进度超出 u32 范围".to_string())
                    })?;
                    report_application_progress(progress, batch_current, total, message)
                };
                results.extend(
                    self.generate_chapter_outlines(ChapterOutlineGenerationInput {
                        project_id: input.project_id,
                        part_id: part.id,
                        start_chapter: part.start_chapter,
                        count,
                        prompt_renderer: self,
                        variables: &variables,
                        progress: Some(&offset_progress),
                        workflow_run_id: Some(workflow_run_id),
                    })
                    .await?,
                );
                completed = completed.checked_add(part_total).ok_or_else(|| {
                    AppError::InvalidArgument("批量章节大纲进度超出 u32 范围".to_string())
                })?;
            }
            self.advance_project_status_if_allowed(
                input.project_id,
                ProjectStatus::ChapterOutlinesReady,
            )
            .await?;
            Ok(results)
        }
        .await;
        self.finalize_outline_application(workflow_run_id, result)
            .await
    }

    pub async fn generate_chapter_outlines_by_count_validated(
        &self,
        input: GenerateChapterOutlinesByCountInput,
        progress: ProgressReporter<'_>,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        report_application_progress(
            progress,
            0,
            progress_total(input.count)?,
            "启动章节大纲追加生成",
        )?;
        let workflow_run_id = self
            .begin_outline_workflow_validated(
                input.project_id,
                "chapter_outline_generation",
                "generate_by_count",
                &format!("chapter_{}", input.start_chapter),
                input.task_id,
                json!({
                    "schema_version": 1,
                    "workflow": "chapter_outline_generation",
                    "mode": "generate_by_count",
                    "project_id": input.project_id,
                    "start_chapter": input.start_chapter,
                    "count": input.count,
                }),
            )
            .await?
            .id;
        let variables = HashMap::new();
        let result = async {
            let outlines = self
                .generate_chapter_outlines_by_count(ChapterOutlineCountGenerationInput {
                    project_id: input.project_id,
                    start_chapter: input.start_chapter,
                    count: input.count,
                    prompt_renderer: self,
                    variables: &variables,
                    progress,
                    workflow_run_id: Some(workflow_run_id),
                })
                .await?;
            self.advance_project_status_if_allowed(
                input.project_id,
                ProjectStatus::ChapterOutlinesReady,
            )
            .await?;
            Ok(outlines)
        }
        .await;
        self.finalize_outline_application(workflow_run_id, result)
            .await
    }

    pub async fn regenerate_chapter_outline_validated(
        &self,
        input: RegenerateChapterOutlineInput,
        progress: ProgressReporter<'_>,
    ) -> Result<chapter_outline::Model, AppError> {
        report_application_progress(progress, 0, 3, "启动章节大纲重生成")?;
        let variables = parse_template_variables_app_error(&input.variables)?;
        let workflow_run_id = self
            .begin_outline_workflow_validated(
                input.project_id,
                "chapter_outline_generation",
                "regenerate_one",
                &format!("outline_id_{}", input.outline_id),
                input.task_id,
                json!({
                    "schema_version": 1,
                    "workflow": "chapter_outline_generation",
                    "mode": "regenerate_one",
                    "project_id": input.project_id,
                    "outline_id": input.outline_id,
                    "variables": &variables,
                }),
            )
            .await?
            .id;
        let result = async {
            let outline = self
                .regenerate_chapter_outline(ChapterOutlineRegenerationInput {
                    project_id: input.project_id,
                    outline_id: input.outline_id,
                    prompt_renderer: self,
                    variables: &variables,
                    progress,
                    workflow_run_id: Some(workflow_run_id),
                })
                .await?;
            self.advance_project_status_if_allowed(
                input.project_id,
                ProjectStatus::ChapterOutlinesReady,
            )
            .await?;
            report_application_progress(progress, 3, 3, "章节大纲已重新生成")?;
            Ok(outline)
        }
        .await;
        self.finalize_outline_application(workflow_run_id, result)
            .await
    }

    pub async fn apply_beat_mapping_validated(
        &self,
        input: ApplyBeatMappingInput,
        progress: ProgressReporter<'_>,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        let part = self.get_part(input.part_id).await?;
        ensure_part_project(&part, input.project_id)?;
        report_application_progress(progress, 0, 3, "启动节拍映射")?;
        let workflow_run_id = self
            .begin_outline_workflow_validated(
                input.project_id,
                "chapter_outline_beat_mapping",
                "apply",
                &format!("part_id_{}", input.part_id),
                input.task_id,
                json!({
                    "schema_version": 1,
                    "workflow": "chapter_outline_beat_mapping",
                    "mode": "apply",
                    "project_id": input.project_id,
                    "part_id": input.part_id,
                    "beat_sheet": &input.beat_sheet,
                }),
            )
            .await?
            .id;
        let variables = HashMap::new();
        let result = async {
            let outlines = self
                .apply_beat_mapping(BeatMappingApplyInput {
                    project_id: input.project_id,
                    part_id: input.part_id,
                    beat_sheet: &input.beat_sheet,
                    prompt_renderer: self,
                    variables: &variables,
                    progress,
                    workflow_run_id: Some(workflow_run_id),
                })
                .await?;
            self.advance_project_status_if_allowed(
                input.project_id,
                ProjectStatus::ChapterOutlinesReady,
            )
            .await?;
            Ok(outlines)
        }
        .await;
        self.finalize_outline_application(workflow_run_id, result)
            .await
    }

    pub async fn preview_generate_parts_validated(
        &self,
        input: GeneratePartsInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        let variables = parse_template_variables_app_error(&input.variables)?;
        self.preview_generate_parts_requests(
            input.project_id,
            input.part_count,
            input.chapters_per_part,
            &[],
            false,
            self,
            &variables,
        )
        .await
    }

    pub async fn preview_regenerate_parts_validated(
        &self,
        input: RegeneratePartsInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        let variables = parse_template_variables_app_error(&input.variables)?;
        self.preview_generate_parts_requests(
            input.project_id,
            None,
            input.chapters_per_part,
            &input.expected_parts,
            true,
            self,
            &variables,
        )
        .await
    }

    pub async fn preview_regenerate_last_part_validated(
        &self,
        input: RegenerateLastPartInput,
    ) -> Result<ChatRequest, AppError> {
        let variables = parse_template_variables_app_error(&input.variables)?;
        self.preview_regenerate_last_part_request(input.project_id, self, &variables)
            .await
    }

    pub async fn preview_continue_part_outlines_validated(
        &self,
        input: ContinuePartOutlinesInput,
    ) -> Result<ChatRequest, AppError> {
        let variables = parse_template_variables_app_error(&input.variables)?;
        self.preview_continue_part_outline_request(
            input.project_id,
            input.part_number,
            input.start_chapter,
            input.end_chapter,
            self,
            &variables,
        )
        .await
    }

    pub async fn preview_regenerate_part_outline_validated(
        &self,
        input: RegeneratePartOutlineInput,
    ) -> Result<ChatRequest, AppError> {
        let part = self.get_part(input.part_id).await?;
        ensure_part_project(&part, input.project_id)?;
        let variables = parse_template_variables_app_error(&input.variables)?;
        self.preview_regenerate_part_outline_request(
            input.project_id,
            input.part_id,
            self,
            &variables,
        )
        .await
    }

    pub async fn preview_generate_part_chapter_outlines_validated(
        &self,
        input: GeneratePartChapterOutlinesInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        let part = self.get_part(input.part_id).await?;
        ensure_part_project(&part, input.project_id)?;
        let variables = HashMap::new();
        self.preview_generate_part_chapter_outline_requests(
            input.project_id,
            input.part_id,
            self,
            &variables,
        )
        .await
    }

    pub async fn preview_batch_generate_part_outlines_validated(
        &self,
        input: BatchGeneratePartOutlinesInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        let parts = self
            .load_unique_outline_parts(input.project_id, &input.part_ids)
            .await?;
        let variables = HashMap::new();
        let mut all_steps = Vec::new();
        for part in parts {
            let steps = self
                .preview_generate_part_chapter_outline_requests(
                    input.project_id,
                    part.id,
                    self,
                    &variables,
                )
                .await?;
            let has_request = steps.iter().any(|step| step.request.is_some());
            all_steps.extend(steps);
            if has_request {
                break;
            }
        }
        Ok(all_steps)
    }

    pub async fn preview_generate_chapter_outlines_by_count_validated(
        &self,
        input: GenerateChapterOutlinesByCountInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        let variables = HashMap::new();
        self.preview_generate_chapter_outlines_by_count_requests(
            input.project_id,
            input.start_chapter,
            input.count,
            self,
            &variables,
        )
        .await
    }

    pub async fn preview_regenerate_chapter_outline_validated(
        &self,
        input: RegenerateChapterOutlineInput,
    ) -> Result<ChatRequest, AppError> {
        let variables = parse_template_variables_app_error(&input.variables)?;
        self.preview_regenerate_chapter_outline_request(
            input.project_id,
            input.outline_id,
            self,
            &variables,
        )
        .await
    }

    pub async fn preview_apply_beat_mapping_validated(
        &self,
        input: ApplyBeatMappingInput,
    ) -> Result<ChatRequest, AppError> {
        let part = self.get_part(input.part_id).await?;
        ensure_part_project(&part, input.project_id)?;
        let variables = HashMap::new();
        self.preview_beat_mapping_request(
            input.project_id,
            input.part_id,
            &input.beat_sheet,
            self,
            &variables,
        )
        .await
    }

    async fn load_unique_outline_parts(
        &self,
        project_id: i64,
        part_ids: &[i64],
    ) -> Result<Vec<part_outline::Model>, AppError> {
        if part_ids.is_empty() {
            return Err(AppError::InvalidArgument("part_ids 不能为空".to_string()));
        }
        let mut seen = HashSet::new();
        let mut parts = Vec::with_capacity(part_ids.len());
        for part_id in part_ids {
            if !seen.insert(*part_id) {
                return Err(AppError::InvalidArgument(format!(
                    "part_ids 包含重复分部: {part_id}"
                )));
            }
            let part = self.get_part(*part_id).await?;
            ensure_part_project(&part, project_id)?;
            parts.push(part);
        }
        order_parts_for_batch_generation(&mut parts);
        Ok(parts)
    }

    async fn finalize_outline_application<T>(
        &self,
        workflow_run_id: i64,
        result: Result<T, AppError>,
    ) -> Result<T, AppError> {
        self.finalize_workflow_result(workflow_run_id, result, "大纲生成")
            .await
    }
}

fn ensure_part_project(part: &part_outline::Model, project_id: i64) -> Result<(), AppError> {
    if part.project_id == project_id {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "分部大纲 {} 不属于项目 {project_id}",
            part.id
        )))
    }
}

pub(super) fn completed_part_count(
    parts: &[part_outline::Model],
    outlines: &[chapter_outline::Model],
) -> Result<u64, AppError> {
    let mut outlines_by_chapter = HashMap::<i32, &chapter_outline::Model>::new();
    for outline in outlines {
        if !chapter_outline::is_valid_status(&outline.status) {
            return Err(outline_progress_state_error(format!(
                "章节大纲 {} 状态无效：{}",
                outline.id, outline.status
            )));
        }
        if outlines_by_chapter
            .insert(outline.chapter_number, outline)
            .is_some()
        {
            return Err(outline_progress_state_error(format!(
                "章节号 {} 存在重复章节大纲",
                outline.chapter_number
            )));
        }
    }

    let mut claimed_chapters = HashSet::<i32>::new();
    let mut completed = 0_u64;
    for part in parts {
        if part.start_chapter <= 0 || part.end_chapter < part.start_chapter {
            return Err(outline_progress_state_error(format!(
                "分部 {} 章节范围无效：{}..={}",
                part.id, part.start_chapter, part.end_chapter
            )));
        }
        for chapter_number in part.start_chapter..=part.end_chapter {
            if !claimed_chapters.insert(chapter_number) {
                return Err(outline_progress_state_error(format!(
                    "章节号 {} 被多个分部范围覆盖",
                    chapter_number
                )));
            }
        }

        if (part.start_chapter..=part.end_chapter).all(|chapter_number| {
            outlines_by_chapter
                .get(&chapter_number)
                .is_some_and(|outline| outline.status == chapter_outline::STATUS_COMPLETED)
        }) {
            completed += 1;
        }
    }

    Ok(completed)
}

fn outline_progress_state_error(current: String) -> AppError {
    AppError::InvalidState {
        current,
        operation: "查询大纲生成进度".to_string(),
    }
}

fn report_application_progress(
    progress: ProgressReporter<'_>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(report) = progress {
        report(current, total.max(1), message)?;
    }
    Ok(())
}
