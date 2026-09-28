use std::collections::HashSet;

use a4n_error::AppError;
use serde_json::Value;

use super::contracts::*;
use super::lines::line_draft_from_raw;
use super::rules::*;

pub fn adaptation_projection_from_value(
    value: Value,
    context: &str,
) -> Result<GalgameAdaptationProjection, AppError> {
    let raw: AdaptationProjectionRaw = from_value(value, context)?;
    require_nonempty(raw.scenes.as_slice(), &format!("{context}.scenes"))?;
    let mut seen = HashSet::new();
    let mut scenes = Vec::with_capacity(raw.scenes.len());
    for scene in raw.scenes {
        require_positive(scene.index, &format!("{context}.scenes[].index"))?;
        if !seen.insert(scene.index) {
            return Err(invalid(format!(
                "{context}.scenes.index 重复: {}",
                scene.index
            )));
        }
        require_positive_i64(
            scene.adaptation_plan_id,
            &format!("{context}.scenes[].adaptation_plan_id"),
        )?;
        require_positive(
            scene.adaptation_plan_revision,
            &format!("{context}.scenes[].adaptation_plan_revision"),
        )?;
        require_positive(
            scene.chapter_number,
            &format!("{context}.scenes[].chapter_number"),
        )?;
        require_positive_i64(
            scene.source_chapter_id,
            &format!("{context}.scenes[].source_chapter_id"),
        )?;
        require_positive_i64(
            scene.source_snapshot_chapter_id,
            &format!("{context}.scenes[].source_snapshot_chapter_id"),
        )?;
        let unit_id = required_text(scene.unit_id, &format!("{context}.scenes[].unit_id"))?;
        let source_span_ids = normalize_source_span_ids(
            scene.source_span_ids,
            &format!("{context}.scenes[].source_span_ids"),
            false,
        )?;
        let source_span_id_set = source_span_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        require_nonempty(scene.beats.as_slice(), &format!("{context}.scenes[].beats"))?;
        let mut beats = Vec::with_capacity(scene.beats.len());
        let mut beat_ids = HashSet::new();
        for mut beat in scene.beats {
            beat.id = required_text(beat.id, &format!("{context}.scenes[].beats[].id"))?;
            if !beat_ids.insert(beat.id.clone()) {
                return Err(invalid(format!("{context} 改编节拍 ID 重复: {}", beat.id)));
            }
            let beat_source_span_ids = normalize_source_span_ids(
                beat.source_span_ids,
                &format!("{context}.scenes[].beats[].source_span_ids"),
                false,
            )?;
            if let Some(unknown_span_id) = beat_source_span_ids
                .iter()
                .find(|span_id| !source_span_id_set.contains(span_id.as_str()))
            {
                return Err(invalid(format!(
                    "{context}.scenes[].beats[].source_span_ids 包含不属于所在场景的引用: {unknown_span_id}"
                )));
            }
            beat.action =
                required_text(beat.action, &format!("{context}.scenes[].beats[].action"))?;
            beat.source_span_ids = beat_source_span_ids;
            beats.push(beat);
        }
        scenes.push(GalgamePipelineScene {
            index: scene.index,
            origin: scene.origin,
            title: required_text(scene.title, &format!("{context}.scenes[].title"))?,
            location: optional_text(scene.location),
            time_of_day: optional_text(scene.time_of_day),
            participants: normalize_text_vec(
                scene.participants,
                &format!("{context}.scenes[].participants"),
            )?,
            summary: optional_text(scene.summary),
            source_span_ids,
            adaptation_plan_id: scene.adaptation_plan_id,
            adaptation_plan_revision: scene.adaptation_plan_revision,
            source_chapter_id: scene.source_chapter_id,
            source_snapshot_chapter_id: scene.source_snapshot_chapter_id,
            chapter_number: scene.chapter_number,
            unit_id,
            beats,
        });
    }
    ensure_contiguous_indexes(
        scenes.iter().map(|scene| scene.index),
        &format!("{context}.scenes"),
    )?;
    Ok(GalgameAdaptationProjection { scenes })
}

pub fn script_adaptation_from_value(
    value: Value,
    context: &str,
) -> Result<GalgameScriptAdaptation, AppError> {
    let raw: ScriptAdaptationRaw = from_value(value, context)?;
    require_nonempty(
        raw.scene_lines.as_slice(),
        &format!("{context}.scene_lines"),
    )?;
    let scene_lines = raw
        .scene_lines
        .into_iter()
        .map(|scene| {
            require_positive(
                scene.scene_index,
                &format!("{context}.scene_lines[].scene_index"),
            )?;
            require_nonempty(
                scene.lines.as_slice(),
                &format!("{context}.scene_lines[].lines"),
            )?;
            let lines = scene
                .lines
                .into_iter()
                .map(|line| line_draft_from_raw(line, context))
                .collect::<Result<Vec<_>, _>>()?;
            ensure_contiguous_indexes(
                lines.iter().map(|line| line.index),
                &format!("{context}.scene_lines[{}].lines", scene.scene_index),
            )?;
            Ok(GalgamePipelineSceneLineDraft {
                scene_index: scene.scene_index,
                lines,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    ensure_unique_scene_indexes(scene_lines.iter().map(|item| item.scene_index), context)?;
    let branch_scene_lines = raw
        .branch_scene_lines
        .into_iter()
        .map(|branch| {
            let branch_id = required_text(
                branch.branch_id,
                &format!("{context}.branch_scene_lines[].branch_id"),
            )?;
            require_nonempty(
                branch.lines.as_slice(),
                &format!("{context}.branch_scene_lines[].lines"),
            )?;
            let lines = branch
                .lines
                .into_iter()
                .map(|line| line_draft_from_raw(line, context))
                .collect::<Result<Vec<_>, _>>()?;
            ensure_contiguous_indexes(
                lines.iter().map(|line| line.index),
                &format!("{context}.branch_scene_lines[{branch_id}].lines"),
            )?;
            Ok(GalgamePipelineBranchSceneLineDraft { branch_id, lines })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let mut branch_ids = HashSet::new();
    for branch in &branch_scene_lines {
        if !branch_ids.insert(branch.branch_id.as_str()) {
            return Err(invalid(format!(
                "{context}.branch_scene_lines.branch_id 重复: {}",
                branch.branch_id
            )));
        }
    }
    Ok(GalgameScriptAdaptation {
        scene_lines,
        branch_scene_lines,
    })
}
