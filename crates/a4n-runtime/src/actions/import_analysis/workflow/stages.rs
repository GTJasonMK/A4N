mod analysis;
mod blueprint;

use super::super::ImportProjectAnalysisSummary;
use crate::state::AppState;
use a4n_core::models::{chapter::chapter, project::novel_project};
use a4n_orchestrator::import_analysis::chapter_outline::importable_chapters;
use a4n_orchestrator::import_analysis::lifecycle::cancelled_import_analysis_summary;
use a4n_orchestrator::import_analysis::progress::{
    STAGE_ANALYSIS, STAGE_BLUEPRINT, STAGE_OUTLINES, import_analysis_cancelled,
};
use a4n_orchestrator::import_analysis::summary::completed_import_stage;
use a4n_orchestrator::import_analysis::workflow::{
    ImportAnalysisStageResult, complete_import_project_analysis, run_import_analysis_outlines,
    run_import_analysis_parts,
};
use serde_json::Value;

pub(in crate::actions::import_analysis) async fn run_import_project_analysis_inner(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    project: &novel_project::Model,
    mut chapters: Vec<chapter::Model>,
    needs_part_outlines: bool,
    progress: &mut Value,
) -> Result<ImportProjectAnalysisSummary, String> {
    let total_chapters = chapters.len();
    let mut analyzed = completed_import_stage(progress, STAGE_ANALYSIS, total_chapters)?;

    if analyzed < total_chapters {
        match analysis::run(
            state,
            user_id,
            project_id,
            chapters.as_slice(),
            analyzed,
            progress,
        )
        .await?
        {
            analysis::StageResult::Complete(value) => analyzed = value,
            analysis::StageResult::Cancelled(value) => {
                let outlined = completed_import_stage(progress, STAGE_OUTLINES, total_chapters)?;
                return cancelled_import_analysis_summary(
                    &state.orchestrator,
                    project_id,
                    progress,
                    value,
                    outlined,
                    needs_part_outlines,
                    chapters.len(),
                )
                .await;
            }
        }
    } else {
        analyzed = total_chapters;
    }

    chapters = importable_chapters(&state.orchestrator, project_id).await?;
    let mut outlined = completed_import_stage(progress, STAGE_OUTLINES, chapters.len())?;

    if outlined < chapters.len() {
        match run_import_analysis_outlines(
            &state.orchestrator,
            project_id,
            chapters.as_slice(),
            outlined,
            progress,
        )
        .await?
        {
            ImportAnalysisStageResult::Complete(value) => outlined = value,
            ImportAnalysisStageResult::Cancelled(value) => {
                return cancelled_import_analysis_summary(
                    &state.orchestrator,
                    project_id,
                    progress,
                    analyzed,
                    value,
                    needs_part_outlines,
                    chapters.len(),
                )
                .await;
            }
        }
    } else {
        outlined = chapters.len();
    }

    blueprint::run(
        state,
        project,
        project_id,
        &chapters,
        needs_part_outlines,
        progress,
    )
    .await?;

    if import_analysis_cancelled(&state.orchestrator, project_id).await? {
        return cancelled_import_analysis_summary(
            &state.orchestrator,
            project_id,
            progress,
            analyzed,
            outlined,
            needs_part_outlines,
            chapters.len(),
        )
        .await;
    }

    let generated_parts = run_import_analysis_parts(
        &state.orchestrator,
        project_id,
        &chapters,
        needs_part_outlines,
        progress,
    )
    .await?;

    complete_import_project_analysis(
        &state.orchestrator,
        project_id,
        progress,
        analyzed,
        outlined,
        generated_parts,
    )
    .await
}
