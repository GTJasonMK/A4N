use std::collections::{BTreeMap, BTreeSet};

use a4n_error::AppError;

use super::super::super::{
    GalgameCharacterStaging, GalgamePipelineBackgroundPlan, GalgamePipelineEventCgPlan,
    GalgamePipelineScene, GalgameVisualPlan,
};
use super::contracts::*;
use super::rules::invalid;

pub(super) fn validate_staging_scene_indexes(
    staged_line_indexes_by_scene: &BTreeMap<i32, BTreeSet<i32>>,
    scene_indexes: &BTreeSet<i32>,
) -> Result<(), AppError> {
    for scene_index in staged_line_indexes_by_scene.keys() {
        if !scene_indexes.contains(scene_index) {
            return Err(invalid(format!(
                "Galgame character_staging 引用不存在的 scene_index: {scene_index}"
            )));
        }
    }
    Ok(())
}

pub(super) type VisualPlanMappings<'a> = (
    BTreeMap<i32, &'a GalgamePipelineBackgroundPlan>,
    BTreeMap<(i32, i32), &'a GalgamePipelineEventCgPlan>,
);

pub(super) fn visual_plan_mappings<'a>(
    visual_plan: &'a GalgameVisualPlan,
    scenes: &[GalgamePipelineScene],
    staging: &GalgameCharacterStaging,
    source_span_by_id: &BTreeMap<&str, &GalgamePipelineSourceSpan>,
) -> Result<VisualPlanMappings<'a>, AppError> {
    let scenes_by_index = scenes
        .iter()
        .map(|scene| (scene.index, scene))
        .collect::<BTreeMap<_, _>>();
    let staged_by_scene = staging
        .scene_lines
        .iter()
        .map(|scene| (scene.scene_index, scene))
        .collect::<BTreeMap<_, _>>();
    let mut background_by_scene = BTreeMap::new();
    for background in &visual_plan.backgrounds {
        let scene = scenes_by_index
            .get(&background.scene_index)
            .ok_or_else(|| invalid("背景引用的改编场景不存在"))?;
        a4n_domain::media_authoring::validate_adaptation_evidence(
            scene.origin,
            &background.source_span_ids,
            &scene.source_span_ids.iter().cloned().collect(),
        )
        .map_err(invalid)?;
        validate_visual_source_spans(
            &background.source_span_ids,
            source_span_by_id,
            "Galgame visual_planning background",
        )?;
        if background_by_scene
            .insert(background.scene_index, background)
            .is_some()
        {
            return Err(invalid(format!(
                "Galgame visual_planning 同一场只能有一个 background: scene_index={}",
                background.scene_index
            )));
        }
    }
    for scene_index in scenes_by_index.keys() {
        if !background_by_scene.contains_key(scene_index) {
            return Err(invalid(format!(
                "Galgame visual_planning 缺少 scene_index={} 的 background",
                scene_index
            )));
        }
    }

    let mut cg_by_scene_line = BTreeMap::new();
    for cg in &visual_plan.event_cgs {
        let line = staged_by_scene
            .get(&cg.scene_index)
            .and_then(|scene| scene.lines.iter().find(|line| line.index == cg.line_index))
            .ok_or_else(|| invalid("CG 引用的剧本行不存在"))?;
        a4n_domain::media_authoring::validate_adaptation_evidence(
            line.origin,
            &cg.source_span_ids,
            &line.source_span_ids.iter().cloned().collect(),
        )
        .map_err(invalid)?;
        validate_visual_source_spans(
            &cg.source_span_ids,
            source_span_by_id,
            "Galgame visual_planning event_cg",
        )?;
        if cg_by_scene_line
            .insert((cg.scene_index, cg.line_index), cg)
            .is_some()
        {
            return Err(invalid(format!(
                "Galgame visual_planning 同一行只能有一个 event_cg: scene_index={}, line_index={}",
                cg.scene_index, cg.line_index
            )));
        }
    }
    Ok((background_by_scene, cg_by_scene_line))
}

pub(super) fn validate_visual_source_spans(
    source_span_ids: &[String],
    source_span_by_id: &BTreeMap<&str, &GalgamePipelineSourceSpan>,
    label: &str,
) -> Result<(), AppError> {
    for source_span_id in source_span_ids {
        if !source_span_by_id.contains_key(source_span_id.as_str()) {
            return Err(invalid(format!(
                "{label} source_span_id 不存在: {source_span_id}"
            )));
        }
    }
    Ok(())
}
