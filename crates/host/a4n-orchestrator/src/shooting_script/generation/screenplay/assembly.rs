use crate::shooting_script::contracts::{
    ScreenplayLayoutUnitDetailV1, ScreenplayLayoutUnitV1, ScreenplayPlanUnit,
    ScreenplaySkeletonUnitV1, ScreenplayUnitAdaptationV1,
};

pub(in crate::shooting_script::generation) fn assemble_screenplay_layout_unit(
    unit: &ScreenplaySkeletonUnitV1,
    detail: ScreenplayLayoutUnitDetailV1,
) -> ScreenplayLayoutUnitV1 {
    ScreenplayLayoutUnitV1 {
        scene_identity: unit.scene_identity.clone(),
        sequence_index: unit.sequence_index,
        narrative_unit_index: unit.narrative_unit_index,
        timeline_position: unit.timeline_position,
        intercut_group_id: unit.intercut_group_id.clone(),
        editing_mode: unit.editing_mode,
        transition_contracts: detail.transition_contracts,
        title: detail.title,
        slugline: detail.slugline,
        source_span_ids: unit.source_span_ids.clone(),
        participants: detail.participants,
        dramatic_question: detail.dramatic_question,
        scene_objective: detail.scene_objective,
        scene_obstacle: detail.scene_obstacle,
        entry_state: detail.entry_state,
        exit_state: detail.exit_state,
    }
}

pub(in crate::shooting_script::generation) fn assemble_screenplay_plan_unit(
    unit: &ScreenplayLayoutUnitV1,
    output: ScreenplayUnitAdaptationV1,
) -> ScreenplayPlanUnit {
    ScreenplayPlanUnit {
        scene_identity: unit.scene_identity.clone(),
        sequence_index: unit.sequence_index,
        narrative_unit_index: unit.narrative_unit_index,
        timeline_position: unit.timeline_position,
        intercut_group_id: unit.intercut_group_id.clone(),
        editing_mode: unit.editing_mode,
        transition_contracts: unit.transition_contracts.clone(),
        title: unit.title.clone(),
        slugline: unit.slugline.clone(),
        source_span_ids: unit.source_span_ids.clone(),
        participants: unit.participants.clone(),
        dramatic_question: unit.dramatic_question.clone(),
        scene_objective: unit.scene_objective.clone(),
        scene_obstacle: unit.scene_obstacle.clone(),
        entry_state: unit.entry_state.clone(),
        exit_state: unit.exit_state.clone(),
        performance_facts: output.performance_facts,
    }
}
