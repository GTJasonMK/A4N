//! Typed command payloads owned by the novel-planning worker.

pub use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanReadResult, ChapterVisualPlanRecordInput, ChapterVisualPlanSetView,
    ChapterVisualPlanSourceInput, ChapterVisualPlanValidationResult, ChapterVisualPlanView,
    ConfirmedChapterVisualPlanView, ReadChapterVisualPlanInput, SaveChapterVisualPlanDraftInput,
    ValidateChapterVisualPlanInput,
};
pub use a4n_api_contract::shooting_script::{
    ChapterShootingScriptSourceInput, ChapterShootingScriptView, DeleteChapterShootingScriptInput,
    EditChapterShootingScriptInput, EditedChapterShootingScriptView,
    PublishChapterShootingScriptInput,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

pub const NOVEL_PLANNING_WORKER_ID: &str = "novel-planning-worker";
pub const NOVEL_PLANNING_WORKER_CONTRACT_VERSION: &str = "novel-planning-shooting-script-v1";

pub const GET_CURRENT_SHOOTING_SCRIPT_OPERATION: &str = "novel_planning.shooting_script.current";
pub const EDIT_SHOOTING_SCRIPT_OPERATION: &str = "novel_planning.shooting_script.edit";
pub const DELETE_SHOOTING_SCRIPT_OPERATION: &str = "novel_planning.shooting_script.delete";
pub const PUBLISH_SHOOTING_SCRIPT_OPERATION: &str = "novel_planning.shooting_script.publish";

pub const SHOOTING_SCRIPT_CURRENT_CAPABILITY: &str = "shooting_script.current";
pub const SHOOTING_SCRIPT_EDIT_CAPABILITY: &str = "shooting_script.edit";
pub const SHOOTING_SCRIPT_DELETE_CAPABILITY: &str = "shooting_script.delete";
pub const SHOOTING_SCRIPT_PUBLISH_CAPABILITY: &str = "shooting_script.publish";
pub const SAVE_VISUAL_PLAN_DRAFT_OPERATION: &str = "novel_planning.visual_plan.save_draft";
pub const CONFIRM_VISUAL_PLAN_OPERATION: &str = "novel_planning.visual_plan.confirm";
pub const DISCARD_VISUAL_PLAN_OPERATION: &str = "novel_planning.visual_plan.discard";
pub const VISUAL_PLAN_PERSISTENCE_CAPABILITY: &str = "visual_plan.persistence";
pub const GET_VISUAL_PLAN_OPERATION: &str = "novel_planning.visual_plan.get";
pub const READ_VISUAL_PLAN_OPERATION: &str = "novel_planning.visual_plan.read";
pub const VALIDATE_VISUAL_PLAN_OPERATION: &str = "novel_planning.visual_plan.validate";

pub type CurrentShootingScriptPayload = ChapterShootingScriptSourceInput;
pub type EditShootingScriptPayload = EditChapterShootingScriptInput;
pub type DeleteShootingScriptPayload = DeleteChapterShootingScriptInput;
pub type PublishShootingScriptPayload = PublishChapterShootingScriptInput;
pub type SaveVisualPlanDraftPayload = SaveChapterVisualPlanDraftInput;
pub type ConfirmVisualPlanPayload = ChapterVisualPlanRecordInput;
pub type DiscardVisualPlanPayload = ChapterVisualPlanRecordInput;
pub type GetVisualPlanPayload = ChapterVisualPlanSourceInput;
pub type ReadVisualPlanPayload = ReadChapterVisualPlanInput;
pub type ValidateVisualPlanPayload = ValidateChapterVisualPlanInput;

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn command_identity_is_stable_and_payload_uses_public_contract() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request-1".into(),
            trace_id: "trace-1".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: GET_CURRENT_SHOOTING_SCRIPT_OPERATION.into(),
            idempotency_key: "idempotency-1".into(),
            project_id: Some(1),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: NOVEL_PLANNING_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ChapterShootingScriptSourceInput {
                project_id: 1,
                source_chapter_id: 2,
                visual_plan_id: None,
            },
        };
        request
            .validate()
            .expect("payload envelope should validate");
        assert_eq!(NOVEL_PLANNING_WORKER_ID, "novel-planning-worker");
    }
}
