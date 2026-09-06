use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_db::chapter_visual_plan::ParsedChapterVisualPlan;
use a4n_domain::chapter_visual_plan::ChapterVisualPlanV1;
use a4n_domain::chapter_visual_plan::{
    VisualNarrativeUnitV1, VisualShotPlanV1, VisualTransitionV1,
};
use a4n_domain::shooting_script::{
    ChapterShootingScriptSchemaVersion, ChapterShootingScriptV1, ShootingDirectingStatementV1,
    ShootingSceneV1, ShootingVisualConstraintsV1, ShootingVisualPlanSourceV1,
};

use super::contracts::ScreenplayPlanUnit;

pub(super) fn assemble_chapter_shooting_script(
    directing_statement: &ShootingDirectingStatementV1,
    units: &[ScreenplayPlanUnit],
    scenes: Vec<ShootingSceneV1>,
    visual_plan_record: &ParsedChapterVisualPlan,
    visual_plan: &ChapterVisualPlanV1,
) -> Result<ChapterShootingScriptV1, AppError> {
    let script = assemble_script(
        directing_statement,
        units,
        scenes,
        visual_plan_record,
        visual_plan,
    )?;
    script
        .validate()
        .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
    Ok(script)
}

pub(super) fn assemble_available_shooting_script(
    directing_statement: &ShootingDirectingStatementV1,
    units: &[ScreenplayPlanUnit],
    scenes: Vec<ShootingSceneV1>,
    visual_plan_record: &ParsedChapterVisualPlan,
    visual_plan: &ChapterVisualPlanV1,
) -> Result<ChapterShootingScriptV1, AppError> {
    if scenes.is_empty() {
        return Err(AppError::InvalidArgument(
            "已完成拍摄设计单元数量无效".to_string(),
        ));
    }
    assemble_script(
        directing_statement,
        units,
        scenes,
        visual_plan_record,
        visual_plan,
    )
}

fn assemble_script(
    directing_statement: &ShootingDirectingStatementV1,
    units: &[ScreenplayPlanUnit],
    scenes: Vec<ShootingSceneV1>,
    visual_plan_record: &ParsedChapterVisualPlan,
    visual_plan: &ChapterVisualPlanV1,
) -> Result<ChapterShootingScriptV1, AppError> {
    if scenes.len() != units.len() {
        return Err(AppError::InvalidArgument(format!(
            "拍摄设计单元数量 {} 与叙事编排单元数量 {} 不一致",
            scenes.len(),
            units.len()
        )));
    }
    if units.len() != visual_plan.narrative_units.len() {
        return Err(AppError::InvalidArgument(
            "拍摄剧本叙事单元数量与 confirmed 导演编排不一致".into(),
        ));
    }
    let visual_index = VisualPlanIndex::new(visual_plan);
    let mut assembled = scenes
        .into_iter()
        .zip(units.iter().zip(&visual_plan.narrative_units))
        .enumerate()
        .map(|(planned_index, (scene, (unit, planned)))| {
            assemble_shooting_scene(unit, planned, planned_index, &visual_index, scene)
        })
        .collect::<Result<Vec<_>, _>>()?;
    assembled.sort_by_key(|scene| scene.timeline_position);
    for (scene_offset, scene) in assembled.iter_mut().enumerate() {
        scene.scene_index = checked_index(scene_offset, "拍摄场景数量")?;
    }
    Ok(ChapterShootingScriptV1 {
        schema_version: ChapterShootingScriptSchemaVersion::V1,
        visual_plan_source: Some(ShootingVisualPlanSourceV1 {
            plan_id: visual_plan_record.record.id,
            plan_version: visual_plan_record.record.plan_version,
            plan_sha256: visual_plan_record.record.plan_sha256.clone(),
        }),
        user_additions: visual_plan.user_additions.clone(),
        directing_statement: directing_statement.clone(),
        scenes: assembled,
    })
}

fn assemble_shooting_scene(
    unit: &super::contracts::ScreenplayPlanUnit,
    planned: &VisualNarrativeUnitV1,
    planned_index: usize,
    visual_index: &VisualPlanIndex<'_>,
    mut scene: ShootingSceneV1,
) -> Result<ShootingSceneV1, AppError> {
    if scene.shots.len() != planned.shots.len() {
        return Err(AppError::InvalidArgument(format!(
            "拍摄设计 unit {} 的镜头数量 {} 与 confirmed 导演编排 {} 不一致",
            unit.narrative_unit_index,
            scene.shots.len(),
            planned.shots.len()
        )));
    }
    scene.scene_identity = unit.scene_identity.clone();
    scene.sequence_index = unit.sequence_index;
    scene.narrative_unit_index = unit.narrative_unit_index;
    scene.timeline_position = unit.timeline_position;
    scene.intercut_group_id = unit.intercut_group_id.clone();
    scene.editing_mode = unit.editing_mode;
    scene.transition_contracts = visual_transition_contracts(planned, planned_index, visual_index)?;
    scene.source_span_ids = planned.source_span_ids.clone();
    scene.visual_narrative_unit_id = Some(planned.id.clone());
    scene.user_addition_ids = planned.user_addition_ids.clone();
    scene.title = planned.title.clone();
    scene.slugline = unit.slugline.clone();
    scene.location = planned.location.clone();
    scene.time_of_day = planned.time_of_day.clone();
    scene.participants = planned
        .participant_entity_ids
        .iter()
        .map(|id| visual_entity_label(visual_index, id))
        .collect::<Result<Vec<_>, _>>()?;
    scene.focal_character = planned
        .focal_entity_id
        .as_deref()
        .map(|id| visual_entity_label(visual_index, id))
        .transpose()?;
    scene.dramatic_question = unit.dramatic_question.clone();
    scene.scene_objective = planned.narrative_purpose.clone();
    scene.scene_obstacle = unit.scene_obstacle.clone();
    scene.entry_state = unit.entry_state.clone();
    scene.exit_state = unit.exit_state.clone();

    for (shot_offset, (shot, planned_shot)) in
        scene.shots.iter_mut().zip(&planned.shots).enumerate()
    {
        shot.shot_index = checked_index(shot_offset, "单个拍摄单元镜头数量")?;
        apply_visual_shot_plan(shot, planned_shot, visual_index);
        let mut beat_ids = HashMap::new();
        for (beat_offset, beat) in shot.performance_beats.iter_mut().enumerate() {
            let model_beat_id = beat.beat_id.clone();
            beat.beat_index = checked_index(beat_offset, "单个镜头表演节拍数量")?;
            beat.beat_id = format!("{}_beat_{}", shot.shot_id, beat.beat_index);
            beat.user_addition_ids = shot.user_addition_ids.clone();
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
        fit_beats_to_planned_duration(shot, planned_shot);
    }
    Ok(scene)
}

fn apply_visual_shot_plan(
    shot: &mut a4n_domain::shooting_script::ShootingShotV1,
    planned: &VisualShotPlanV1,
    visual_index: &VisualPlanIndex<'_>,
) {
    shot.shot_id = planned.id.clone();
    shot.visual_shot_id = Some(planned.id.clone());
    shot.planned_start_ms = Some(planned.start_ms);
    shot.planned_end_ms = Some(planned.end_ms);
    shot.title = planned.title.clone();
    shot.source_span_ids = planned.source_span_ids.clone();
    shot.user_addition_ids = planned.user_addition_ids.clone();
    shot.visual_constraints = Some(ShootingVisualConstraintsV1 {
        keyframes: planned.keyframes.clone(),
        entity_events: planned.entity_events.clone(),
        camera_events: planned.camera_events.clone(),
        sound_events: planned.sound_events.clone(),
        technique_events: planned.technique_events.clone(),
        continuity_notes: planned.continuity_notes.clone(),
    });
    shot.coverage = planned.coverage;
    shot.narrative_purpose = planned.narrative_purpose.clone();
    if let Some(value) = planned.shot_size {
        shot.shot_size = value;
    }
    if let Some(value) = planned.camera_angle {
        shot.camera_angle = value;
    }
    if let Some(value) = planned.camera_height {
        shot.camera_height = value;
    }
    if let Some(value) = planned.lens_intent.as_ref() {
        shot.lens_intent = value.clone();
    }
    if let Some(value) = planned.composition.as_ref() {
        shot.composition = value.clone();
    }
    shot.continuity_requirements = planned.continuity_notes.clone();
    if let Some(transition) = visual_index.transition_in.get(planned.id.as_str()) {
        shot.transition_in = transition.kind.as_str().to_string();
    }
    if let Some(transition) = visual_index.transition_out.get(planned.id.as_str()) {
        shot.transition_out = transition.kind.as_str().to_string();
    }
}

fn fit_beats_to_planned_duration(
    shot: &mut a4n_domain::shooting_script::ShootingShotV1,
    planned: &VisualShotPlanV1,
) {
    let duration = (planned.end_ms - planned.start_ms) as f64 / 1000.0;
    if !shot.performance_beats.is_empty() {
        let current = shot
            .performance_beats
            .iter()
            .map(|beat| beat.estimated_seconds)
            .sum::<f64>();
        if current > 0.0 {
            let scale = duration / current;
            for beat in &mut shot.performance_beats {
                beat.estimated_seconds *= scale;
            }
        }
    }
    shot.estimated_natural_seconds = duration;
}

fn visual_entity_label(
    visual_index: &VisualPlanIndex<'_>,
    entity_id: &str,
) -> Result<String, AppError> {
    visual_index
        .entity_labels
        .get(entity_id)
        .map(|label| (*label).to_string())
        .ok_or_else(|| AppError::InvalidArgument(format!("视觉实体不存在: {entity_id}")))
}

fn visual_transition_contracts(
    planned: &VisualNarrativeUnitV1,
    planned_index: usize,
    visual_index: &VisualPlanIndex<'_>,
) -> Result<Vec<a4n_domain::shooting_script::ShootingTransitionContractV1>, AppError> {
    visual_index
        .transitions_by_unit
        .get(planned_index)
        .into_iter()
        .flatten()
        .filter(|(_, target)| visual_index.unit_ids[*target] != planned.id)
        .map(|(transition, target)| {
            Ok(a4n_domain::shooting_script::ShootingTransitionContractV1 {
                to_narrative_unit_index: checked_index(*target, "视觉编排叙事单元数量")?,
                transition_type: transition.kind,
                preserved_action: transition.preserved_action.clone(),
                preserved_sound: transition.preserved_sound.clone(),
                preserved_knowledge: Vec::new(),
                required_cut_point: Some(format!("{}ms", transition.at_ms)),
            })
        })
        .collect()
}

struct VisualPlanIndex<'a> {
    entity_labels: HashMap<&'a str, &'a str>,
    unit_ids: Vec<&'a str>,
    transitions_by_unit: Vec<Vec<(&'a VisualTransitionV1, usize)>>,
    transition_in: HashMap<&'a str, &'a VisualTransitionV1>,
    transition_out: HashMap<&'a str, &'a VisualTransitionV1>,
}

impl<'a> VisualPlanIndex<'a> {
    fn new(plan: &'a ChapterVisualPlanV1) -> Self {
        let entity_labels = plan
            .entities
            .iter()
            .map(|entity| (entity.id.as_str(), entity.label.as_str()))
            .collect::<HashMap<_, _>>();

        let unit_ids = plan
            .narrative_units
            .iter()
            .map(|unit| unit.id.as_str())
            .collect::<Vec<_>>();
        let shot_unit_indices = plan
            .narrative_units
            .iter()
            .enumerate()
            .flat_map(|(unit_index, unit)| {
                unit.shots
                    .iter()
                    .map(move |shot| (shot.id.as_str(), unit_index))
            })
            .collect::<HashMap<_, _>>();

        let mut transitions_by_unit = vec![Vec::new(); plan.narrative_units.len()];
        let mut transition_in = HashMap::with_capacity(plan.transitions.len());
        let mut transition_out = HashMap::with_capacity(plan.transitions.len());
        for transition in &plan.transitions {
            transition_in.insert(transition.to_shot_id.as_str(), transition);
            transition_out.insert(transition.from_shot_id.as_str(), transition);
            let source_unit_index = shot_unit_indices[transition.from_shot_id.as_str()];
            let target_unit_index = shot_unit_indices[transition.to_shot_id.as_str()];
            transitions_by_unit[source_unit_index].push((transition, target_unit_index));
        }

        Self {
            entity_labels,
            unit_ids,
            transitions_by_unit,
            transition_in,
            transition_out,
        }
    }
}

fn checked_index(offset: usize, label: &str) -> Result<u32, AppError> {
    u32::try_from(offset + 1)
        .map_err(|_| AppError::InvalidArgument(format!("{label}超出 u32 范围")))
}

#[cfg(test)]
mod tests {
    use a4n_core::{source_spans::SourceSpan, source_spans::source_spans_for_content};
    use a4n_db::project::ResolvedChapterSource;
    use a4n_domain::chapter_visual_plan::{VisualEntityKindV1, VisualEntityV1, VisualTransitionV1};
    use a4n_domain::shooting_script::ShootingTransitionTypeV1;

    use super::*;

    fn fixture() -> ChapterVisualPlanV1 {
        let source = ResolvedChapterSource {
            requested_project_id: 1,
            root_project_id: 1,
            source_chapter_id: 3,
            source_snapshot_id: None,
            source_snapshot_chapter_id: None,
            chapter_number: 1,
            title: Some("测试".into()),
            summary: None,
            analysis_data_json: None,
            content: "甲进入房间。乙从窗边回头。".into(),
            content_sha256: "content-hash".into(),
            source_revision_key: "revision".into(),
        };
        let spans: Vec<SourceSpan> = source_spans_for_content(
            source.source_chapter_id,
            source.chapter_number,
            &source.content,
        );
        let visual_plan = crate::shooting_script::contracts::visual_plan_fixture(&source);
        crate::chapter_visual_plan::project_visual_plan_for_shooting_script(
            &visual_plan.plan,
            &spans,
        )
        .expect("visual plan projection")
    }

    #[test]
    fn visual_plan_index_tracks_labels_and_transition_order() {
        let mut visual_plan = fixture();
        let source_span_id = visual_plan.narrative_units[0].source_span_ids[0].clone();
        visual_plan.entities = vec![VisualEntityV1 {
            id: "entity_1".into(),
            label: "首个标签".into(),
            kind: VisualEntityKindV1::Character,
            description: "实体".into(),
            source_span_ids: vec![source_span_id],
            user_addition_ids: Vec::new(),
        }];
        let shots = &visual_plan.narrative_units[0].shots;
        assert!(shots.len() >= 2);
        let first = VisualTransitionV1 {
            id: "transition_1".into(),
            from_shot_id: shots[0].id.clone(),
            to_shot_id: shots[1].id.clone(),
            at_ms: shots[0].end_ms,
            duration_ms: 0,
            kind: ShootingTransitionTypeV1::HardCut,
            preserved_action: Some("动作".into()),
            preserved_sound: None,
            notes: None,
        };
        visual_plan.transitions = vec![first];

        let index = VisualPlanIndex::new(&visual_plan);

        assert_eq!(
            index.entity_labels.get("entity_1").copied(),
            Some("首个标签")
        );
        assert_eq!(
            index
                .transition_in
                .get(shots[1].id.as_str())
                .map(|t| t.kind),
            Some(ShootingTransitionTypeV1::HardCut)
        );
        assert_eq!(
            index
                .transition_out
                .get(shots[0].id.as_str())
                .map(|t| t.kind),
            Some(ShootingTransitionTypeV1::HardCut)
        );
        assert_eq!(index.transitions_by_unit[0].len(), 1);
    }
}
