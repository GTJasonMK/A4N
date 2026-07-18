use a4n_core::models::chapter::chapter;
use serde_json::Value;

use super::{ImportAnalysisStageResult, save_import_stage};
use crate::{
    OrchestratorContext,
    import_analysis::{
        chapter_outline::upsert_outline_from_chapter, progress::import_analysis_cancelled,
    },
};

pub async fn run_import_analysis_outlines(
    context: &OrchestratorContext,
    project_id: i64,
    chapters: &[chapter::Model],
    mut outlined: usize,
    progress: &mut Value,
) -> Result<ImportAnalysisStageResult, String> {
    let total = chapters.len();
    save_import_stage(
        context,
        project_id,
        progress,
        crate::import_analysis::progress::STAGE_OUTLINES,
        outlined,
        total,
        format!("正在更新第{}章大纲", chapters[outlined].chapter_number),
    )
    .await?;

    for ch in chapters.iter().skip(outlined) {
        if import_analysis_cancelled(context, project_id).await? {
            return Ok(ImportAnalysisStageResult::Cancelled(outlined));
        }

        save_import_stage(
            context,
            project_id,
            progress,
            crate::import_analysis::progress::STAGE_OUTLINES,
            outlined,
            total,
            format!("正在更新第{}章大纲", ch.chapter_number),
        )
        .await?;
        upsert_outline_from_chapter(context, ch).await?;
        outlined += 1;
        save_import_stage(
            context,
            project_id,
            progress,
            crate::import_analysis::progress::STAGE_OUTLINES,
            outlined,
            total,
            format!("已更新第{}章大纲", ch.chapter_number),
        )
        .await?;
    }

    save_import_stage(
        context,
        project_id,
        progress,
        crate::import_analysis::progress::STAGE_OUTLINES,
        total,
        total,
        "章节大纲已更新".to_owned(),
    )
    .await?;
    Ok(ImportAnalysisStageResult::Complete(total))
}
