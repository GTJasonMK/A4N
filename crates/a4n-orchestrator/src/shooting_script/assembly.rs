use std::collections::HashMap;

use a4n_core::{
    error::AppError,
    shooting_script::{CHAPTER_SHOOTING_SCRIPT_V1, ChapterShootingScriptV1, ShootingSceneV1},
};

use super::contracts::ChapterScreenplayPlan;

pub(super) fn assemble_chapter_shooting_script(
    plan: &ChapterScreenplayPlan,
    scenes: Vec<ShootingSceneV1>,
) -> Result<ChapterShootingScriptV1, AppError> {
    if scenes.len() != plan.units.len() {
        return Err(AppError::InvalidArgument(format!(
            "拍摄设计单元数量 {} 与叙事编排单元数量 {} 不一致",
            scenes.len(),
            plan.units.len()
        )));
    }
    let mut assembled = Vec::with_capacity(scenes.len());
    for (mut scene, unit) in scenes.into_iter().zip(&plan.units) {
        scene.scene_identity = unit.scene_identity.clone();
        scene.sequence_index = unit.sequence_index;
        scene.narrative_unit_index = unit.narrative_unit_index;
        scene.timeline_position = unit.timeline_position;
        scene.intercut_group_id = unit.intercut_group_id.clone();
        scene.editing_mode = unit.editing_mode;
        scene.transition_contracts = unit.transition_contracts.clone();
        scene.source_span_ids = unit.source_span_ids.clone();
        scene.title = unit.title.clone();
        scene.slugline = unit.slugline.clone();
        scene.participants = unit.participants.clone();
        scene.dramatic_question = unit.dramatic_question.clone();
        scene.scene_objective = unit.scene_objective.clone();
        scene.scene_obstacle = unit.scene_obstacle.clone();
        scene.entry_state = unit.entry_state.clone();
        scene.exit_state = unit.exit_state.clone();

        for (shot_offset, shot) in scene.shots.iter_mut().enumerate() {
            shot.shot_index = checked_index(shot_offset, "单个拍摄单元镜头数量")?;
            shot.shot_id = format!(
                "unit_{}_shot_{}",
                scene.narrative_unit_index, shot.shot_index
            );
            let mut beat_ids = HashMap::new();
            for (beat_offset, beat) in shot.performance_beats.iter_mut().enumerate() {
                let model_beat_id = beat.beat_id.clone();
                beat.beat_index = checked_index(beat_offset, "单个镜头表演节拍数量")?;
                beat.beat_id = format!("{}_beat_{}", shot.shot_id, beat.beat_index);
                if beat_ids
                    .insert(model_beat_id.clone(), beat.beat_id.clone())
                    .is_some()
                {
                    return Err(AppError::InvalidArgument(format!(
                        "design unit {} 的模型 beat_id 重复: {}",
                        scene.narrative_unit_index, model_beat_id
                    )));
                }
            }
            for cue in &mut shot.dialogue_cues {
                cue.beat_id = beat_ids.get(&cue.beat_id).cloned().ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "design unit {} 的 dialogue cue 引用了不存在的 beat_id: {}",
                        scene.narrative_unit_index, cue.beat_id
                    ))
                })?;
            }
            shot.estimated_natural_seconds = shot
                .performance_beats
                .iter()
                .map(|beat| beat.estimated_seconds)
                .sum();
        }
        assembled.push(scene);
    }
    assembled.sort_by_key(|scene| scene.timeline_position);
    for (scene_offset, scene) in assembled.iter_mut().enumerate() {
        scene.scene_index = checked_index(scene_offset, "拍摄场景数量")?;
    }
    let script = ChapterShootingScriptV1 {
        schema_version: CHAPTER_SHOOTING_SCRIPT_V1.to_string(),
        directing_statement: plan.directing_statement.clone(),
        scenes: assembled,
    };
    script
        .validate()
        .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
    Ok(script)
}

fn checked_index(offset: usize, label: &str) -> Result<u32, AppError> {
    u32::try_from(offset + 1)
        .map_err(|_| AppError::InvalidArgument(format!("{label}超出 u32 范围")))
}
