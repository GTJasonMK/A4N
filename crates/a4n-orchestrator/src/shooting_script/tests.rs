use crate::structured_llm::parse_strict_json_response;

use super::{
    contracts::{
        ChapterScreenplayLayoutV1, ChapterScreenplayPlan, ChapterScreenplaySkeletonV1,
        ScreenplayLayoutUnitDetailV1, ScreenplayLayoutUnitV1, ScreenplayPerformanceFactV1,
        ScreenplaySkeletonUnitV1, ScreenplayUnitAdaptationV1,
    },
    screenplay::{
        assemble_screenplay_layout_unit, assemble_screenplay_plan_unit, validate_screenplay_layout,
    },
};

fn directing_statement() -> a4n_core::shooting_script::ShootingDirectingStatementV1 {
    a4n_core::shooting_script::ShootingDirectingStatementV1 {
        central_dramatic_question: "q".into(),
        protagonist_throughline: "p".into(),
        audience_experience: "a".into(),
        viewpoint_strategy: "v".into(),
        visual_strategy: "v".into(),
        spatial_strategy: "s".into(),
        sound_strategy: "s".into(),
        rhythm_strategy: "r".into(),
    }
}

#[test]
fn assembles_layout_without_allowing_unit_details_to_replace_skeleton_topology() {
    let skeleton = ChapterScreenplaySkeletonV1 {
        schema_version: "chapter_screenplay_skeleton_v1".into(),
        directing_statement: directing_statement(),
        units: vec![ScreenplaySkeletonUnitV1 {
            scene_identity: "classroom".into(),
            sequence_index: 1,
            narrative_unit_index: 1,
            timeline_position: 2,
            intercut_group_id: Some("crosscut".into()),
            editing_mode: a4n_core::shooting_script::ShootingEditingModeV1::Intercut,
            source_span_ids: vec!["span-1".into()],
        }],
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

    let unit = assemble_screenplay_layout_unit(&skeleton.units[0], detail)
        .expect("skeleton and detail should assemble");
    let layout = ChapterScreenplayLayoutV1 {
        schema_version: "chapter_screenplay_layout_v1".into(),
        directing_statement: skeleton.directing_statement,
        units: vec![unit],
    };

    assert_eq!(layout.units[0].scene_identity, "classroom");
    assert_eq!(layout.units[0].timeline_position, 2);
    assert_eq!(
        layout.units[0].intercut_group_id.as_deref(),
        Some("crosscut")
    );
    assert_eq!(layout.units[0].title, "回切教室");
}

#[test]
fn strict_parser_rejects_markdown_fence() {
    let error = parse_strict_json_response::<a4n_core::shooting_script::ChapterShootingScriptV1>(
        "```json\n{}\n```",
        "章节共享拍摄剧本",
    )
    .expect_err("markdown fence must fail");

    assert!(error.to_string().contains("Markdown 代码围栏"));
}

#[test]
fn layout_parser_rejects_string_for_preserved_knowledge_array() {
    let raw = r#"
    {
      "schema_version": "chapter_screenplay_layout_v1",
      "directing_statement": {
        "central_dramatic_question": "q",
        "protagonist_throughline": "p",
        "audience_experience": "a",
        "viewpoint_strategy": "v",
        "visual_strategy": "v",
        "spatial_strategy": "s",
        "sound_strategy": "s",
        "rhythm_strategy": "r"
      },
      "units": [{
        "scene_identity": "room",
        "sequence_index": 1,
        "narrative_unit_index": 1,
        "timeline_position": 1,
        "intercut_group_id": null,
        "editing_mode": "continuity",
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
        "source_span_ids": ["span-1"],
        "participants": ["甲"],
        "dramatic_question": "q",
        "scene_objective": "o",
        "scene_obstacle": "o",
        "entry_state": "e",
        "exit_state": "x"
      }]
    }
    "#;

    let error = parse_strict_json_response::<ChapterScreenplayLayoutV1>(raw, "章节叙事布局")
        .expect_err("string must not be accepted for preserved_knowledge");

    assert!(error.to_string().contains("expected a sequence"));
}

#[test]
fn layout_validation_rejects_unknown_schema() {
    let layout = ChapterScreenplayLayoutV1 {
        schema_version: "wrong".into(),
        directing_statement: directing_statement(),
        units: Vec::new(),
    };
    let error =
        validate_screenplay_layout(&layout, &[]).expect_err("invalid layout schema must fail");
    assert!(error.to_string().contains("schema_version"));
}

#[test]
fn assembles_independent_unit_adaptation_without_model_layout_duplication() {
    let layout = ChapterScreenplayLayoutV1 {
        schema_version: "chapter_screenplay_layout_v1".into(),
        directing_statement: directing_statement(),
        units: vec![ScreenplayLayoutUnitV1 {
            scene_identity: "room".into(),
            sequence_index: 1,
            narrative_unit_index: 1,
            timeline_position: 1,
            intercut_group_id: None,
            editing_mode: a4n_core::shooting_script::ShootingEditingModeV1::Continuity,
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
        }],
    };
    let output = ScreenplayUnitAdaptationV1 {
        schema_version: "screenplay_unit_adaptation_v1".into(),
        narrative_unit_index: 1,
        performance_facts: vec![ScreenplayPerformanceFactV1 {
            fact_index: 1,
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
    };

    let unit = assemble_screenplay_plan_unit(&layout.units[0], output)
        .expect("independent unit output should assemble");
    let plan = ChapterScreenplayPlan {
        directing_statement: layout.directing_statement,
        units: vec![unit],
    };

    assert_eq!(plan.units.len(), 1);
    assert_eq!(plan.units[0].scene_identity, "room");
    assert_eq!(plan.units[0].performance_facts.len(), 1);
}
