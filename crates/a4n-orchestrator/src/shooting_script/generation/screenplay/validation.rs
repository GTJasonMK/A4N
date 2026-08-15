use std::collections::HashSet;

use a4n_core::error::AppError;
use a4n_domain::shooting_script::ShootingEditingModeV1;

use crate::shooting_script::contracts::{
    ChapterScreenplaySkeletonV1, PreparedChapterShootingScriptRequest,
    ScreenplayLayoutUnitDetailV1, ScreenplayLayoutUnitV1, ScreenplayUnitAdaptationV1,
};

pub(super) fn validate_skeleton(
    skeleton: &ChapterScreenplaySkeletonV1,
    prepared: &PreparedChapterShootingScriptRequest,
) -> Result<(), AppError> {
    if skeleton.schema_version != "chapter_screenplay_skeleton_v1" || skeleton.units.is_empty() {
        return Err(AppError::InvalidArgument("章节叙事骨架无效或为空".into()));
    }
    let allowed = prepared
        .source_spans
        .iter()
        .map(|span| span.id.as_str())
        .collect::<HashSet<_>>();
    let expected_positions = (1..=skeleton.units.len())
        .map(|position| {
            u32::try_from(position)
                .map_err(|_| AppError::InvalidArgument("叙事骨架单元数量超出 u32 范围".into()))
        })
        .collect::<Result<HashSet<_>, _>>()?;
    let mut timeline_positions = HashSet::new();
    for (offset, unit) in skeleton.units.iter().enumerate() {
        let expected_index = u32::try_from(offset + 1)
            .map_err(|_| AppError::InvalidArgument("叙事骨架单元数量超出 u32 范围".into()))?;
        if unit.narrative_unit_index != expected_index
            || unit.sequence_index == 0
            || unit.timeline_position == 0
            || unit.scene_identity.trim().is_empty()
            || unit.source_span_ids.is_empty()
            || unit
                .source_span_ids
                .iter()
                .any(|id| !allowed.contains(id.as_str()))
        {
            return Err(AppError::InvalidArgument(format!(
                "叙事骨架 unit {} 的索引或来源无效",
                unit.narrative_unit_index
            )));
        }
        if !timeline_positions.insert(unit.timeline_position) {
            return Err(AppError::InvalidArgument(format!(
                "叙事骨架 timeline_position={} 重复",
                unit.timeline_position
            )));
        }
        if matches!(
            unit.editing_mode,
            ShootingEditingModeV1::Intercut | ShootingEditingModeV1::ParallelAction
        ) && unit
            .intercut_group_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(AppError::InvalidArgument(format!(
                "叙事骨架 unit {} 缺少交叉剪辑分组",
                unit.narrative_unit_index
            )));
        }
    }
    if timeline_positions != expected_positions {
        return Err(AppError::InvalidArgument(
            "叙事骨架 timeline_position 必须从 1 连续覆盖全部单元".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_layout_unit_detail(
    output: &ScreenplayLayoutUnitDetailV1,
    expected_index: u32,
    total_unit_count: usize,
) -> Result<(), AppError> {
    if output.schema_version != "screenplay_layout_unit_detail_v1"
        || output.narrative_unit_index != expected_index
    {
        return Err(AppError::InvalidArgument(format!(
            "叙事单元 {expected_index} 的布局细化合同无效"
        )));
    }
    let max_index = u32::try_from(total_unit_count)
        .map_err(|_| AppError::InvalidArgument("叙事布局单元数量超出 u32 范围".into()))?;
    if output.transition_contracts.iter().any(|transition| {
        transition.to_narrative_unit_index == 0
            || transition.to_narrative_unit_index == expected_index
            || transition.to_narrative_unit_index > max_index
    }) {
        return Err(AppError::InvalidArgument(format!(
            "叙事布局 unit {expected_index} 含无效转场合同"
        )));
    }
    Ok(())
}

pub(super) fn validate_unit_adaptation(
    output: &ScreenplayUnitAdaptationV1,
    unit: &ScreenplayLayoutUnitV1,
) -> Result<(), AppError> {
    if output.schema_version != "screenplay_unit_adaptation_v1"
        || output.narrative_unit_index != unit.narrative_unit_index
        || output.performance_facts.is_empty()
    {
        return Err(AppError::InvalidArgument(format!(
            "叙事单元 {} 的独立改编合同无效",
            unit.narrative_unit_index
        )));
    }
    for (offset, fact) in output.performance_facts.iter().enumerate() {
        let expected_fact_index = u32::try_from(offset + 1)
            .map_err(|_| AppError::InvalidArgument("表演事实数量超出 u32 范围".into()))?;
        if fact.fact_index != expected_fact_index {
            return Err(AppError::InvalidArgument(format!(
                "叙事单元 {} fact_index={} 编号不连续",
                unit.narrative_unit_index, fact.fact_index
            )));
        }
        if fact.source_span_ids.is_empty()
            || fact
                .source_span_ids
                .iter()
                .any(|id| !unit.source_span_ids.contains(id))
        {
            return Err(AppError::InvalidArgument(format!(
                "叙事单元 {} fact {} 引用了当前单元范围外的 source span",
                unit.narrative_unit_index, fact.fact_index
            )));
        }
    }
    Ok(())
}
