use crate::video::prompt_generation::frame_prompts::{
    PreparedSegmentPlan, StagedSegmentPlanInput, VideoPromptChunk, VideoPromptProgress,
    render_staged_segment_plan_with_duration, report_video_prompt_progress,
    video_projection_from_shooting_script, video_prompt_generation_trace,
};
use a4n_core::error::AppError;
use a4n_core::video_execution::VideoShootingScriptIdentityV1;
use a4n_domain::shooting_script::ChapterShootingScriptV1;

use super::contracts::{
    SegmentPlanFromShootingScriptProjectionInput, ShootingScriptProjectionPreparationInput,
};

pub(super) async fn render_segment_plan_from_shooting_script_projection(
    input: SegmentPlanFromShootingScriptProjectionInput<'_, '_>,
    progress: &mut Option<VideoPromptProgress<'_>>,
    chunks: &mut Option<VideoPromptChunk<'_>>,
) -> Result<PreparedSegmentPlan, AppError> {
    let SegmentPlanFromShootingScriptProjectionInput {
        shooting_script_projection,
        segment_context,
        segment_progress,
        segment_span,
    } = input;
    let ShootingScriptProjectionPreparationInput {
        service,
        project_id,
        chapter_id,
        source,
        source_metadata,
        source_spans,
        project_context,
        style,
        prompt_output_language,
        previous_segment_plan,
        previous_segment_recipes,
        reuse_published_shots,
        duration_secs,
        aspect_ratio,
        prompt_renderer,
        variables,
        workflow_run_id,
        target_narrative_unit_index,
        progress_span: shooting_script_projection_span,
        cancel_token,
        ..
    } = shooting_script_projection;
    let (script, identity, source_shot_order, source_complete) =
        if let Some(narrative_unit_index) = target_narrative_unit_index {
            let unit_progress = service
                .current_chapter_shooting_script_unit_progress(
                    crate::shooting_script::ChapterShootingScriptSourceInput {
                        project_id,
                        source_chapter_id: chapter_id,
                    },
                )
                .await?
                .ok_or_else(|| {
                    AppError::InvalidArgument("当前章节没有可用的场景级拍摄设计".to_string())
                })?;
            let source_shot_order = shooting_script_shot_ids(&unit_progress.script);
            let source_complete = unit_progress.resolved.is_some();
            let scene = unit_progress
                .script
                .scenes
                .iter()
                .find(|scene| scene.narrative_unit_index == narrative_unit_index)
                .cloned()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "拍摄场景 {narrative_unit_index} 尚未完成，不能生成片段规划"
                    ))
                })?;
            let script = ChapterShootingScriptV1 {
                schema_version: unit_progress.script.schema_version,
                directing_statement: unit_progress.script.directing_statement,
                scenes: vec![scene],
            };
            let script_json = serde_json::to_string(&script)?;
            let revision = i32::try_from(unit_progress.completed_unit_count).map_err(|_| {
                AppError::InvalidArgument("已完成拍摄场景数量超出 i32 范围".to_string())
            })?;
            (
                script,
                VideoShootingScriptIdentityV1 {
                    record_id: unit_progress.workflow_run_id,
                    revision,
                    source_revision_key: format!(
                        "workflow:{}:unit:{}",
                        unit_progress.workflow_run_id, narrative_unit_index
                    ),
                    source_content_sha256: a4n_core::source_spans::source_text_hash(source.content),
                    script_sha256: a4n_core::source_spans::source_text_hash(&script_json),
                },
                source_shot_order,
                source_complete,
            )
        } else {
            let resolved = service
                .require_current_chapter_shooting_script(
                    crate::shooting_script::ChapterShootingScriptSourceInput {
                        project_id,
                        source_chapter_id: chapter_id,
                    },
                )
                .await?;
            let record = &resolved.shooting_script.record;
            let source_shot_order = shooting_script_shot_ids(&resolved.shooting_script.script);
            (
                resolved.shooting_script.script,
                VideoShootingScriptIdentityV1 {
                    record_id: record.id,
                    revision: record.revision,
                    source_revision_key: record.source_revision_key.clone(),
                    source_content_sha256: record.source_content_sha256.clone(),
                    script_sha256: record.script_sha256.clone(),
                },
                source_shot_order,
                true,
            )
        };
    let analysis = video_projection_from_shooting_script(&script, identity, duration_secs)?;
    let analysis_json = serde_json::to_value(&analysis).map_err(|error| {
        AppError::InvalidArgument(format!("拍摄剧本视频执行投影序列化失败: {error}"))
    })?;
    report_video_prompt_progress(
        progress,
        shooting_script_projection_span.end(),
        100,
        "已从结构化拍摄剧本构建视频执行投影",
    )?;
    let (segment_trace, segment_plan, segment_stage_outputs) =
        render_staged_segment_plan_with_duration(StagedSegmentPlanInput {
            service,
            prompt_renderer,
            project_id,
            chapter_id,
            workflow_run_id,
            variables,
            source,
            source_metadata,
            source_spans,
            project_context,
            shooting_script_projection_json: &analysis_json,
            shooting_script_projection: &analysis,
            source_shot_order: &source_shot_order,
            source_complete,
            default_aspect_ratio: aspect_ratio,
            style,
            prompt_output_language,
            previous_segment_plan,
            previous_segment_recipes,
            reuse_published_shots,
            duration_secs,
            context: segment_context,
            progress_message: segment_progress,
            progress_span: segment_span,
            cancel_token,
            progress,
            chunks,
        })
        .await?;
    if cancel_token.is_some_and(tokio_util::sync::CancellationToken::is_cancelled) {
        return Err(AppError::Cancelled);
    }
    report_video_prompt_progress(
        progress,
        segment_span.end().saturating_add(5).min(99),
        100,
        "片段规划校验完成，正在提交正式记录",
    )?;
    Ok(PreparedSegmentPlan {
        segment_stage_outputs,
        segment_trace: segment_trace
            .map(|trace| video_prompt_generation_trace(trace, workflow_run_id)),
        shooting_script_projection: analysis,
        segment_plan,
        source_shot_order,
        source_complete,
    })
}

fn shooting_script_shot_ids(script: &ChapterShootingScriptV1) -> Vec<String> {
    script
        .scenes
        .iter()
        .flat_map(|scene| scene.shots.iter().map(|shot| shot.shot_id.clone()))
        .collect()
}
