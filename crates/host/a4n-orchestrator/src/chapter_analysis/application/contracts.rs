#[async_trait]
impl ChapterAnalysisGenerationApplicationPort for OrchestratorContext {
    async fn preflight_generate(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<ChapterSummary, AppError> {
        let chapter = self
            .preflight_chapter_analysis_validated(chapter_id, false)
            .await?;
        self.verify_project_permission(
            chapter.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.chapter_summary_for_user(user_id, chapter.id).await
    }

    async fn generate(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ChapterAnalysisMutationResult, AppError> {
        let chapter = self
            .preflight_chapter_analysis_validated(chapter_id, false)
            .await?;
        self.verify_project_permission(
            chapter.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        match self
            .run_chapter_analysis_validated(
                chapter_id,
                prompt_renderer,
                host_port,
                progress,
                character_trace_port,
            )
            .await
        {
            Ok(chapter) => Ok(ChapterAnalysisMutationResult {
                chapter: self.chapter_summary_for_user(user_id, chapter.id).await?,
                warnings: Vec::new(),
            }),
            Err(sync_error) => {
                let summary = self.chapter_summary_for_user(user_id, chapter_id).await?;
                if !summary.analysis_sync_has_saved_analysis || !summary.analysis_sync_can_resume {
                    return Err(sync_error);
                }
                Ok(ChapterAnalysisMutationResult {
                    chapter: summary,
                    warnings: vec![format!(
                        "结构化分析已保存，但后续资料同步未完成：{sync_error}"
                    )],
                })
            }
        }
    }
}

#[async_trait]
impl ChapterAnalysisResumeApplicationPort for OrchestratorContext {
    async fn preflight_resume(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<ChapterSummary, AppError> {
        let chapter = self
            .preflight_resume_chapter_analysis_validated(chapter_id)
            .await?;
        self.verify_project_permission(
            chapter.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.chapter_summary_for_user(user_id, chapter.id).await
    }

    async fn resume_sync(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ChapterSummary, AppError> {
        let chapter = self
            .preflight_resume_chapter_analysis_validated(chapter_id)
            .await?;
        self.verify_project_permission(
            chapter.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        let chapter = self
            .resume_chapter_analysis_sync_validated(
                chapter_id,
                prompt_renderer,
                host_port,
                progress,
                character_trace_port,
            )
            .await?;
        self.chapter_summary_for_user(user_id, chapter.id).await
    }

    async fn resume_workflow_run(
        &self,
        user_id: i64,
        workflow_run_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<(), AppError> {
        let run = self.workflow_run_model_by_id(workflow_run_id).await?;
        let chapter_id = run.chapter_id.ok_or_else(|| {
            AppError::InvalidArgument("章节结构化分析工作流缺少 chapter_id".to_string())
        })?;
        let chapter = self
            .chapter_by_id(chapter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: chapter_id={chapter_id}")))?;
        self.verify_project_permission(
            chapter.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.resume_chapter_analysis_workflow_run_validated(
            &run,
            prompt_renderer,
            host_port,
            character_trace_port,
        )
        .await
    }
}

#[async_trait]
impl ChapterAnalysisGenerationPreviewApplicationPort for OrchestratorContext {
    async fn preview(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChapterAnalysisRequestPreview, AppError> {
        let chapter = self
            .preflight_chapter_analysis_validated(chapter_id, false)
            .await?;
        self.verify_project_permission(
            chapter.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        let step = build_chapter_analysis_initial_request(&chapter, prompt_renderer).await?;
        Ok(ChapterAnalysisRequestPreview {
            title: format!("第 {} 章结构化分析请求预览", chapter.chapter_number),
            steps: vec![step],
        })
    }
}

#[async_trait]
impl ChapterAnalysisResumePreviewApplicationPort for OrchestratorContext {
    async fn preview_resume(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChapterAnalysisRequestPreview, AppError> {
        let chapter = self
            .preflight_resume_chapter_analysis_validated(chapter_id)
            .await?;
        self.verify_project_permission(
            chapter.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        Ok(ChapterAnalysisRequestPreview {
            title: format!("第 {} 章分析继续同步请求预览", chapter.chapter_number),
            steps: self
                .prepare_resume_chapter_analysis_preview_validated(&chapter, prompt_renderer)
                .await?,
        })
    }
}

fn ensure_chapter_not_generating(chapter: &chapter::Model) -> Result<(), AppError> {
    if chapter.is_generating() {
        Err(AppError::InvalidState {
            current: "chapter_generating".to_string(),
            operation: format!("处理第 {} 章结构化分析", chapter.chapter_number),
        })
    } else {
        Ok(())
    }
}

fn workflow_contract_error(error: impl ToString) -> AppError {
    AppError::InvalidState {
        current: "chapter_analysis_checkpoint".to_string(),
        operation: error.to_string(),
    }
}

fn replacement_cleanup_artifacts_from_checkpoint(
    checkpoint: &Value,
) -> Result<ChapterDerivedCleanupArtifacts, AppError> {
    let artifacts = required_checkpoint_field(
        checkpoint,
        CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP,
        "cleanup_artifacts",
    )
    .map_err(workflow_contract_error)?;
    serde_json::from_value(artifacts).map_err(|error| AppError::InvalidState {
        current: "chapter_analysis_replacement_commit".to_string(),
        operation: format!("章节分析替换清理产物合同无效: {error}"),
    })
}
