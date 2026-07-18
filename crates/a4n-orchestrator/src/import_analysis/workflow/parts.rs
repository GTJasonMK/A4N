use a4n_core::models::chapter::chapter;
use serde_json::Value;

use super::save_import_stage;
use crate::{
    OrchestratorContext,
    import_analysis::{
        part_outline::upsert_import_part_outlines,
        progress::{CHAPTERS_PER_PART, STAGE_PARTS, progress_stage_completed},
    },
};

pub async fn run_import_analysis_parts(
    context: &OrchestratorContext,
    project_id: i64,
    chapters: &[chapter::Model],
    needs_part_outlines: bool,
    progress: &mut Value,
) -> Result<usize, String> {
    if !needs_part_outlines {
        return Ok(0);
    }

    let expected_parts = chapters.len().div_ceil(CHAPTERS_PER_PART);
    let progress_parts = progress_stage_completed(progress, STAGE_PARTS)?;
    if progress_parts >= expected_parts {
        return Ok(progress_parts);
    }

    save_import_stage(
        context,
        project_id,
        progress,
        STAGE_PARTS,
        progress_parts,
        expected_parts,
        "正在生成分部大纲".to_owned(),
    )
    .await?;
    let generated = upsert_import_part_outlines(context, project_id, chapters).await?;
    save_import_stage(
        context,
        project_id,
        progress,
        STAGE_PARTS,
        generated,
        expected_parts,
        "分部大纲已生成".to_owned(),
    )
    .await?;
    Ok(generated)
}
