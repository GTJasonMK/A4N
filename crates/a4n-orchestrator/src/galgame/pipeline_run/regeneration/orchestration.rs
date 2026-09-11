use a4n_error::AppError;

use crate::galgame::{
    GalgameSceneRegenerationInput, GalgameSceneRegenerationPlan, OrchestratorContext,
};
use a4n_prompt::rendering::PromptRenderer;

use super::finalization::finalize_galgame_scene_regeneration;
use super::preparation::prepare_galgame_scene_regeneration;
use super::stages::run_galgame_scene_regeneration_stages;

impl OrchestratorContext {
    pub async fn build_source_sync_scene_regeneration(
        &self,
        input: GalgameSceneRegenerationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<GalgameSceneRegenerationPlan, AppError> {
        let prepared = prepare_galgame_scene_regeneration(&input)?;
        let outputs =
            run_galgame_scene_regeneration_stages(self, &input, &prepared, prompt_renderer).await?;
        finalize_galgame_scene_regeneration(&input, &prepared, outputs)
    }
}
