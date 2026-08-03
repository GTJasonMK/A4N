use a4n_core::models::generation::workflow_checkpoint;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, QueryFilter, QuerySelect,
    Select,
    sea_query::{
        Expr,
        extension::postgres::{PgExpr, PgFunc},
    },
};
use serde_json::Value;

use crate::workflow::WORKFLOW_STATUS_COMPLETED;

use super::{VideoDbError, required_video_record_text, validate_positive_video_i64};

#[derive(Debug, Clone, FromQueryResult, PartialEq)]
pub struct VideoSegmentPlanDraftCheckpoint {
    pub step_key: String,
    pub extra_json: Option<Value>,
    pub parsed_output_json: Option<Value>,
    pub artifacts_json: Option<Value>,
}

pub async fn video_segment_plan_draft_checkpoints(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    shot_step_key_prefix: &str,
) -> Result<Vec<VideoSegmentPlanDraftCheckpoint>, VideoDbError> {
    let workflow_run_id = validate_positive_video_i64(workflow_run_id, "workflow_run_id")?;
    let shot_step_key_prefix =
        required_video_record_text(shot_step_key_prefix, "shot_step_key_prefix")?;
    video_segment_plan_draft_checkpoint_query()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(workflow_checkpoint::Column::Status.eq(WORKFLOW_STATUS_COMPLETED))
        .filter(
            Expr::expr(PgFunc::starts_with(
                Expr::col(workflow_checkpoint::Column::StepKey),
                shot_step_key_prefix.clone(),
            ))
            .eq(true),
        )
        .filter(workflow_checkpoint::Column::StepKey.ne(shot_step_key_prefix))
        .filter(Expr::col(workflow_checkpoint::Column::StepKey).not_like("%:%"))
        .into_model::<VideoSegmentPlanDraftCheckpoint>()
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}

fn video_segment_plan_draft_checkpoint_query() -> Select<workflow_checkpoint::Entity> {
    workflow_checkpoint::Entity::find()
        .select_only()
        .column(workflow_checkpoint::Column::StepKey)
        .column_as(
            Expr::col(workflow_checkpoint::Column::InputFingerprintJson).get_json_field("extra"),
            "extra_json",
        )
        .column_as(
            Expr::col(workflow_checkpoint::Column::CheckpointJson).get_json_field("parsed_output"),
            "parsed_output_json",
        )
        .column(workflow_checkpoint::Column::ArtifactsJson)
}
