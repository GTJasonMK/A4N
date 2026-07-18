#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PipelineProgress {
    pub(crate) current: i32,
    pub(crate) total: i32,
}
