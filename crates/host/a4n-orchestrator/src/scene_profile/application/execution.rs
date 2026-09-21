mod chapters;
mod persistence;

use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;

use crate::OrchestratorContext;

use super::{
    SceneProfileSyncHostPort,
    context::select_scene_profile_sync_chapters,
    progress::{SceneProfileSyncProgress, report_scene_profile_sync_progress},
};

impl OrchestratorContext {
    pub async fn sync_scene_profiles_validated(
        &self,
        project_id: i64,
        chapter_number: Option<i32>,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn SceneProfileSyncHostPort,
        progress: Option<&SceneProfileSyncProgress<'_>>,
    ) -> Result<(), AppError> {
        report_scene_profile_sync_progress(progress, 0, "正在读取可同步章节")?;
        let chapters = select_scene_profile_sync_chapters(self, project_id, chapter_number).await?;
        let workflow_run = self
            .begin_scene_profile_sync_workflow_validated(
                project_id,
                chapter_number,
                &chapters,
                progress.and_then(SceneProfileSyncProgress::active_task_id),
            )
            .await?;
        let result = chapters::sync_scene_profile_chapters(
            self,
            prompt_renderer,
            host_port,
            project_id,
            workflow_run.id,
            chapters,
            progress,
        )
        .await;
        self.finalize_workflow_result(workflow_run.id, result, "场景档案同步")
            .await
    }
}
