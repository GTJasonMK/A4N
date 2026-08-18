mod call;
mod contracts;
mod preparation;
mod rules;
mod transport;

use a4n_core::error::AppError;

use crate::blueprint::OrchestratorContext;

use self::call::execute_blueprint_stage_call;
use self::preparation::prepare_blueprint_stage;

pub(super) use contracts::{BlueprintStageExecution, BlueprintStageRunInput};
pub(super) use rules::ensure_not_cancelled;

impl OrchestratorContext {
    pub(super) async fn generate_or_resume_blueprint_stage(
        &self,
        input: BlueprintStageRunInput<'_>,
    ) -> Result<BlueprintStageExecution, AppError> {
        let prepared = prepare_blueprint_stage(self, input).await?;
        execute_blueprint_stage_call(self, prepared).await
    }
}
