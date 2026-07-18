use a4n_core::error::AppError;

use super::super::contracts::{
    ChapterScreenplayLayoutV1, ChapterScreenplayPlan, ChapterScreenplaySkeletonV1,
    ScreenplayLayoutUnitDetailV1, ScreenplayLayoutUnitV1, ScreenplayPlanUnit,
    ScreenplayUnitAdaptationV1,
};

pub(in crate::shooting_script) fn assemble_screenplay_layout(
    skeleton: ChapterScreenplaySkeletonV1,
    details: Vec<ScreenplayLayoutUnitDetailV1>,
) -> Result<ChapterScreenplayLayoutV1, AppError> {
    if skeleton.schema_version != "chapter_screenplay_skeleton_v1"
        || skeleton.units.len() != details.len()
        || skeleton.units.is_empty()
    {
        return Err(AppError::InvalidArgument(
            "章节叙事骨架或单元细化数量无效".into(),
        ));
    }
    let units = skeleton
        .units
        .into_iter()
        .zip(details)
        .map(|(unit, detail)| {
            if detail.schema_version != "screenplay_layout_unit_detail_v1"
                || detail.narrative_unit_index != unit.narrative_unit_index
            {
                return Err(AppError::InvalidArgument(format!(
                    "叙事单元 {} 的布局细化合同无效",
                    unit.narrative_unit_index
                )));
            }
            Ok(ScreenplayLayoutUnitV1 {
                scene_identity: unit.scene_identity,
                sequence_index: unit.sequence_index,
                narrative_unit_index: unit.narrative_unit_index,
                timeline_position: unit.timeline_position,
                intercut_group_id: unit.intercut_group_id,
                editing_mode: unit.editing_mode,
                transition_contracts: detail.transition_contracts,
                title: detail.title,
                slugline: detail.slugline,
                source_span_ids: unit.source_span_ids,
                participants: detail.participants,
                dramatic_question: detail.dramatic_question,
                scene_objective: detail.scene_objective,
                scene_obstacle: detail.scene_obstacle,
                entry_state: detail.entry_state,
                exit_state: detail.exit_state,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ChapterScreenplayLayoutV1 {
        schema_version: "chapter_screenplay_layout_v1".into(),
        directing_statement: skeleton.directing_statement,
        units,
    })
}

pub(in crate::shooting_script) fn assemble_screenplay_plan(
    layout: ChapterScreenplayLayoutV1,
    outputs: Vec<ScreenplayUnitAdaptationV1>,
) -> Result<ChapterScreenplayPlan, AppError> {
    if outputs.len() != layout.units.len() {
        return Err(AppError::InvalidArgument(format!(
            "叙事单元改编数量 {} 与布局单元数量 {} 不一致",
            outputs.len(),
            layout.units.len()
        )));
    }
    let scenes = layout
        .units
        .into_iter()
        .zip(outputs)
        .map(|(unit, output)| {
            if output.schema_version != "screenplay_unit_adaptation_v1"
                || output.narrative_unit_index != unit.narrative_unit_index
                || output.performance_facts.is_empty()
            {
                return Err(AppError::InvalidArgument(format!(
                    "叙事单元 {} 的独立改编合同无效",
                    unit.narrative_unit_index
                )));
            }
            Ok(ScreenplayPlanUnit {
                scene_identity: unit.scene_identity,
                sequence_index: unit.sequence_index,
                narrative_unit_index: unit.narrative_unit_index,
                timeline_position: unit.timeline_position,
                intercut_group_id: unit.intercut_group_id,
                editing_mode: unit.editing_mode,
                transition_contracts: unit.transition_contracts,
                title: unit.title,
                slugline: unit.slugline,
                source_span_ids: unit.source_span_ids,
                participants: unit.participants,
                dramatic_question: unit.dramatic_question,
                scene_objective: unit.scene_objective,
                scene_obstacle: unit.scene_obstacle,
                entry_state: unit.entry_state,
                exit_state: unit.exit_state,
                performance_facts: output.performance_facts,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ChapterScreenplayPlan {
        directing_statement: layout.directing_statement,
        units: scenes,
    })
}
