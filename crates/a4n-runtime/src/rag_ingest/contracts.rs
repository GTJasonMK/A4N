use super::expected_records::ExpectedRagRecords;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneRagSyncTraceSource {
    WorkflowRun(i64),
    Independent,
    CurrentOperation,
}

#[derive(Debug, Clone)]
pub struct RagIngestTaskScope {
    pub project_id: i64,
    pub total: u32,
    pub(crate) expected: ExpectedRagRecords,
}
