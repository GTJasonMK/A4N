use std::collections::HashSet;

use a4n_domain::shooting_script::ShootingEditingModeV1;
use a4n_error::AppError;

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
        let planned = prepared
            .visual_plan_projection
            .narrative_units
            .get(offset)
            .ok_or_else(|| AppError::InvalidArgument("视觉编排叙事单元缺失".into()))?;
        if unit.scene_identity != planned.id
            || unit.sequence_index != planned.sequence_index
            || unit.timeline_position != planned.timeline_position
            || unit.editing_mode != planned.editing_mode
            || unit.intercut_group_id != planned.intercut_group_id
            || unit.source_span_ids != planned.source_span_ids
        {
            return Err(AppError::InvalidArgument(format!(
                "叙事骨架 unit {} 偏离 confirmed 导演编排",
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
    visual_unit: &crate::chapter_visual_plan::DirectorScriptUnit,
) -> Result<(), AppError> {
    if output.schema_version != "screenplay_unit_adaptation_v1"
        || output.narrative_unit_index != unit.narrative_unit_index
    {
        return Err(AppError::InvalidArgument(format!(
            "叙事单元 {} 的独立改编合同无效",
            unit.narrative_unit_index
        )));
    }
    let allowed_sources = unit
        .source_span_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let allowed_additions = visual_unit
        .shots
        .iter()
        .map(|shot| crate::chapter_visual_plan::segment_addition_id(&shot.segment_id))
        .collect::<HashSet<_>>();
    let allowed_shot_ids = visual_unit
        .shots
        .iter()
        .map(|shot| shot.segment_id.as_str())
        .collect::<HashSet<_>>();
    for (offset, fact) in output.performance_facts.iter().enumerate() {
        let expected_fact_index = u32::try_from(offset + 1)
            .map_err(|_| AppError::InvalidArgument("表演事实数量超出 u32 范围".into()))?;
        if fact.fact_index != expected_fact_index {
            return Err(AppError::InvalidArgument(format!(
                "叙事单元 {} fact_index={} 编号不连续",
                unit.narrative_unit_index, fact.fact_index
            )));
        }
        if !allowed_shot_ids.contains(fact.shot_id.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "叙事单元 {} fact {} 引用了不存在的承载镜头 {}，允许值为 {:?}",
                unit.narrative_unit_index, fact.fact_index, fact.shot_id, allowed_shot_ids
            )));
        }
        let invalid_additions = fact
            .user_addition_ids
            .iter()
            .filter(|id| !allowed_additions.contains(id.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        if !invalid_additions.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "叙事单元 {} fact {} 包含非法导演事实来源 {:?}，允许值为 {:?}",
                unit.narrative_unit_index, fact.fact_index, invalid_additions, allowed_additions
            )));
        }
        if fact.source_span_ids.is_empty() && fact.user_addition_ids.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "叙事单元 {} fact {} 缺少有效正文或导演事实来源；正文来源只能使用 {:?}，导演事实来源只能使用 {:?}",
                unit.narrative_unit_index, fact.fact_index, allowed_sources, allowed_additions
            )));
        }
        if fact
            .source_span_ids
            .iter()
            .any(|id| !allowed_sources.contains(id.as_str()))
        {
            return Err(AppError::InvalidArgument(format!(
                "叙事单元 {} fact {} 引用了当前单元范围外的 source span",
                unit.narrative_unit_index, fact.fact_index
            )));
        }
    }
    Ok(())
}
