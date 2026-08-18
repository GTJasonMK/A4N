use super::expected_records::ExpectedRagRecords;

pub enum SceneRagSyncTraceSource {
    WorkflowRun(i64),
    Independent,
    CurrentOperation,
}

pub struct RagIngestTaskScope {
    pub project_id: i64,
    pub total: u32,
    pub(crate) expected: ExpectedRagRecords,
}
