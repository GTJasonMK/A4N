pub const MEDIA_WORKFLOW_CONTRACT_V2_RESET_KIND: &str = "media_workflow_contract_v2_reset";

#[derive(Debug, Clone, Default)]
pub struct MediaWorkflowV2ResetOutcome {
    pub media_paths: Vec<String>,
}
