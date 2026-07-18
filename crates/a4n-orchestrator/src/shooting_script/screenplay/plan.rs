use std::collections::HashSet;

use a4n_core::{error::AppError, source_spans::SourceSpan};

use super::super::contracts::ChapterScreenplayPlan;

pub(in crate::shooting_script) fn validate_screenplay_plan(
    plan: &ChapterScreenplayPlan,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    if plan.units.is_empty() {
        return Err(AppError::InvalidArgument(
            "章节剧本计划 units 不能为空".into(),
        ));
    }
    let allowed = source_spans
        .iter()
        .map(|span| span.id.as_str())
        .collect::<HashSet<_>>();
    let mut narrative_units = HashSet::new();
    let mut timeline_positions = HashSet::new();
    let known_units = plan
        .units
        .iter()
        .map(|scene| scene.narrative_unit_index)
        .collect::<HashSet<_>>();
    for (scene_offset, scene) in plan.units.iter().enumerate() {
        let expected_unit_index = u32::try_from(scene_offset + 1)
            .map_err(|_| AppError::InvalidArgument("叙事编排单元数量超出 u32 范围".into()))?;
        if scene.narrative_unit_index != expected_unit_index || scene.performance_facts.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "章节剧本计划 narrative_unit_index={} 编号不连续或 performance_facts 为空",
                scene.narrative_unit_index
            )));
        }
        if scene.scene_identity.trim().is_empty()
            || scene.sequence_index == 0
            || scene.narrative_unit_index == 0
            || scene.timeline_position == 0
            || scene.source_span_ids.is_empty()
        {
            return Err(AppError::InvalidArgument(format!(
                "章节剧本计划 unit {} 的身份、编排编号或来源锚点无效",
                scene.narrative_unit_index
            )));
        }
        if !narrative_units.insert(scene.narrative_unit_index) {
            return Err(AppError::InvalidArgument(format!(
                "章节剧本计划 narrative_unit_index={} 重复",
                scene.narrative_unit_index
            )));
        }
        if !timeline_positions.insert(scene.timeline_position) {
            return Err(AppError::InvalidArgument(format!(
                "章节剧本计划 timeline_position={} 重复",
                scene.timeline_position
            )));
        }
        if matches!(
            scene.editing_mode,
            a4n_core::shooting_script::ShootingEditingModeV1::Intercut
                | a4n_core::shooting_script::ShootingEditingModeV1::ParallelAction
        ) && scene.intercut_group_id.as_deref().is_none_or(str::is_empty)
        {
            return Err(AppError::InvalidArgument(format!(
                "章节剧本计划 unit {} 的交叉剪辑缺少 intercut_group_id",
                scene.narrative_unit_index
            )));
        }
        for transition in &scene.transition_contracts {
            if transition.transition_type.trim().is_empty()
                || !known_units.contains(&transition.to_narrative_unit_index)
                || transition.to_narrative_unit_index == scene.narrative_unit_index
            {
                return Err(AppError::InvalidArgument(format!(
                    "章节剧本计划 unit {} 含无效 transition contract",
                    scene.narrative_unit_index
                )));
            }
        }
        for (index, fact) in scene.performance_facts.iter().enumerate() {
            if fact.fact_index != index as u32 + 1 {
                return Err(AppError::InvalidArgument(format!(
                    "章节剧本计划 unit {} fact_index={} 编号不连续",
                    scene.narrative_unit_index, fact.fact_index
                )));
            }
            if fact.source_span_ids.is_empty()
                || fact
                    .source_span_ids
                    .iter()
                    .any(|id| !scene.source_span_ids.contains(id))
            {
                return Err(AppError::InvalidArgument(format!(
                    "章节剧本计划 unit {} fact {} 引用了当前单元范围外的 source span",
                    scene.narrative_unit_index, fact.fact_index
                )));
            }
        }
        for id in scene.source_span_ids.iter().chain(
            scene
                .performance_facts
                .iter()
                .flat_map(|fact| fact.source_span_ids.iter()),
        ) {
            if !allowed.contains(id.as_str()) {
                return Err(AppError::InvalidArgument(format!(
                    "章节剧本计划引用了输入外 source_span_id: {id}"
                )));
            }
        }
    }
    let expected_timeline_positions = (1..=plan.units.len())
        .map(|position| {
            u32::try_from(position)
                .map_err(|_| AppError::InvalidArgument("叙事编排时间线超出 u32 范围".into()))
        })
        .collect::<Result<HashSet<_>, _>>()?;
    if timeline_positions != expected_timeline_positions {
        return Err(AppError::InvalidArgument(
            "章节剧本计划 timeline_position 必须从 1 连续覆盖全部叙事单元".into(),
        ));
    }
    Ok(())
}
