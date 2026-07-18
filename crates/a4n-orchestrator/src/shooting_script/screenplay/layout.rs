use std::collections::HashSet;

use a4n_core::{error::AppError, source_spans::SourceSpan};

use super::super::contracts::ChapterScreenplayLayoutV1;

pub(in crate::shooting_script) fn validate_screenplay_layout(
    layout: &ChapterScreenplayLayoutV1,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    if layout.schema_version != "chapter_screenplay_layout_v1" || layout.units.is_empty() {
        return Err(AppError::InvalidArgument(
            "章节叙事布局 schema_version 无效或 units 为空".into(),
        ));
    }
    let allowed = source_spans
        .iter()
        .map(|span| span.id.as_str())
        .collect::<HashSet<_>>();
    let known_units = layout
        .units
        .iter()
        .map(|unit| unit.narrative_unit_index)
        .collect::<HashSet<_>>();
    let mut timeline_positions = HashSet::new();
    for (offset, unit) in layout.units.iter().enumerate() {
        let expected_index = u32::try_from(offset + 1)
            .map_err(|_| AppError::InvalidArgument("叙事布局单元数量超出 u32 范围".into()))?;
        if unit.narrative_unit_index != expected_index
            || unit.sequence_index == 0
            || unit.timeline_position == 0
            || unit.scene_identity.trim().is_empty()
            || unit.source_span_ids.is_empty()
        {
            return Err(AppError::InvalidArgument(format!(
                "叙事布局 unit {} 的身份、编号或来源范围无效",
                unit.narrative_unit_index
            )));
        }
        if !timeline_positions.insert(unit.timeline_position) {
            return Err(AppError::InvalidArgument(format!(
                "叙事布局 timeline_position={} 重复",
                unit.timeline_position
            )));
        }
        if matches!(
            unit.editing_mode,
            a4n_core::shooting_script::ShootingEditingModeV1::Intercut
                | a4n_core::shooting_script::ShootingEditingModeV1::ParallelAction
        ) && unit
            .intercut_group_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(AppError::InvalidArgument(format!(
                "叙事布局 unit {} 缺少交叉剪辑分组",
                unit.narrative_unit_index
            )));
        }
        if unit
            .source_span_ids
            .iter()
            .any(|id| !allowed.contains(id.as_str()))
        {
            return Err(AppError::InvalidArgument(format!(
                "叙事布局 unit {} 引用了输入外 source span",
                unit.narrative_unit_index
            )));
        }
        for transition in &unit.transition_contracts {
            if transition.transition_type.trim().is_empty()
                || transition.to_narrative_unit_index == unit.narrative_unit_index
                || !known_units.contains(&transition.to_narrative_unit_index)
            {
                return Err(AppError::InvalidArgument(format!(
                    "叙事布局 unit {} 含无效转场合同",
                    unit.narrative_unit_index
                )));
            }
        }
    }
    let expected_positions = (1..=layout.units.len())
        .map(|position| {
            u32::try_from(position)
                .map_err(|_| AppError::InvalidArgument("叙事布局时间线超出 u32 范围".into()))
        })
        .collect::<Result<HashSet<_>, _>>()?;
    if timeline_positions != expected_positions {
        return Err(AppError::InvalidArgument(
            "叙事布局 timeline_position 必须从 1 连续覆盖全部单元".into(),
        ));
    }
    Ok(())
}
