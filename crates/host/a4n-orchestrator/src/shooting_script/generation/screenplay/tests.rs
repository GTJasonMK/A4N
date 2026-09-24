use a4n_domain::chapter::ResolvedChapterSource;
use a4n_domain::shooting_script::{
    ShootingDirectingStatementV1, ShootingEditingModeV1, ShootingTransitionContractV1,
    ShootingTransitionTypeV1,
};
use a4n_domain::source_spans::source_spans_for_content;

use crate::{
    shooting_script::contracts::{
        ChapterScreenplaySkeletonV1, PreparedChapterShootingScriptRequest,
        ScreenplayLayoutUnitDetailV1, ScreenplayLayoutUnitV1, ScreenplayPerformanceFactV1,
        ScreenplaySkeletonUnitV1, ScreenplayUnitAdaptationV1,
    },
    structured_llm::parse_strict_json_response,
};

use super::{
    assembly::{assemble_screenplay_layout_unit, assemble_screenplay_plan_unit},
    validation::{validate_layout_unit_detail, validate_skeleton, validate_unit_adaptation},
};

fn prepared() -> PreparedChapterShootingScriptRequest {
    let content = "甲进入房间。乙从窗边回头。".to_string();
    let source = ResolvedChapterSource {
        requested_project_id: 1,
        root_project_id: 1,
        source_chapter_id: 3,
        source_snapshot_id: None,
        source_snapshot_chapter_id: None,
        chapter_number: 1,
        title: None,
        summary: None,
        analysis_data_json: None,
        content: content.clone(),
        content_sha256: "hash".into(),
        source_revision_key: "revision".into(),
    };
    let source_spans = source_spans_for_content(3, 1, &content);
    let visual_plan = crate::shooting_script::contracts::visual_plan_fixture(&source);
    let visual_plan_projection =
        crate::chapter_visual_plan::project_visual_plan_for_shooting_script(
            &visual_plan.plan,
            &source_spans,
        )
        .expect("visual plan projection");
    PreparedChapterShootingScriptRequest {
        source,
        source_spans,
        input_fingerprint_sha256: "fingerprint".into(),
        existing: None,
        visual_plan,
        visual_plan_projection,
        scene_profiles: Vec::new(),
        layout_unit_system_prompt: String::new(),
        unit_adaptation_system_prompt: String::new(),
        design_unit_system_prompt: String::new(),
    }
}

#[test]
fn skeleton_rejects_duplicate_timeline_positions_before_checkpoint_completion() {
    let mut prepared = prepared();
    let mut second = prepared.visual_plan_projection.narrative_units[0].clone();
    second.id = "unit_2".into();
    second.sequence_index = 2;
    prepared.visual_plan_projection.narrative_units.push(second);
    let skeleton = ChapterScreenplaySkeletonV1 {
        schema_version: "chapter_screenplay_skeleton_v1".into(),
        directing_statement: ShootingDirectingStatementV1 {
            central_dramatic_question: "问题".into(),
            protagonist_throughline: "主线".into(),
            audience_experience: "体验".into(),
            viewpoint_strategy: "视点".into(),
            visual_strategy: "视觉".into(),
            spatial_strategy: "空间".into(),
            sound_strategy: "声音".into(),
            rhythm_strategy: "节奏".into(),
        },
        units: prepared
            .visual_plan_projection
            .narrative_units
            .iter()
            .enumerate()
            .map(|(offset, unit)| ScreenplaySkeletonUnitV1 {
                scene_identity: unit.id.clone(),
                sequence_index: unit.sequence_index,
                narrative_unit_index: u32::try_from(offset + 1).expect("test unit index"),
                timeline_position: unit.timeline_position,
                intercut_group_id: unit.intercut_group_id.clone(),
                editing_mode: unit.editing_mode,
                source_span_ids: unit.source_span_ids.clone(),
            })
            .collect(),
    };

    let error = validate_skeleton(&skeleton, &prepared)
        .expect_err("duplicate timeline positions must be rejected");

    assert!(error.to_string().contains("timeline_position=1 重复"));
}

#[test]
fn layout_assembly_keeps_skeleton_topology() {
    let skeleton = ScreenplaySkeletonUnitV1 {
        scene_identity: "classroom".into(),
        sequence_index: 1,
        narrative_unit_index: 1,
        timeline_position: 2,
        intercut_group_id: Some("crosscut".into()),
        editing_mode: ShootingEditingModeV1::Intercut,
        source_span_ids: vec!["span-1".into()],
    };
    let detail = ScreenplayLayoutUnitDetailV1 {
        schema_version: "screenplay_layout_unit_detail_v1".into(),
        narrative_unit_index: 1,
        transition_contracts: Vec::new(),
        title: "回切教室".into(),
        slugline: "内景 教室 日".into(),
        participants: vec!["甲".into()],
        dramatic_question: "是否表达".into(),
        scene_objective: "表达".into(),
        scene_obstacle: "犹豫".into(),
        entry_state: "沉默".into(),
        exit_state: "行动".into(),
    };

    let unit = assemble_screenplay_layout_unit(&skeleton, detail);

    assert_eq!(unit.scene_identity, "classroom");
    assert_eq!(unit.timeline_position, 2);
    assert_eq!(unit.intercut_group_id.as_deref(), Some("crosscut"));
    assert_eq!(unit.title, "回切教室");
}

#[test]
fn layout_detail_rejects_transition_outside_skeleton() {
    let output = ScreenplayLayoutUnitDetailV1 {
        schema_version: "screenplay_layout_unit_detail_v1".into(),
        narrative_unit_index: 1,
        transition_contracts: vec![ShootingTransitionContractV1 {
            to_narrative_unit_index: 3,
            transition_type: ShootingTransitionTypeV1::HardCut,
            preserved_action: None,
            preserved_sound: None,
            preserved_knowledge: Vec::new(),
            required_cut_point: None,
        }],
        title: "进入房间".into(),
        slugline: "内景 房间 日".into(),
        participants: vec!["甲".into()],
        dramatic_question: "是否进入".into(),
        scene_objective: "进入".into(),
        scene_obstacle: "门锁".into(),
        entry_state: "门外".into(),
        exit_state: "门内".into(),
    };

    let error = validate_layout_unit_detail(&output, 1, 2)
        .expect_err("transition target outside skeleton must fail");

    assert!(error.to_string().contains("无效转场合同"));
}

#[test]
fn adaptation_assembly_uses_the_single_layout_unit() {
    let layout = layout_unit();
    let output = adaptation(1);

    let unit = assemble_screenplay_plan_unit(&layout, output);

    assert_eq!(unit.scene_identity, "room");
    assert_eq!(unit.performance_facts.len(), 1);
}

#[test]
fn adaptation_rejects_non_contiguous_fact_indexes() {
    let layout = layout_unit();
    let output = adaptation(2);

    let prepared = prepared();
    let error = validate_unit_adaptation(
        &output,
        &layout,
        &prepared.visual_plan_projection.narrative_units[0],
    )
    .expect_err("fact indexes must start at one and remain contiguous");

    assert!(error.to_string().contains("fact_index=2"));
}

#[test]
fn adaptation_accepts_director_facts_without_fabricating_source_spans() {
    let prepared = prepared();
    let visual_unit = &prepared.visual_plan_projection.narrative_units[0];
    let mut output = adaptation(1);
    let fact = &mut output.performance_facts[0];
    fact.source_span_ids.clear();
    fact.user_addition_ids = vec![crate::chapter_visual_plan::segment_addition_id(
        &visual_unit.shots[0].segment_id,
    )];
    assert!(validate_unit_adaptation(&output, &layout_unit(), visual_unit).is_ok());
    output.performance_facts[0].user_addition_ids = vec!["director:missing".into()];
    assert!(validate_unit_adaptation(&output, &layout_unit(), visual_unit).is_err());
    output.performance_facts.clear();
    assert!(validate_unit_adaptation(&output, &layout_unit(), visual_unit).is_ok());
}

#[test]
fn adaptation_requires_each_fact_to_name_its_carrier_shot() {
    let prepared = prepared();
    let visual_unit = &prepared.visual_plan_projection.narrative_units[0];
    let mut output = adaptation(1);
    output.performance_facts[0].shot_id.clear();

    let error = validate_unit_adaptation(&output, &layout_unit(), visual_unit)
        .expect_err("a fact without a carrier shot must be rejected");

    assert!(error.to_string().contains("不存在的承载镜头"));
}

#[test]
fn adaptation_rejects_unusable_facts_before_shooting_design() {
    let prepared = prepared();
    let visual = &prepared.visual_plan_projection.narrative_units[0];
    let layout = layout_unit();
    let mut output = adaptation(1);
    assert!(validate_unit_adaptation(&output, &layout, visual).is_ok());
    output.performance_facts[0].actor = Some("甲与乙".into());
    assert!(
        validate_unit_adaptation(&output, &layout, visual)
            .unwrap_err()
            .to_string()
            .contains("actor")
    );
    output.performance_facts[0].actor = Some("甲".into());
    output.performance_facts[0].visible_action.clear();
    assert!(
        validate_unit_adaptation(&output, &layout, visual)
            .unwrap_err()
            .to_string()
            .contains("visible_action")
    );
}

#[test]
fn layout_detail_parser_rejects_string_for_preserved_knowledge_array() {
    let raw = r#"
    {
      "schema_version": "screenplay_layout_unit_detail_v1",
      "narrative_unit_index": 1,
      "transition_contracts": [{
        "to_narrative_unit_index": 2,
        "transition_type": "hard_cut",
        "preserved_action": null,
        "preserved_sound": null,
        "preserved_knowledge": "她借固定日常维持安全距离",
        "required_cut_point": null
      }],
      "title": "title",
      "slugline": "slugline",
      "participants": ["甲"],
      "dramatic_question": "q",
      "scene_objective": "o",
      "scene_obstacle": "o",
      "entry_state": "e",
      "exit_state": "x"
    }
    "#;

    let error = parse_strict_json_response::<ScreenplayLayoutUnitDetailV1>(raw, "叙事单元布局")
        .expect_err("string must not be accepted for preserved_knowledge");

    assert!(error.to_string().contains("expected a sequence"));
}

fn layout_unit() -> ScreenplayLayoutUnitV1 {
    ScreenplayLayoutUnitV1 {
        scene_identity: "room".into(),
        sequence_index: 1,
        narrative_unit_index: 1,
        timeline_position: 1,
        intercut_group_id: None,
        editing_mode: ShootingEditingModeV1::Continuity,
        transition_contracts: Vec::new(),
        title: "进入房间".into(),
        slugline: "内景 房间 日".into(),
        source_span_ids: vec!["span-1".into()],
        participants: vec!["甲".into()],
        dramatic_question: "是否进入".into(),
        scene_objective: "进入".into(),
        scene_obstacle: "门锁".into(),
        entry_state: "门外".into(),
        exit_state: "门内".into(),
    }
}

fn adaptation(fact_index: u32) -> ScreenplayUnitAdaptationV1 {
    ScreenplayUnitAdaptationV1 {
        schema_version: "screenplay_unit_adaptation_v1".into(),
        narrative_unit_index: 1,
        performance_facts: vec![ScreenplayPerformanceFactV1 {
            fact_index,
            shot_id: "shot-1".into(),
            source_span_ids: vec!["span-1".into()],
            user_addition_ids: Vec::new(),
            actor: Some("甲".into()),
            objective: "开门".into(),
            obstacle: "门锁".into(),
            visible_action: "转动钥匙".into(),
            reaction: None,
            dialogue_intent: None,
            entry_state: "门外".into(),
            exit_state: "门开".into(),
            required: true,
        }],
    }
}
