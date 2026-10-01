use a4n_chapter_analysis_host_contract::{ChapterAnalysisHostPort, ChapterAnalysisRequestStep};
use a4n_error::AppError;
use a4n_import_analysis_host_contract::{
    ImportAnalysisGenerationApplicationPort, ImportAnalysisPreviewApplicationPort,
};
use a4n_prompt::rendering::PromptRenderer;

use crate::{OrchestratorContext, character::CharacterProfileSyncStageTracePort};

use super::{ImportProjectAnalysisSummary, ProjectImportAnalysisProgress};

#[async_trait::async_trait]
impl a4n_api_contract::import::ImportAnalysisProgressApplicationPort for OrchestratorContext {
    async fn progress(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Option<ProjectImportAnalysisProgress>, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        super::lifecycle::project_import_analysis_progress(self, project_id).await
    }
}

#[async_trait::async_trait]
impl ImportAnalysisGenerationApplicationPort for OrchestratorContext {
    async fn run(
        &self,
        user_id: i64,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        chapter_analysis_host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ImportProjectAnalysisSummary, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.run_import_project_analysis_validated(
            user_id,
            project_id,
            prompt_renderer,
            chapter_analysis_host_port,
            character_trace_port,
        )
        .await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::import::ImportAnalysisCancellationApplicationPort for OrchestratorContext {
    async fn cancel(&self, user_id: i64, project_id: i64) -> Result<(), AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        super::lifecycle::cancel_import_analysis(self, project_id).await
    }
}

#[async_trait::async_trait]
impl ImportAnalysisPreviewApplicationPort for OrchestratorContext {
    async fn preview(
        &self,
        user_id: i64,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<ChapterAnalysisRequestStep>, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.prepare_import_project_analysis_preview_validated(project_id, prompt_renderer)
            .await
    }
}
