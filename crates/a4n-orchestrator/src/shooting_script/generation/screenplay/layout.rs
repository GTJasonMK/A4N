use a4n_core::{config::app_config::llm_token_profiles, error::AppError};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    shooting_script::{
        context::shooting_context_request,
        contracts::{
            CHAPTER_SCREENPLAY_LAYOUT_UNIT_PROMPT, CHAPTER_SCREENPLAY_SKELETON_PROMPT,
            ChapterScreenplayLayoutV1, ChapterScreenplaySkeletonV1,
            PreparedChapterShootingScriptRequest, ScreenplayLayoutUnitDetailV1,
            ShootingScriptProgressReporter,
        },
        progress::{SKELETON_STREAM_SPAN, layout_unit_stream_progress, skeleton_stream_progress},
        screenplay::{assemble_screenplay_layout, validate_screenplay_layout},
    },
};

use super::validation::{validate_layout_unit_detail, validate_skeleton};

pub(super) async fn generate_screenplay_layout(
    orchestrator: &OrchestratorContext,
    workflow_run_id: i64,
    prepared: &PreparedChapterShootingScriptRequest,
    progress_total: u32,
    progress: ShootingScriptProgressReporter<'_>,
    cancel_token: Option<&CancellationToken>,
) -> Result<ChapterScreenplayLayoutV1, AppError> {
    progress(1, progress_total, "正在生成或恢复全章叙事骨架")?;
    let skeleton_request = shooting_context_request::<serde_json::Value>(
        prepared,
        workflow_run_id,
        CHAPTER_SCREENPLAY_SKELETON_PROMPT,
        "screenplay_skeleton".into(),
        prepared.skeleton_system_prompt.clone(),
        None,
        "基于全部稳定上下文生成全章叙事骨架。".into(),
        llm_token_profiles::LONG.into(),
    )?;
    let skeleton_stage = orchestrator
        .run_shooting_script_stage::<ChapterScreenplaySkeletonV1>(
            workflow_run_id,
            "screenplay_skeleton",
            1,
            skeleton_request,
            "章节叙事骨架",
            skeleton_stream_progress(),
            progress_total,
            progress,
            cancel_token,
            |output| validate_skeleton(output, prepared),
        )
        .await?;
    progress(
        SKELETON_STREAM_SPAN.end(),
        progress_total,
        if skeleton_stage.restored {
            "叙事骨架已恢复，开始逐单元细化"
        } else {
            "叙事骨架已生成，开始逐单元细化"
        },
    )?;
    let skeleton = skeleton_stage.output;

    let mut details = Vec::with_capacity(skeleton.units.len());
    for (offset, unit) in skeleton.units.iter().enumerate() {
        let unit_progress = layout_unit_stream_progress(offset, skeleton.units.len());
        let request = shooting_context_request(
            prepared,
            workflow_run_id,
            CHAPTER_SCREENPLAY_LAYOUT_UNIT_PROMPT,
            format!("screenplay_layout_unit:{}", unit.narrative_unit_index),
            prepared.layout_unit_system_prompt.clone(),
            Some(("shooting_layout_skeleton", &skeleton)),
            format!(
                "只细化 narrative_unit_index={}。",
                unit.narrative_unit_index
            ),
            llm_token_profiles::LONG.into(),
        )?;
        let stage = orchestrator
            .run_shooting_script_stage::<ScreenplayLayoutUnitDetailV1>(
                workflow_run_id,
                &format!("screenplay_layout_unit:{}", unit.narrative_unit_index),
                i32::try_from(offset + 2)
                    .map_err(|_| AppError::InvalidArgument("布局单元数量超限".into()))?,
                request,
                "叙事单元布局细化",
                unit_progress,
                progress_total,
                progress,
                cancel_token,
                |output| validate_layout_unit_detail(output, unit.narrative_unit_index),
            )
            .await?;
        progress(
            unit_progress.span.end(),
            progress_total,
            &format!(
                "布局单元 {}/{} {}",
                offset + 1,
                skeleton.units.len(),
                if stage.restored {
                    "已从调用断点恢复"
                } else {
                    "细化完成"
                }
            ),
        )?;
        details.push(stage.output);
    }

    let layout = assemble_screenplay_layout(skeleton, details)?;
    validate_screenplay_layout(&layout, &prepared.source_spans)?;
    Ok(layout)
}
