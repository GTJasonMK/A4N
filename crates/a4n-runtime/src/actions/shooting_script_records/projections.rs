use a4n_api_contract::shooting_script::{
    ChapterShootingScriptRecordSummary, ChapterShootingScriptUnitProgressView,
    ChapterShootingScriptView,
};
use a4n_core::models::chapter::chapter_shooting_script;
use a4n_domain::shooting_script::ChapterShootingScriptV1;

pub(super) fn contract_view(
    record: chapter_shooting_script::Model,
    script: ChapterShootingScriptV1,
) -> ChapterShootingScriptView {
    ChapterShootingScriptView {
        record: record_summary(record),
        script,
    }
}

pub(super) fn progress_contract_view(
    progress: a4n_orchestrator::shooting_script::ChapterShootingScriptUnitProgress,
) -> ChapterShootingScriptUnitProgressView {
    let completed_record = progress
        .resolved
        .map(|resolved| record_summary(resolved.shooting_script.record));
    ChapterShootingScriptUnitProgressView {
        workflow_run_id: progress.workflow_run_id,
        script: progress.script,
        completed_unit_count: progress.completed_unit_count,
        total_unit_count: progress.total_unit_count,
        completed_record,
    }
}

fn record_summary(record: chapter_shooting_script::Model) -> ChapterShootingScriptRecordSummary {
    ChapterShootingScriptRecordSummary {
        id: record.id,
        revision: record.revision,
    }
}
