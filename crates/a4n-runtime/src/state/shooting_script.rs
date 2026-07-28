//! 拍摄剧本领域 runtime 窄 facade。

use a4n_core::error::AppError;
use a4n_orchestrator::{
    OrchestratorContext,
    shooting_script::{
        ChapterShootingScriptSourceInput, EditChapterShootingScriptInput,
        EditedChapterShootingScript, ResolvedChapterShootingScript, ShootingScriptProgressReporter,
    },
};
use tokio_util::sync::CancellationToken;

pub(crate) struct ShootingScriptRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ShootingScriptRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn current_chapter_shooting_script_for_user_validated(
        &self,
        user_id: i64,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ResolvedChapterShootingScript>, AppError> {
        self.orchestrator
            .current_chapter_shooting_script_for_user_validated(user_id, input)
            .await
    }

    pub(crate) async fn generate_chapter_shooting_script_for_user_validated(
        &self,
        user_id: i64,
        input: ChapterShootingScriptSourceInput,
        progress: ShootingScriptProgressReporter<'_>,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        self.orchestrator
            .generate_chapter_shooting_script_for_user_validated(
                user_id,
                input,
                progress,
                cancel_token,
            )
            .await
    }

    pub(crate) async fn edit_current_chapter_shooting_script_for_user_validated(
        &self,
        user_id: i64,
        source: ChapterShootingScriptSourceInput,
        input: EditChapterShootingScriptInput,
    ) -> Result<EditedChapterShootingScript, AppError> {
        self.orchestrator
            .edit_current_chapter_shooting_script_for_user_validated(user_id, source, input)
            .await
    }
}
