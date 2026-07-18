use crate::state::AppState;
use a4n_core::models::{chapter::chapter, project::novel_project};
use a4n_orchestrator::import_analysis::{
    blueprint::upsert_import_blueprint, progress::progress_stage_completed,
    workflow::save_import_stage,
};
use serde_json::Value;

pub(super) async fn run(
    state: &AppState,
    project: &novel_project::Model,
    project_id: i64,
    chapters: &[chapter::Model],
    needs_part_outlines: bool,
    progress: &mut Value,
) -> Result<(), String> {
    let completed = progress_stage_completed(progress, super::STAGE_BLUEPRINT)? >= 1;
    if completed {
        return Ok(());
    }

    save_import_stage(
        &state.orchestrator,
        project_id,
        progress,
        super::STAGE_BLUEPRINT,
        0,
        1,
        "正在生成导入蓝图".to_owned(),
    )
    .await?;
    upsert_import_blueprint(&state.orchestrator, project, chapters, needs_part_outlines).await?;
    super::super::super::blueprint::apply_reverse_blueprint(state, project_id).await?;
    save_import_stage(
        &state.orchestrator,
        project_id,
        progress,
        super::STAGE_BLUEPRINT,
        1,
        1,
        "导入蓝图已生成".to_owned(),
    )
    .await
}
