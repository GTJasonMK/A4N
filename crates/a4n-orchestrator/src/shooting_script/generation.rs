mod design;
mod screenplay;

use a4n_core::error::AppError;
use tokio_util::sync::CancellationToken;

use crate::OrchestratorContext;

use self::{design::generate_shooting_scenes, screenplay::generate_screenplay_plan};
use super::{
    assembly::assemble_chapter_shooting_script,
    contracts::{
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL, ChapterShootingScriptSourceInput,
        RegenerateChapterShootingScriptInput, ResolvedChapterShootingScript,
        ShootingScriptProgressReporter,
    },
    screenplay::{validate_script_against_plan, validate_script_source_spans},
};

impl OrchestratorContext {
    pub async fn generate_chapter_shooting_script_for_user_validated(
        &self,
        user_id: i64,
        source: ChapterShootingScriptSourceInput,
        progress: ShootingScriptProgressReporter<'_>,
        cancel_token: Option<&CancellationToken>,
        active_task_id: Option<&str>,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, source.project_id)
            .await?;
        self.generate_chapter_shooting_script_with_progress(
            source,
            progress,
            cancel_token,
            active_task_id,
        )
            .await
    }

    pub async fn generate_chapter_shooting_script_with_progress(
        &self,
        source: ChapterShootingScriptSourceInput,
        progress: ShootingScriptProgressReporter<'_>,
        cancel_token: Option<&CancellationToken>,
        active_task_id: Option<&str>,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        let target_revision = self
            .next_chapter_shooting_script_revision(source.clone())
            .await?;
        let workflow = self
            .begin_chapter_shooting_script_workflow_validated(
                source.project_id,
                source.source_chapter_id,
                target_revision,
                active_task_id,
            )
            .await?;
        let result = self
            .regenerate_chapter_shooting_script_with_progress(
                RegenerateChapterShootingScriptInput {
                    project_id: source.project_id,
                    source_chapter_id: source.source_chapter_id,
                    workflow_run_id: workflow.id,
                },
                progress,
                cancel_token,
            )
            .await;
        self.finalize_workflow_outcome_result(workflow.id, result, "章节拍摄剧本生成")
            .await
    }

    pub(crate) async fn regenerate_chapter_shooting_script_with_progress(
        &self,
        input: RegenerateChapterShootingScriptInput,
        progress: ShootingScriptProgressReporter<'_>,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        let workflow_run_id = input.workflow_run_id;
        let progress_total = CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL;
        progress(0, progress_total, "正在读取章节正文和来源锚点")?;
        let prepared = self.prepare_chapter_shooting_script_request(input).await?;
        let plan = generate_screenplay_plan(
            self,
            workflow_run_id,
            &prepared,
            progress_total,
            progress,
            cancel_token,
        )
        .await?;
        progress(48, progress_total, "剧本改编校验完成，开始逐单元镜头设计")?;
        let scenes = generate_shooting_scenes(
            self,
            workflow_run_id,
            &prepared,
            &plan,
            progress_total,
            progress,
            cancel_token,
        )
        .await?;
        let script = assemble_chapter_shooting_script(&plan, scenes)?;
        validate_script_source_spans(&script, &prepared.source_spans)?;
        validate_script_against_plan(&script, &plan)?;
        progress(
            94,
            progress_total,
            "确定性装配与合同校验完成，正在发布新版本",
        )?;

        let resolved = self
            .publish_chapter_shooting_script(
                prepared.source,
                prepared.existing,
                prepared.input_fingerprint_sha256,
                script,
                workflow_run_id,
            )
            .await?;
        progress(progress_total, progress_total, "结构化拍摄剧本已原子发布")?;
        Ok(resolved)
    }
}
