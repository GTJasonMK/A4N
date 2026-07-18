use crate::state::AppState;
use a4n_core::models::chapter::chapter;
use a4n_orchestrator::import_analysis::{
    progress::import_analysis_cancelled, workflow::save_import_stage,
};
use serde_json::Value;

pub(super) enum StageResult {
    Complete(usize),
    Cancelled(usize),
}

pub(super) async fn run(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapters: &[chapter::Model],
    mut analyzed: usize,
    progress: &mut Value,
) -> Result<StageResult, String> {
    let total = chapters.len();
    save_import_stage(
        &state.orchestrator,
        project_id,
        progress,
        super::STAGE_ANALYSIS,
        analyzed,
        total,
        format!("正在分析第{}章", chapters[analyzed].chapter_number),
    )
    .await?;

    for ch in chapters.iter().skip(analyzed) {
        if import_analysis_cancelled(&state.orchestrator, project_id).await? {
            return Ok(StageResult::Cancelled(analyzed));
        }

        save_import_stage(
            &state.orchestrator,
            project_id,
            progress,
            super::STAGE_ANALYSIS,
            analyzed,
            total,
            format!("正在分析第{}章", ch.chapter_number),
        )
        .await?;
        crate::workflow::postprocess_chapter_after_content_change(state, ch.id)
            .await
            .map_err(|err| err.to_string())?;
        crate::workflow::generate_and_sync_chapter_analysis_with_progress(
            state, user_id, ch.id, None,
        )
        .await
        .map_err(|err| format!("第{}章结构化分析失败: {err}", ch.chapter_number))?;
        analyzed += 1;
        save_import_stage(
            &state.orchestrator,
            project_id,
            progress,
            super::STAGE_ANALYSIS,
            analyzed,
            total,
            format!("已完成第{}章分析", ch.chapter_number),
        )
        .await?;
    }

    save_import_stage(
        &state.orchestrator,
        project_id,
        progress,
        super::STAGE_ANALYSIS,
        total,
        total,
        "分析数据生成完成".to_owned(),
    )
    .await?;
    Ok(StageResult::Complete(total))
}
