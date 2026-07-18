use serde_json::Value;

#[derive(Clone, Copy)]
pub(in crate::manga) struct MangaPipelineStatusInput<'a> {
    pub(in crate::manga) chapter_id: i64,
    pub(in crate::manga) project_id: i64,
    pub(in crate::manga) pipeline_status: &'a str,
    pub(in crate::manga) stage: &'a str,
    pub(in crate::manga) current: i32,
    pub(in crate::manga) total: i32,
    pub(in crate::manga) message: &'a str,
    pub(in crate::manga) task_id: Option<&'a str>,
    pub(in crate::manga) allow_cancel_override: bool,
}

#[derive(Clone, Copy)]
pub(in crate::manga) struct MangaPipelineCheckpointInput<'a> {
    pub(in crate::manga) status: MangaPipelineStatusInput<'a>,
    pub(in crate::manga) source_metadata: &'a Value,
    pub(in crate::manga) reference_assets_metadata: &'a Value,
    pub(in crate::manga) prompt_fingerprint: &'a Value,
    pub(in crate::manga) checkpoint_data: &'a Value,
}
