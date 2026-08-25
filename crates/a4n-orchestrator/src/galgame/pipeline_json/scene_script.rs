use std::collections::HashSet;

use a4n_core::error::AppError;
use a4n_domain::shooting_script::ShootingCoverageV1;
use serde_json::Value;

use super::contracts::*;
use super::lines::line_draft_from_raw;
use super::rules::*;

pub fn shooting_script_projection_from_value(
    value: Value,
    context: &str,
) -> Result<GalgameShootingScriptProjection, AppError> {
    let raw: ShootingScriptProjectionRaw = from_value(value, context)?;
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
            scene.shooting_script_id,
            &format!("{context}.scenes[].shooting_script_id"),
        )?;
        require_positive(
            scene.shooting_script_revision,
            &format!("{context}.scenes[].shooting_script_revision"),
        )?;
        require_positive(
            scene.chapter_number,
            &format!("{context}.scenes[].chapter_number"),
        )?;
        if scene.shooting_scene_index == 0 {
            return Err(invalid(format!(
                "{context}.scenes[].shooting_scene_index 必须是正整数"
            )));
        }
        let source_span_ids = normalize_source_span_ids(
            scene.source_span_ids,
            &format!("{context}.scenes[].source_span_ids"),
            true,
        )?;
        let source_span_id_set = source_span_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        require_nonempty(scene.beats.as_slice(), &format!("{context}.scenes[].beats"))?;
        let mut has_must_show = false;
        let mut beats = Vec::with_capacity(scene.beats.len());
        let mut previous_ref: Option<(u32, u32)> = None;
        for beat in scene.beats {
            if beat.shot_index == 0 || beat.beat_index == 0 {
                return Err(invalid(format!(
                    "{context}.scenes[].beats[].shot_index 和 beat_index 必须是正整数"
                )));
            }
            let valid_sequence = match previous_ref {
                None => beat.shot_index == 1 && beat.beat_index == 1,
                Some((shot_index, beat_index)) if beat.shot_index == shot_index => {
                    beat_index.checked_add(1) == Some(beat.beat_index)
                }
                Some((shot_index, _)) => {
                    shot_index.checked_add(1) == Some(beat.shot_index) && beat.beat_index == 1
                }
            };
            if !valid_sequence {
                return Err(invalid(format!(
                    "{context}.scenes[].beats 必须按 shot_index、beat_index 连续排列，实际 shot_index={} beat_index={}",
                    beat.shot_index, beat.beat_index
                )));
            }
            previous_ref = Some((beat.shot_index, beat.beat_index));
            let beat_source_span_ids = normalize_source_span_ids(
                beat.source_span_ids,
                &format!("{context}.scenes[].beats[].source_span_ids"),
                true,
            )?;
            if let Some(unknown_span_id) = beat_source_span_ids
                .iter()
                .find(|span_id| !source_span_id_set.contains(span_id.as_str()))
            {
                return Err(invalid(format!(
                    "{context}.scenes[].beats[].source_span_ids 包含不属于所在场景的引用: {unknown_span_id}"
                )));
            }
            has_must_show |= beat.required || beat.coverage == ShootingCoverageV1::MustShow;
            beats.push(GalgamePipelineShootingBeat {
                shot_index: beat.shot_index,
                beat_index: beat.beat_index,
                coverage: beat.coverage,
                required: beat.required,
                objective: required_text(
                    beat.objective,
                    &format!("{context}.scenes[].beats[].objective"),
                )?,
                obstacle: required_text(
                    beat.obstacle,
                    &format!("{context}.scenes[].beats[].obstacle"),
                )?,
                visible_action: required_text(
                    beat.visible_action,
                    &format!("{context}.scenes[].beats[].visible_action"),
                )?,
                reaction: optional_text(beat.reaction),
                dialogue_intent: optional_text(beat.dialogue_intent),
                audience_question: required_text(
                    beat.audience_question,
                    &format!("{context}.scenes[].beats[].audience_question"),
                )?,
                answer_or_delay: required_text(
                    beat.answer_or_delay,
                    &format!("{context}.scenes[].beats[].answer_or_delay"),
                )?,
                entry_state: required_text(
                    beat.entry_state,
                    &format!("{context}.scenes[].beats[].entry_state"),
                )?,
                exit_state: required_text(
                    beat.exit_state,
                    &format!("{context}.scenes[].beats[].exit_state"),
                )?,
                source_span_ids: beat_source_span_ids,
            });
        }
        if !has_must_show {
            return Err(invalid(format!(
                "{context}.scenes[].beats 每个拍摄场至少需要一个 required 或 must_show 节拍"
            )));
        }
        scenes.push(GalgamePipelineScene {
            index: scene.index,
            title: required_text(scene.title, &format!("{context}.scenes[].title"))?,
            location: optional_text(scene.location),
            time_of_day: optional_text(scene.time_of_day),
            participants: normalize_text_vec(
                scene.participants,
                &format!("{context}.scenes[].participants"),
            )?,
            summary: optional_text(scene.summary),
            source_span_ids,
            shooting_script_id: scene.shooting_script_id,
            shooting_script_revision: scene.shooting_script_revision,
            chapter_number: scene.chapter_number,
            shooting_scene_index: scene.shooting_scene_index,
            beats,
        });
    }
    ensure_contiguous_indexes(
        scenes.iter().map(|scene| scene.index),
        &format!("{context}.scenes"),
    )?;
    Ok(GalgameShootingScriptProjection { scenes })
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
