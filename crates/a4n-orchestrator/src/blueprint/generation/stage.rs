mod call;
mod contracts;
mod preparation;
mod repair;
mod rules;
mod transport;

use a4n_core::error::AppError;
use serde_json::Value;

use crate::blueprint::OrchestratorContext;

use self::call::execute_blueprint_stage_call;
use self::preparation::prepare_blueprint_stage;

pub(super) use contracts::BlueprintStageRunInput;
pub(super) use rules::{ensure_not_cancelled, report};

impl OrchestratorContext {
    pub(super) async fn generate_or_resume_blueprint_stage(
        &self,
        input: BlueprintStageRunInput<'_>,
    ) -> Result<Value, AppError> {
        let prepared = prepare_blueprint_stage(self, input).await?;
        execute_blueprint_stage_call(self, prepared).await
    }
}
