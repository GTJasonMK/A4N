use a4n_core::{error::AppError, models::chapter::chapter};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use crate::{
    OrchestratorContext,
    chapter_analysis::{
        ChapterAnalysisApplicationProgress, ChapterAnalysisHostPort, ChapterAnalysisPreviewStep,
    },
    character::CharacterProfileSyncStageTracePort,
};

use super::{
    ImportProjectAnalysisSummary,
    blueprint::upsert_import_blueprint,
    chapter_outline::importable_chapters,
    lifecycle::cancelled_import_analysis_summary,
    progress::{
        STAGE_ANALYSIS, STAGE_BLUEPRINT, STAGE_OUTLINES, import_analysis_cancelled,
        progress_stage_completed,
    },
    summary::completed_import_stage,
    workflow::{
        ImportAnalysisStageResult, PreparedImportAnalysisRun, complete_import_project_analysis,
        prepare_import_project_analysis, record_import_analysis_failure,
        run_import_analysis_outlines, run_import_analysis_parts, save_import_stage,
    },
};

impl OrchestratorContext {
    pub async fn run_import_project_analysis_validated(
        &self,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        chapter_analysis_host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ImportProjectAnalysisSummary, AppError> {
        let mut prepared = match prepare_import_project_analysis(self, project_id)
            .await
            .map_err(import_analysis_error)?
        {
            PreparedImportAnalysisRun::Completed(summary) => return Ok(summary),
            PreparedImportAnalysisRun::Ready(prepared) => prepared,
        };

        let result = self
            .execute_import_project_analysis(
                project_id,
                &prepared.project,
                &mut prepared.chapters,
                prepared.needs_part_outlines,
                &mut prepared.progress,
                prompt_renderer,
                chapter_analysis_host_port,
                character_trace_port,
            )
            .await;
        match result {
            Ok(summary) => Ok(summary),
            Err(error) => {
                let error_text = error.to_string();
                record_import_analysis_failure(self, project_id, &error_text)
                    .await
                    .map_err(import_analysis_error)?;
                Err(error)
            }
        }
    }

    pub async fn prepare_import_project_analysis_preview_validated(
        &self,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<ChapterAnalysisPreviewStep>, AppError> {
        let chapters = importable_chapters(self, project_id)
            .await
            .map_err(import_analysis_error)?;
        if chapters.is_empty() {
            return Err(AppError::InvalidState {
                current: "no_importable_chapters".to_string(),
                operation: "预览导入项目分析请求".to_string(),
            });
        }
        let mut steps = Vec::with_capacity(chapters.len() * 4 + 2);
        for chapter in chapters {
            for request in self
                .build_chapter_analysis_requests(chapter.id, prompt_renderer)
                .await?
            {
                steps.push(ChapterAnalysisPreviewStep {
                    key: format!("import_chapter_analysis_{}_{}", chapter.id, request.key),
                    title: format!("第 {} 章{}", chapter.chapter_number, request.title),
                    prompt_name: Some(request.prompt_name),
                    status: "ready".to_string(),
                    note: Some("导入项目分析的分析阶段会按顺序发送该请求".to_string()),
                    request: Some(request.request),
                });
            }
        }
        steps.push(ChapterAnalysisPreviewStep {
            key: "import_analysis_character_sync".to_string(),
            title: "导入章节角色发现和角色同步请求".to_string(),
            prompt_name: Some("character_discovery".to_string()),
            status: "blocked".to_string(),
            note: Some(
                "这些请求依赖对应章节结构化分析输出，运行前无法提前构造真实请求".to_string(),
            ),
            request: None,
        });
        steps.push(ChapterAnalysisPreviewStep {
            key: "import_analysis_reverse_blueprint".to_string(),
            title: "导入蓝图反推请求".to_string(),
            prompt_name: Some("reverse_blueprint".to_string()),
            status: "blocked".to_string(),
            note: Some(
                "该请求依赖前序章节分析和大纲更新后的上下文，运行前无法提前构造真实请求"
                    .to_string(),
            ),
            request: None,
        });
        Ok(steps)
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_import_project_analysis(
        &self,
        project_id: i64,
        project: &a4n_core::models::project::novel_project::Model,
        chapters: &mut Vec<chapter::Model>,
        needs_part_outlines: bool,
        progress: &mut Value,
        prompt_renderer: &dyn PromptRenderer,
        chapter_analysis_host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ImportProjectAnalysisSummary, AppError> {
        let total_chapters = chapters.len();
        let mut analyzed = completed_import_stage(progress, STAGE_ANALYSIS, total_chapters)
            .map_err(import_analysis_error)?;
        if analyzed < total_chapters {
            match self
                .run_import_analysis_chapter_stage(
                    project_id,
                    chapters,
                    analyzed,
                    progress,
                    prompt_renderer,
                    chapter_analysis_host_port,
                    character_trace_port,
                )
                .await?
            {
                ImportAnalysisStageResult::Complete(value) => analyzed = value,
                ImportAnalysisStageResult::Cancelled(value) => {
                    let outlined = completed_import_stage(progress, STAGE_OUTLINES, total_chapters)
                        .map_err(import_analysis_error)?;
                    return cancelled_import_analysis_summary(
                        self,
                        project_id,
                        progress,
                        value,
                        outlined,
                        needs_part_outlines,
                        chapters.len(),
                    )
                    .await
                    .map_err(import_analysis_error);
                }
            }
        } else {
            analyzed = total_chapters;
        }

        *chapters = importable_chapters(self, project_id)
            .await
            .map_err(import_analysis_error)?;
        let mut outlined = completed_import_stage(progress, STAGE_OUTLINES, chapters.len())
            .map_err(import_analysis_error)?;
        if outlined < chapters.len() {
            match run_import_analysis_outlines(self, project_id, chapters, outlined, progress)
                .await
                .map_err(import_analysis_error)?
            {
                ImportAnalysisStageResult::Complete(value) => outlined = value,
                ImportAnalysisStageResult::Cancelled(value) => {
                    return cancelled_import_analysis_summary(
                        self,
                        project_id,
                        progress,
                        analyzed,
                        value,
                        needs_part_outlines,
                        chapters.len(),
                    )
                    .await
                    .map_err(import_analysis_error);
                }
            }
        } else {
            outlined = chapters.len();
        }

        self.run_import_analysis_blueprint_stage(
            project,
            project_id,
            chapters,
            needs_part_outlines,
            progress,
            prompt_renderer,
        )
        .await?;
        if import_analysis_cancelled(self, project_id)
            .await
            .map_err(import_analysis_error)?
        {
            return cancelled_import_analysis_summary(
                self,
                project_id,
                progress,
                analyzed,
                outlined,
                needs_part_outlines,
                chapters.len(),
            )
            .await
            .map_err(import_analysis_error);
        }
        let generated_parts =
            run_import_analysis_parts(self, project_id, chapters, needs_part_outlines, progress)
                .await
                .map_err(import_analysis_error)?;
        complete_import_project_analysis(
            self,
            project_id,
            progress,
            analyzed,
            outlined,
            generated_parts,
        )
        .await
        .map_err(import_analysis_error)
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_import_analysis_chapter_stage(
        &self,
        project_id: i64,
        chapters: &[chapter::Model],
        mut analyzed: usize,
        progress: &mut Value,
        prompt_renderer: &dyn PromptRenderer,
        chapter_analysis_host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ImportAnalysisStageResult, AppError> {
        let total = chapters.len();
        save_import_stage(
            self,
            project_id,
            progress,
            STAGE_ANALYSIS,
            analyzed,
            total,
            format!("正在分析第{}章", chapters[analyzed].chapter_number),
        )
        .await
        .map_err(import_analysis_error)?;
        for chapter in chapters.iter().skip(analyzed) {
            if import_analysis_cancelled(self, project_id)
                .await
                .map_err(import_analysis_error)?
            {
                return Ok(ImportAnalysisStageResult::Cancelled(analyzed));
            }
            save_import_stage(
                self,
                project_id,
                progress,
                STAGE_ANALYSIS,
                analyzed,
                total,
                format!("正在分析第{}章", chapter.chapter_number),
            )
            .await
            .map_err(import_analysis_error)?;
            self.run_chapter_analysis_validated(
                chapter.id,
                prompt_renderer,
                chapter_analysis_host_port,
                ChapterAnalysisApplicationProgress::default(),
                character_trace_port,
            )
            .await
            .map_err(|error| AppError::InvalidState {
                current: format!("import_chapter_{}", chapter.chapter_number),
                operation: format!("结构化分析失败: {error}"),
            })?;
            analyzed += 1;
            save_import_stage(
                self,
                project_id,
                progress,
                STAGE_ANALYSIS,
                analyzed,
                total,
                format!("已完成第{}章分析", chapter.chapter_number),
            )
            .await
            .map_err(import_analysis_error)?;
        }
        save_import_stage(
            self,
            project_id,
            progress,
            STAGE_ANALYSIS,
            total,
            total,
            "分析数据生成完成".to_owned(),
        )
        .await
        .map_err(import_analysis_error)?;
        Ok(ImportAnalysisStageResult::Complete(total))
    }

    async fn run_import_analysis_blueprint_stage(
        &self,
        project: &a4n_core::models::project::novel_project::Model,
        project_id: i64,
        chapters: &[chapter::Model],
        needs_part_outlines: bool,
        progress: &mut Value,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<(), AppError> {
        if progress_stage_completed(progress, STAGE_BLUEPRINT).map_err(import_analysis_error)? >= 1
        {
            return Ok(());
        }
        save_import_stage(
            self,
            project_id,
            progress,
            STAGE_BLUEPRINT,
            0,
            1,
            "正在生成导入蓝图".to_owned(),
        )
        .await
        .map_err(import_analysis_error)?;
        upsert_import_blueprint(self, project, chapters, needs_part_outlines)
            .await
            .map_err(import_analysis_error)?;
        let applied = self
            .reverse_apply_blueprint_from_context_validated(project_id, 50, prompt_renderer)
            .await?;
        let character_count = required_applied_count(&applied, "character_count")?;
        let relationship_count = required_applied_count(&applied, "relationship_count")?;
        tracing::info!(
            project_id,
            character_count,
            relationship_count,
            "项目蓝图反推完成"
        );
        save_import_stage(
            self,
            project_id,
            progress,
            STAGE_BLUEPRINT,
            1,
            1,
            "导入蓝图已生成".to_owned(),
        )
        .await
        .map_err(import_analysis_error)
    }
}

fn required_applied_count(value: &Value, field: &str) -> Result<u64, AppError> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| AppError::InvalidState {
            current: "reverse_blueprint_result".to_string(),
            operation: format!("读取非负整数字段 {field}"),
        })
}

fn import_analysis_error(error: impl ToString) -> AppError {
    AppError::InvalidState {
        current: "import_project_analysis".to_string(),
        operation: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reverse_blueprint_counts_are_strict() {
        assert_eq!(
            required_applied_count(&json!({"character_count": 2}), "character_count").unwrap(),
            2
        );
        assert!(required_applied_count(&json!({}), "character_count").is_err());
        assert!(
            required_applied_count(&json!({"character_count": -1}), "character_count").is_err()
        );
    }
}
