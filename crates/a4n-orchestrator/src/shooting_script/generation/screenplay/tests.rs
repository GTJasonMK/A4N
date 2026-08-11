use a4n_core::source_spans::source_spans_for_content;
use a4n_db::project::{ResolvedChapterSource, ResolvedChapterSourceKind};
use a4n_domain::shooting_script::{
    ShootingDirectingStatementV1, ShootingEditingModeV1, ShootingTransitionContractV1,
};

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
    PreparedChapterShootingScriptRequest {
        source: ResolvedChapterSource {
            source_kind: ResolvedChapterSourceKind::Live,
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
        },
        source_spans: source_spans_for_content(3, 1, &content),
        input_fingerprint_sha256: "fingerprint".into(),
        existing: None,
        skeleton_system_prompt: String::new(),
        layout_unit_system_prompt: String::new(),
        unit_adaptation_system_prompt: String::new(),
        design_unit_system_prompt: String::new(),
    }
}

#[test]
fn skeleton_rejects_duplicate_timeline_positions_before_checkpoint_completion() {
    let prepared = prepared();
    let span_ids = prepared
        .source_spans
        .iter()
        .map(|span| span.id.clone())
        .collect::<Vec<_>>();
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
        units: vec![
            ScreenplaySkeletonUnitV1 {
                scene_identity: "room-a".into(),
                sequence_index: 1,
                narrative_unit_index: 1,
                timeline_position: 1,
                intercut_group_id: None,
                editing_mode: ShootingEditingModeV1::Continuity,
                source_span_ids: span_ids.clone(),
            },
            ScreenplaySkeletonUnitV1 {
                scene_identity: "room-b".into(),
                sequence_index: 2,
                narrative_unit_index: 2,
                timeline_position: 1,
                intercut_group_id: None,
                editing_mode: ShootingEditingModeV1::Continuity,
                source_span_ids: span_ids,
            },
        ],
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
            transition_type: "cut".into(),
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

    let error = validate_unit_adaptation(&output, &layout)
        .expect_err("fact indexes must start at one and remain contiguous");

    assert!(error.to_string().contains("fact 2"));
}

#[test]
fn layout_detail_parser_rejects_string_for_preserved_knowledge_array() {
    let raw = r#"
    {
      "schema_version": "screenplay_layout_unit_detail_v1",
      "narrative_unit_index": 1,
      "transition_contracts": [{
        "to_narrative_unit_index": 2,
        "transition_type": "cut",
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
            source_span_ids: vec!["span-1".into()],
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
