use a4n_core::error::AppError;
use a4n_core::models::generation::{generation_spec, prompt_artifact};

use super::{
    CanvasNodeArtifactUpdateInput, CanvasNodeObservationUpdateInput,
    CanvasPromptArtifactUpsertInput, FreeformCanvasGenerationSpecUpsertInput,
};
use crate::OrchestratorContext;
use crate::canvas::errors::canvas_db_error;

impl OrchestratorContext {
    pub async fn upsert_freeform_canvas_generation_spec_record(
        &self,
        input: FreeformCanvasGenerationSpecUpsertInput,
    ) -> Result<generation_spec::Model, AppError> {
        a4n_db::canvas::upsert_freeform_canvas_generation_spec_record(self.db.connection(), input)
            .await
            .map_err(canvas_db_error)
    }

    pub async fn mark_canvas_generation_spec_status_record(
        &self,
        generation_spec_id: i64,
        status: &str,
    ) -> Result<(), AppError> {
        a4n_db::canvas::mark_canvas_generation_spec_status_record(
            self.db.connection(),
            generation_spec_id,
            status,
        )
        .await
        .map_err(canvas_db_error)
    }

    pub async fn upsert_canvas_prompt_artifact_record(
        &self,
        input: CanvasPromptArtifactUpsertInput,
    ) -> Result<prompt_artifact::Model, AppError> {
        a4n_db::canvas::upsert_canvas_prompt_artifact_record(self.db.connection(), input)
            .await
            .map_err(canvas_db_error)
    }

    pub async fn update_canvas_node_artifact_record(
        &self,
        input: CanvasNodeArtifactUpdateInput,
    ) -> Result<(), AppError> {
        a4n_db::canvas::update_canvas_node_artifact_record(self.db.connection(), input)
            .await
            .map_err(canvas_db_error)
    }

    pub async fn update_canvas_node_observation_record(
        &self,
        input: CanvasNodeObservationUpdateInput,
    ) -> Result<(), AppError> {
        a4n_db::canvas::update_canvas_node_observation_record(self.db.connection(), input)
            .await
            .map_err(canvas_db_error)
    }
}
