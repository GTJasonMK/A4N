use std::collections::{BTreeSet, HashMap};

use a4n_domain::source_spans::source_spans_for_content;
use a4n_error::AppError;
use serde_json::Value;

use crate::galgame::{
    GalgamePipelinePromptContext, GalgamePipelineScene, GalgamePipelineShootingBeat,
    GalgamePipelineStartStage, GalgameShootingScriptProjection,
};

use super::super::super::progress::ensure_galgame_pipeline_not_cancelled;
use super::super::contracts::GalgamePipelineExecution;

pub(super) async fn run_shooting_script_projection_stage(
    execution: &GalgamePipelineExecution<'_>,
) -> Result<(Value, GalgameShootingScriptProjection), AppError> {
    let stage = GalgamePipelineStartStage::ShootingScriptProjection;
    ensure_galgame_pipeline_not_cancelled(execution.service, execution.options.workflow_run_id)
        .await?;
    let scenes = galgame_shooting_script_projection(execution.prompt_context)?;
    let scenes_json = serde_json::to_value(&scenes)?;
    execution
        .service
        .report_pipeline_progress(
            execution.options.workflow_run_id,
            execution.options.task_id.as_deref(),
            execution.options.progress_reporter.as_ref(),
            stage,
            1,
            "已从结构化拍摄剧本投影 Galgame 场景",
        )
        .await?;
    Ok((scenes_json, scenes))
}

pub fn galgame_shooting_script_projection(
    prompt_context: &GalgamePipelinePromptContext,
) -> Result<GalgameShootingScriptProjection, AppError> {
    if prompt_context.shooting_scripts.len() != prompt_context.chapters.len() {
        return Err(AppError::InvalidState {
            current: "galgame_shooting_script_count_mismatch".to_string(),
            operation: "投影 Galgame 场景".to_string(),
        });
    }
    let chapters = prompt_context
        .chapters
        .iter()
        .map(|chapter| (chapter.source_chapter_id, chapter))
        .collect::<HashMap<_, _>>();
    let mut scenes = Vec::new();
    for shooting_script in &prompt_context.shooting_scripts {
        let chapter = chapters
            .get(&shooting_script.source_chapter_id)
            .ok_or_else(|| AppError::InvalidState {
                current: format!(
                    "galgame_shooting_script_chapter_missing:{}",
                    shooting_script.source_chapter_id
                ),
                operation: "投影 Galgame 场景".to_string(),
            })?;
        let source_spans = source_spans_for_content(
            shooting_script.source_chapter_id,
            shooting_script.chapter_number,
            &chapter.content,
        );
        let source_text_by_id = source_spans
            .iter()
            .map(|span| (span.id.as_str(), span.text.as_str()))
            .collect::<HashMap<_, _>>();
        for scene in &shooting_script.script.scenes {
            let source_span_ids = map_shooting_span_ids(
                &scene.source_span_ids,
                &source_text_by_id,
                &chapter.source_spans,
            )?;
            let beat_capacity = scene
                .shots
                .iter()
                .map(|shot| shot.performance_beats.len())
                .sum();
            let mut beats = Vec::with_capacity(beat_capacity);
            for shot in &scene.shots {
                for beat in &shot.performance_beats {
                    beats.push(GalgamePipelineShootingBeat {
                        shot_index: shot.shot_index,
                        beat_index: beat.beat_index,
                        coverage: shot.coverage,
                        required: beat.required,
                        objective: beat.objective.clone(),
                        obstacle: beat.obstacle.clone(),
                        visible_action: beat.visible_action.clone(),
                        reaction: beat.reaction.clone(),
                        dialogue_intent: beat.dialogue_intent.clone(),
                        audience_question: beat.audience_question.clone(),
                        answer_or_delay: beat.answer_or_delay.clone(),
                        entry_state: beat.entry_state.clone(),
                        exit_state: beat.exit_state.clone(),
                        source_span_ids: map_shooting_span_ids(
                            &beat.source_span_ids,
                            &source_text_by_id,
                            &chapter.source_spans,
                        )?,
                    });
                }
            }
            scenes.push(GalgamePipelineScene {
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
                    scene.scene_objective,
                    scene.scene_obstacle,
                    scene.entry_state,
                    scene.exit_state
                )),
                source_span_ids,
                shooting_script_id: shooting_script.shooting_script_id,
                shooting_script_revision: shooting_script.shooting_script_revision,
                chapter_number: shooting_script.chapter_number,
                shooting_scene_index: scene.scene_index,
                beats,
            });
        }
    }
    if scenes.is_empty() {
        return Err(AppError::InvalidArgument(
            "结构化拍摄剧本没有可投影的 Galgame 场景".to_string(),
        ));
    }
    Ok(GalgameShootingScriptProjection { scenes })
}

fn map_shooting_span_ids(
    ids: &[String],
    source_text_by_id: &HashMap<&str, &str>,
    prompt_spans: &[crate::galgame::GalgamePipelinePromptSourceSpan],
) -> Result<Vec<String>, AppError> {
    let mut mapped = BTreeSet::new();
    for id in ids {
        let text = source_text_by_id.get(id.as_str()).ok_or_else(|| {
            AppError::InvalidArgument(format!("结构化拍摄剧本引用未知 source_span_id: {id}"))
        })?;
        let matches = prompt_spans
            .iter()
            .filter(|span| span.text.contains(*text) || text.contains(span.text.as_str()))
            .collect::<Vec<_>>();
        if matches.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "结构化拍摄剧本 source_span_id 无法映射到 Galgame text_anchor: {id}"
            )));
        }
        for span in matches {
            mapped.insert(span.id.clone());
        }
    }
    Ok(mapped.into_iter().collect())
}
