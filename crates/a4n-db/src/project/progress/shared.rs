use super::{ProjectProgressStatus, ProjectProgressSummary};

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct ProgressCounts {
    pub(super) pending: i64,
    pub(super) running: i64,
    pub(super) completed: i64,
    pub(super) failed: i64,
    pub(super) cancelled: i64,
}

impl ProgressCounts {
    pub(super) fn add(&mut self, status: &str) {
        match status.trim() {
            "pending" => self.pending += 1,
            "running" | "generating" => self.running += 1,
            "completed" => self.completed += 1,
            "failed" => self.failed += 1,
            "cancelled" => self.cancelled += 1,
            _ => {}
        }
    }

    pub(super) fn total(self) -> i64 {
        self.pending + self.running + self.completed + self.failed + self.cancelled
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct PipelineProjectState {
    pub(super) running: bool,
    pub(super) failed: bool,
    pub(super) cancelled: bool,
    pub(super) message: Option<String>,
}

pub(super) fn project_progress(
    kind: &str,
    stage: &str,
    label: &str,
    current: Option<i64>,
    total: Option<i64>,
    status: ProjectProgressStatus,
    detail: Option<String>,
) -> ProjectProgressSummary {
    ProjectProgressSummary {
        kind: kind.to_string(),
        stage: stage.to_string(),
        label: label.to_string(),
        current,
        total,
        status,
        detail,
    }
}

pub(super) fn progress_count_detail(
    label: &str,
    current: Option<i64>,
    total: Option<i64>,
) -> Option<String> {
    match (current, total) {
        (Some(current), Some(total)) if total > 0 => Some(format!("{label} {current}/{total}")),
        _ => None,
    }
}

pub(super) fn non_zero(value: i64) -> Option<i64> {
    (value > 0).then_some(value)
}
