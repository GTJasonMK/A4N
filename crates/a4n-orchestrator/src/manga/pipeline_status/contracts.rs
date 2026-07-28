use serde_json::Value;

#[derive(Clone, Copy)]
pub(crate) struct MangaPipelineStatusInput<'a> {
    pub(crate) chapter_id: i64,
    pub(crate) project_id: i64,
    pub(crate) pipeline_status: &'a str,
    pub(crate) stage: &'a str,
    pub(crate) current: i32,
    pub(crate) total: i32,
    pub(crate) message: &'a str,
    pub(crate) task_id: Option<&'a str>,
    pub(crate) allow_cancel_override: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct MangaPipelineCheckpointInput<'a> {
    pub(crate) status: MangaPipelineStatusInput<'a>,
    pub(crate) source_metadata: &'a Value,
    pub(crate) reference_assets_metadata: &'a Value,
    pub(crate) prompt_fingerprint: &'a Value,
    pub(crate) checkpoint_data: &'a Value,
}
