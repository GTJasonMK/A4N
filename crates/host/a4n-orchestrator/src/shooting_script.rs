mod assembly;
mod context;
mod contracts;
pub(crate) mod errors;
mod generation;
mod preparation;
mod progress;
mod stage;
mod task;
mod workflow_rules;

pub(crate) use a4n_api_contract::shooting_script::{
    ChapterShootingScriptSourceInput, ShootingScriptGenerationMode,
    StartChapterShootingScriptGenerationInput,
};
pub(crate) use contracts::ResolvedChapterShootingScript;
#[cfg(test)]
pub(crate) use contracts::{seed_visual_plan_scene, visual_plan_fixture};
pub(in crate::shooting_script) use workflow_rules::chapter_shooting_script_workflow_run_input;

use a4n_error::AppError;
use a4n_novel_host_contract::ShootingScriptGenerationApplicationPort;

pub(crate) fn validate_shooting_generation_mode(
    mode: ShootingScriptGenerationMode,
    shot_ids: &[String],
) -> Result<(), AppError> {
    if mode == ShootingScriptGenerationMode::NextScene && !shot_ids.is_empty() {
        return Err(AppError::InvalidArgument(
            "局部镜头更新请使用 full 模式；逐场景续写不接受 shot_ids".into(),
        ));
    }
    Ok(())
}

#[async_trait::async_trait]
impl ShootingScriptGenerationApplicationPort for crate::OrchestratorContext {
    async fn start(
        &self,
        user_id: i64,
        input: &a4n_api_contract::shooting_script::StartChapterShootingScriptGenerationInput,
    ) -> Result<
        a4n_api_contract::generation_tasks::GenerationTaskStart,
        a4n_api_contract::generation_tasks::GenerationTaskStartError,
    > {
        self.start_shooting_script_generation_task(user_id, input)
            .await
            .map_err(crate::GenerationTaskAccessError::into_start_error)
    }

    async fn run(
        &self,
        user_id: i64,
        input: a4n_api_contract::shooting_script::StartChapterShootingScriptGenerationInput,
        handle: a4n_api_contract::generation_tasks::GenerationTaskHandle,
    ) -> Result<(), String> {
        self.run_shooting_script_generation_task(user_id, input, handle)
            .await
    }
}
