use std::collections::{BTreeSet, HashMap};

use a4n_domain::source_spans::{SourceSpan, source_spans_for_content};
use a4n_error::AppError;
use serde_json::Value;

use crate::galgame::{
    GalgameAdaptationProjection, GalgamePipelinePromptContext, GalgamePipelineScene,
    GalgamePipelineStartStage,
};

use super::super::super::progress::ensure_galgame_pipeline_not_cancelled;
use super::super::contracts::GalgamePipelineExecution;

pub(super) async fn run_adaptation_projection_stage(
    execution: &GalgamePipelineExecution<'_>,
) -> Result<(Value, GalgameAdaptationProjection), AppError> {
    let stage = GalgamePipelineStartStage::AdaptationPlan;
    ensure_galgame_pipeline_not_cancelled(execution.service, execution.options.workflow_run_id)
        .await?;
    let scenes = galgame_adaptation_projection(execution.prompt_context)?;
    let scenes_json = serde_json::to_value(&scenes)?;
    execution
        .service
        .report_pipeline_progress(
            execution.options.workflow_run_id,
            execution.options.task_id.as_deref(),
            execution.options.progress_reporter.as_ref(),
            stage,
            1,
            "已从本项目改编计划准备 Galgame 场景",
        )
        .await?;
    Ok((scenes_json, scenes))
}

pub fn galgame_adaptation_projection(
    prompt_context: &GalgamePipelinePromptContext,
) -> Result<GalgameAdaptationProjection, AppError> {
    if prompt_context.adaptations.len() != prompt_context.chapters.len() {
        return Err(AppError::InvalidState {
            current: "galgame_adaptation_count_mismatch".to_string(),
            operation: "投影 Galgame 场景".to_string(),
        });
    }
    let chapters = prompt_context
        .chapters
        .iter()
        .map(|chapter| (chapter.source_chapter_id, chapter))
        .collect::<HashMap<_, _>>();
    let mut scenes = Vec::new();
    for adaptation in &prompt_context.adaptations {
        let chapter =
            chapters
                .get(&adaptation.source_chapter_id)
                .ok_or_else(|| AppError::InvalidState {
                    current: format!(
                        "galgame_adaptation_chapter_missing:{}",
                        adaptation.source_chapter_id
                    ),
                    operation: "投影 Galgame 场景".to_string(),
                })?;
        let source_spans = source_spans_for_content(
            adaptation.source_chapter_id,
            adaptation.chapter_number,
            &chapter.content,
        );
        let source_spans_by_id = source_spans
            .iter()
            .map(|span| (span.id.as_str(), span))
            .collect::<HashMap<_, _>>();
        for scene in &adaptation.plan.units {
            let source_span_ids = map_adaptation_span_ids(
                &scene.source_span_ids,
                &source_spans_by_id,
                &chapter.source_spans,
            )?;
            let mut beats = Vec::with_capacity(scene.beats.len());
            for beat in &scene.beats {
                let mut projected = beat.clone();
                projected.source_span_ids = map_adaptation_span_ids(
                    &beat.source_span_ids,
                    &source_spans_by_id,
                    &chapter.source_spans,
                )?;
                beats.push(projected);
            }
            scenes.push(GalgamePipelineScene {
                origin: scene.origin,
                index: i32::try_from(scenes.len() + 1).map_err(|_| {
                    AppError::InvalidArgument("Galgame 场景数量超出 i32 范围".to_string())
                })?,
                title: scene.title.clone(),
                location: scene.location.clone(),
                time_of_day: None,
                participants: scene.participants.clone(),
                summary: Some(format!(
                    "戏剧问题：{}；目标：{}；阻力：{}；入场：{}；离场：{}",
                    scene.dramatic_question,
                    scene.objective,
                    scene.obstacle,
                    scene.entry_state,
                    scene.exit_state
                )),
                source_span_ids,
                adaptation_plan_id: adaptation.adaptation_plan_id,
                adaptation_plan_revision: adaptation.adaptation_plan_revision,
                source_chapter_id: adaptation.source_chapter_id,
                source_snapshot_chapter_id: adaptation.source_snapshot_chapter_id,
                chapter_number: adaptation.chapter_number,
                unit_id: scene.id.clone(),
                beats,
            });
        }
    }
    if scenes.is_empty() {
        return Err(AppError::InvalidArgument(
            "本项目改编计划没有可用的 Galgame 场景".to_string(),
        ));
    }
    Ok(GalgameAdaptationProjection { scenes })
}

fn map_adaptation_span_ids(
    ids: &[String],
    source_spans_by_id: &HashMap<&str, &SourceSpan>,
    prompt_spans: &[crate::galgame::GalgamePipelinePromptSourceSpan],
) -> Result<Vec<String>, AppError> {
    let mut mapped = BTreeSet::new();
    for id in ids {
        let source = source_spans_by_id.get(id.as_str()).ok_or_else(|| {
            AppError::InvalidArgument(format!("改编计划引用未知 source_span_id: {id}"))
        })?;
        let matches = prompt_spans
            .iter()
            .filter(|span| {
                span.start_offset >= 0
                    && span.end_offset >= span.start_offset
                    && (span.start_offset as usize) <= source.char_start
                    && (span.end_offset as usize) >= source.char_end
            })
            .collect::<Vec<_>>();
        if matches.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "改编计划 source_span_id 的字符范围无法映射到 Galgame text_anchor: {id}"
            )));
        }
        for span in matches {
            mapped.insert(span.id.clone());
        }
    }
    Ok(mapped.into_iter().collect())
}
