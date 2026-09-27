use crate::video::prompts::generation::frame_prompts::{
    PreparedSegmentPlan, StagedSegmentPlanInput, VideoPromptChunk, VideoPromptProgress,
    render_staged_segment_plan_with_duration, report_video_prompt_progress,
    video_projection_from_shooting_script, video_prompt_generation_trace,
};
use a4n_domain::shooting_script::ChapterShootingScriptV2;
use a4n_error::AppError;
use a4n_video_contract::VideoShootingScriptIdentityV1;

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
        previous_script_identity,
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
    let script_source = crate::shooting_script::ChapterShootingScriptSourceInput {
        project_id,
        source_chapter_id: chapter_id,
        visual_plan_id: None,
    };
    let published = if target_narrative_unit_index.is_some() {
        service
            .current_chapter_shooting_script(script_source.clone())
            .await?
    } else {
        Some(
            service
                .require_current_chapter_shooting_script(script_source.clone())
                .await?,
        )
    };
    let (script, identity, source_shot_order, source_complete) = if let Some(resolved) = published {
        let confirmed = service
            .confirmed_visual_plan_for_shooting_script(&resolved.source, None)
            .await?;
        let director = crate::chapter_visual_plan::project_visual_plan_for_shooting_script(
            &confirmed.plan,
            &[],
        )?;
        let sync = resolved.shooting_script.script.plan_sync(&director);
        let scoped_ids = director
            .narrative_units
            .iter()
            .enumerate()
            .filter(|(offset, _)| {
                target_narrative_unit_index.is_none_or(|index| *offset + 1 == index as usize)
            })
            .flat_map(|(_, unit)| unit.shots.iter().map(|shot| &shot.segment_id))
            .collect::<std::collections::HashSet<_>>();
        let stale = sync
            .stale_shot_ids
            .iter()
            .chain(&sync.missing_shot_ids)
            .filter(|id| scoped_ids.contains(id))
            .collect::<Vec<_>>();
        if !stale.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "请先更新这些镜头的拍摄剧本: {stale:?}"
            )));
        }
        let record = &resolved.shooting_script.record;
        let source_fingerprints = resolved
            .shooting_script
            .script
            .shot_content_fingerprints(&director)?;
        let source_shot_order: Vec<String> = director
            .narrative_units
            .iter()
            .flat_map(|unit| unit.shots.iter().map(|shot| shot.segment_id.clone()))
            .collect();
        (
            scope_shooting_script(resolved.shooting_script.script, target_narrative_unit_index)?,
            VideoShootingScriptIdentityV1 {
                record_id: record.id,
                revision: record.revision,
                source_revision_key: record.source_revision_key.clone(),
                source_content_sha256: record.source_content_sha256.clone(),
                script_sha256: record.script_sha256.clone(),
                shot_fingerprints: source_fingerprints,
            },
            source_shot_order,
            sync.is_current(),
        )
    } else {
        let narrative_unit_index = target_narrative_unit_index
            .expect("whole-chapter planning requires a published script above");
        let unit_progress = service
            .current_chapter_shooting_script_unit_progress(script_source.clone())
            .await?
            .ok_or_else(|| {
                AppError::InvalidArgument("当前章节没有可用的场景级拍摄设计".to_string())
            })?;
        let (resolved_source, _) = service
            .load_chapter_shooting_script_source_record(&script_source)
            .await?;
        let confirmed = service
            .confirmed_visual_plan_for_shooting_script(&resolved_source, None)
            .await?;
        let director = crate::chapter_visual_plan::project_visual_plan_for_shooting_script(
            &confirmed.plan,
            &[],
        )?;
        let source_shot_order: Vec<String> = director
            .narrative_units
            .iter()
            .flat_map(|unit| unit.shots.iter().map(|shot| shot.segment_id.clone()))
            .collect();
        let source_fingerprints = unit_progress.script.shot_content_fingerprints(&director)?;
        let script = scope_shooting_script(unit_progress.script, target_narrative_unit_index)?;
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
                source_content_sha256: a4n_domain::source_text_hash(source.content),
                script_sha256: a4n_domain::source_text_hash(&script_json),
                shot_fingerprints: source_fingerprints,
            },
            source_shot_order,
            false,
        )
    };
    let analysis = video_projection_from_shooting_script(&script, identity)?;
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
            shooting_script_projection: &analysis,
            source_shot_order: &source_shot_order,
            source_complete,
            default_aspect_ratio: aspect_ratio,
            style,
            prompt_output_language,
            previous_segment_plan,
            previous_segment_recipes,
            previous_script_identity,
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

fn scope_shooting_script(
    mut script: ChapterShootingScriptV2,
    target_narrative_unit_index: Option<u32>,
) -> Result<ChapterShootingScriptV2, AppError> {
    if let Some(index) = target_narrative_unit_index {
        script
            .scenes
            .retain(|scene| scene.narrative_unit_index == index);
        if script.scenes.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "拍摄场景 {index} 尚未完成，不能生成片段规划"
            )));
        }
    }
    Ok(script)
}
