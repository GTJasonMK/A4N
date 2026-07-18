mod call;
mod contracts;
mod persistence;
mod preparation;
mod schema;

use std::collections::HashMap;

use a4n_core::{error::AppError, models::project::novel_blueprint};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use crate::OrchestratorContext;

use self::call::execute_blueprint_refine_call;
use self::contracts::BlueprintRefinementPersistenceInput;
use self::persistence::persist_blueprint_refinement;
use self::preparation::prepare_blueprint_refine;
use super::finalization::finalize_refine_workflow;

impl OrchestratorContext {
    pub async fn refine_blueprint(
        &self,
        project_id: i64,
        focus: Option<&str>,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<novel_blueprint::Model, AppError> {
        let prepared =
            prepare_blueprint_refine(self, project_id, focus, prompt_renderer, variables).await?;
        let workflow_run_id = prepared.workflow_run_id;

        let result = async {
            let parsed = execute_blueprint_refine_call(self, &prepared).await?;
            persist_blueprint_refinement(
                self,
                BlueprintRefinementPersistenceInput {
                    project_id,
                    blueprint: prepared.blueprint,
                    next_version: prepared.next_version,
                    parsed,
                },
            )
            .await
        }
        .await;

        finalize_refine_workflow(self, workflow_run_id, &result).await?;
        result
    }
}
