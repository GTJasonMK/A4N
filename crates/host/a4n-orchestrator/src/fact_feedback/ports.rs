use a4n_api_contract::fact_feedback::{
    FactFeedbackMode, ProjectFactFeedbackSummary, RunProjectFactFeedbackInput,
};
use a4n_chapter_analysis_host_contract::{
    ChapterAnalysisApplicationProgress, ChapterAnalysisHostPort, ChapterAnalysisRequestPreview,
    ChapterAnalysisRequestStep, ProjectFactFeedbackApplicationPort,
};
use a4n_character_host_contract::CharacterProfileSyncStageTracePort;
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;

use crate::OrchestratorContext;

use super::FACT_CONSOLIDATION_PROMPT;

#[async_trait::async_trait]
impl ProjectFactFeedbackApplicationPort for OrchestratorContext {
    async fn feedback_summary(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ProjectFactFeedbackSummary, AppError> {
        self.fact_feedback_summary_for_user(user_id, project_id)
            .await
    }

    async fn run_feedback(
        &self,
        user_id: i64,
        input: RunProjectFactFeedbackInput,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ProjectFactFeedbackSummary, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        match input.mode {
            FactFeedbackMode::Consolidate => {
                self.consolidate_facts(
                    input.project_id,
                    true,
                    input.workflow_run_id,
                    prompt_renderer,
                    host_port,
                    progress,
                )
                .await?;
            }
            FactFeedbackMode::Rebuild => {
                self.rebuild_facts(
                    user_id,
                    &input,
                    prompt_renderer,
                    host_port,
                    progress,
                    character_trace_port,
                )
                .await?
            }
        }
        self.fact_feedback_summary_for_user(user_id, input.project_id)
            .await
    }

    async fn preview_feedback(
        &self,
        user_id: i64,
        input: RunProjectFactFeedbackInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChapterAnalysisRequestPreview, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let step = match input.mode {
            FactFeedbackMode::Consolidate => {
                let prepared = self
                    .prepare_consolidation(input.project_id, true, prompt_renderer)
                    .await?
                    .ok_or_else(|| AppError::InvalidArgument("请先完成章节分析".into()))?;
                ChapterAnalysisRequestStep {
                    key: FACT_CONSOLIDATION_PROMPT.into(),
                    title: "正文事实归纳".into(),
                    prompt_name: FACT_CONSOLIDATION_PROMPT.into(),
                    request: prepared.request,
                }
            }
            FactFeedbackMode::Rebuild => {
                let chapter = self
                    .chapters_for_project(input.project_id)
                    .await?
                    .into_iter()
                    .find(|chapter| chapter.has_available_content())
                    .ok_or_else(|| AppError::InvalidArgument("项目没有可分析正文".into()))?;
                crate::chapter_analysis::build_chapter_analysis_initial_request(
                    &chapter,
                    prompt_renderer,
                )
                .await?
            }
        };
        Ok(ChapterAnalysisRequestPreview {
            title: "正文事实反馈请求".into(),
            steps: vec![step],
        })
    }
}
