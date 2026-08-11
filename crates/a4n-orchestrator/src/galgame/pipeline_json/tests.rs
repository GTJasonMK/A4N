use super::*;
use a4n_domain::galgame::{
    GalgameFacingV2, GalgameLinePresentationSchemaVersion, GalgameStageLayerV2,
    GalgameStagePositionV2, GalgameVisibleCastMemberV2,
};
use serde_json::{Value, json};

fn directing_beat_json(beat_index: u32, source_span_id: &str) -> Value {
    json!({
        "beat_index": beat_index,
        "shot_index": 1,
        "required": true,
        "coverage": "must_show",
        "objective": "推进当前戏剧目标",
        "obstacle": "信息与行动受阻",
        "visible_action": "角色做出可见行动",
        "reaction": "对方产生可见反应",
        "audience_question": "行动会带来什么结果？",
        "answer_or_delay": "暂时延迟答案",
        "dialogue_intent": null,
        "entry_state": "动作开始",
        "exit_state": "动作完成",
        "source_span_ids": [source_span_id]
    })
}

fn directing_scene_json(index: i32, source_span_id: &str) -> Value {
    json!({
        "index": index,
        "title": "雨夜重逢",
        "location": "车站",
        "time_of_day": "夜晚",
        "participants": ["林晓雨", "林晓雨"],
        "summary": "两人重逢",
        "source_span_ids": [source_span_id, source_span_id],
        "shooting_script_id": 10,
        "shooting_script_revision": 1,
        "chapter_number": 1,
        "shooting_scene_index": index,
        "beats": [directing_beat_json(1, source_span_id)]
    })
}

#[test]
fn parses_shooting_script_projection_contract() {
    let parsed = shooting_script_projection_from_value(
        json!({
            "scenes": [directing_scene_json(1, "ch1:p1")]
        }),
        "galgame_shooting_script_projection",
    )
    .expect("directing projection");

    assert_eq!(parsed.scenes.len(), 1);
    assert_eq!(parsed.scenes[0].participants, vec!["林晓雨"]);
    assert_eq!(parsed.scenes[0].source_span_ids, vec!["ch1:p1"]);
}

#[test]
fn rejects_shooting_script_projection_with_gap_indexes() {
    let err = shooting_script_projection_from_value(
        json!({
            "scenes": [directing_scene_json(2, "s1")]
        }),
        "galgame_shooting_script_projection",
    )
    .expect_err("gap indexes");

    assert!(err.to_string().contains("从 1 开始连续递增"));
}

#[test]
fn rejects_shooting_script_projection_without_plan_identity() {
    let mut scene = directing_scene_json(1, "s1");
    let scene = scene.as_object_mut().expect("scene object");
    scene.remove("shooting_script_id");
    scene.remove("shooting_script_revision");
    let err = shooting_script_projection_from_value(
        json!({"scenes": [scene]}),
        "galgame_shooting_script_projection",
    )
    .expect_err("v2 directing projection must not accept legacy defaults");

    assert!(err.to_string().contains("shooting_script_id"));
}

#[test]
fn rejects_shooting_script_projection_without_beats() {
    let mut scene = directing_scene_json(1, "s1");
    scene["beats"] = json!([]);
    let err = shooting_script_projection_from_value(
        json!({"scenes": [scene]}),
        "galgame_shooting_script_projection",
    )
    .expect_err("shooting script projection must preserve the performance beat ledger");

    assert!(err.to_string().contains("beats 不能为空"));
}

#[test]
fn rejects_shooting_script_projection_with_gap_beat_indexes() {
    let mut scene = directing_scene_json(1, "s1");
    scene["beats"] = json!([directing_beat_json(1, "s1"), directing_beat_json(3, "s1")]);
    let err = shooting_script_projection_from_value(
        json!({"scenes": [scene]}),
        "galgame_shooting_script_projection",
    )
    .expect_err("shooting beat refs must remain stable and contiguous");

    assert!(
        err.to_string()
            .contains("必须按 shot_index、beat_index 连续排列")
    );
}

#[test]
fn rejects_shooting_script_projection_with_cross_scene_beat_span() {
    let mut scene = directing_scene_json(1, "s1");
    scene["beats"] = json!([directing_beat_json(1, "s2")]);
    let err = shooting_script_projection_from_value(
        json!({"scenes": [scene]}),
        "galgame_shooting_script_projection",
    )
    .expect_err("directing beat evidence must belong to its scene");

    assert!(err.to_string().contains("不属于所在场景"));
}

#[test]
fn rejects_shooting_script_projection_without_participants_field() {
    let mut scene = directing_scene_json(1, "s1");
    scene
        .as_object_mut()
        .expect("scene object")
        .remove("participants");
    let err = shooting_script_projection_from_value(
        json!({"scenes": [scene]}),
        "galgame_shooting_script_projection",
    )
    .expect_err("v2 projection must preserve an explicit participants array");

    assert!(err.to_string().contains("participants"));
}

#[test]
fn parses_script_adaptation_contract() {
    let parsed = script_adaptation_from_value(
        json!({
            "scene_lines": [
                {
                    "scene_index": 1,
                    "lines": [
                        {
                            "index": 1,
                            "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                            "line_type": "dialogue",
                            "speaker_name": "林晓雨",
                            "text": "你终于来了。",
                            "source_span_ids": ["s1"]
                        },
                        {
                            "index": 2,
                            "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 2},
                            "line_type": "narration",
                            "speaker_name": "",
                            "text": "雨声压过了脚步声。",
                            "source_span_ids": ["s1"]
                        }
                    ]
                }
            ],
            "branch_scene_lines": []
        }),
        "galgame_script_adaptation",
    )
    .expect("script adaptation");

    assert_eq!(parsed.scene_lines[0].lines.len(), 2);
    assert_eq!(parsed.scene_lines[0].lines[1].speaker_name, None);
}

#[test]
fn rejects_script_adaptation_without_branch_scene_lines_field() {
    let err = script_adaptation_from_value(json!({"scene_lines": []}), "galgame_script_adaptation")
        .expect_err("v2 script adaptation must explicitly declare branch_scene_lines");

    assert!(err.to_string().contains("branch_scene_lines"));
}

#[test]
fn rejects_script_adaptation_dialogue_without_speaker() {
    let err = script_adaptation_from_value(
        json!({
            "scene_lines": [{
                "scene_index": 1,
                "lines": [{
                    "index": 1,
                    "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                    "line_type": "dialogue",
                    "speaker_name": "",
                    "text": "没有说话人",
                    "source_span_ids": ["s1"]
                }]
            }],
            "branch_scene_lines": []
        }),
        "galgame_script_adaptation",
    )
    .expect_err("dialogue speaker");

    assert!(err.to_string().contains("speaker_name"));
}

#[test]
fn rejects_script_adaptation_line_without_source_span() {
    let err = script_adaptation_from_value(
        json!({
            "scene_lines": [{
                "scene_index": 1,
                "lines": [{
                    "index": 1,
                    "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                    "line_type": "narration",
                    "speaker_name": "",
                    "text": "雨声压过了脚步声。",
                    "source_span_ids": []
                }]
            }],
            "branch_scene_lines": []
        }),
        "galgame_script_adaptation",
    )
    .expect_err("script adaptation lines must keep source evidence");

    assert!(err.to_string().contains("source_span_ids 不能为空"));
}

#[test]
fn parses_character_staging_contract() {
    let parsed = character_staging_from_value(
        json!({
            "scene_lines": [{
                "scene_index": 1,
                "lines": [{
                    "index": 1,
                    "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                    "line_type": "dialogue",
                    "speaker_profile_id": 7,
                    "speaker_name": "林晓雨",
                    "text": "你终于来了。",
                    "presentation": {
                        "schema_version": "galgame_line_presentation_v2",
                        "visible_cast": [{
                            "character_profile_id": 7,
                            "expression": "smile",
                            "pose": "standing",
                            "position": "left",
                            "facing": "front",
                            "layer": "front"
                        }],
                        "enter": [],
                        "exit": [],
                        "focus_character_profile_id": 7,
                        "hold": false,
                        "player_knowledge": []
                    },
                    "source_span_ids": ["s1"]
                }]
            }],
            "branch_scene_lines": []
        }),
        "galgame_character_staging",
    )
    .expect("character staging");

    assert_eq!(parsed.scene_lines[0].lines[0].speaker_profile_id, Some(7));
    assert_eq!(
        parsed.scene_lines[0].lines[0].presentation.visible_cast[0]
            .expression
            .as_str(),
        "smile"
    );
}

#[test]
fn validates_exit_against_post_line_visible_cast() {
    let mut presentation = GalgameLinePresentationV2 {
        schema_version: GalgameLinePresentationSchemaVersion::V2,
        visible_cast: vec![GalgameVisibleCastMemberV2 {
            character_profile_id: 7,
            expression: "neutral".to_string(),
            pose: "standing".to_string(),
            position: GalgameStagePositionV2::Left,
            facing: GalgameFacingV2::Front,
            layer: GalgameStageLayerV2::Front,
        }],
        enter: Vec::new(),
        exit: vec![8],
        focus_character_profile_id: Some(7),
        hold: false,
        player_knowledge: Vec::new(),
    };

    validate_staged_presentation(&presentation, Some(7), "test").expect("character 8 exited");

    for invalid_exit in [vec![7], vec![0], vec![8, 8]] {
        presentation.exit = invalid_exit;
        assert!(validate_staged_presentation(&presentation, Some(7), "test").is_err());
    }
}

#[test]
fn rejects_character_staging_without_branch_scene_lines_field() {
    let err = character_staging_from_value(json!({"scene_lines": []}), "galgame_character_staging")
        .expect_err("v2 character staging must explicitly declare branch_scene_lines");

    assert!(err.to_string().contains("branch_scene_lines"));
}

#[test]
fn rejects_character_staging_invalid_expression() {
    let err = character_staging_from_value(
        json!({
            "scene_lines": [{
                "scene_index": 1,
                "lines": [{
                    "index": 1,
                    "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                    "line_type": "dialogue",
                    "speaker_profile_id": 7,
                    "speaker_name": "林晓雨",
                    "text": "你终于来了。",
                    "presentation": {
                        "schema_version": "galgame_line_presentation_v2",
                        "visible_cast": [{
                            "character_profile_id": 7,
                            "expression": "ecstatic",
                            "pose": "standing",
                            "position": "left",
                            "facing": "front",
                            "layer": "front"
                        }],
                        "enter": [],
                        "exit": [],
                        "focus_character_profile_id": 7,
                        "hold": false,
                        "player_knowledge": []
                    },
                    "source_span_ids": ["s1"]
                }]
            }],
            "branch_scene_lines": []
        }),
        "galgame_character_staging",
    )
    .expect_err("invalid expression");

    assert!(err.to_string().contains("expression 无效"));
}

#[test]
fn rejects_character_staging_bound_dialogue_without_speaker_name() {
    let err = character_staging_from_value(
        json!({
            "scene_lines": [{
                "scene_index": 1,
                "lines": [{
                    "index": 1,
                    "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                    "line_type": "dialogue",
                    "speaker_profile_id": 7,
                    "text": "你终于来了。",
                    "presentation": {
                        "schema_version": "galgame_line_presentation_v2",
                        "visible_cast": [{
                            "character_profile_id": 7,
                            "expression": "neutral",
                            "pose": "standing",
                            "position": "left",
                            "facing": "front",
                            "layer": "front"
                        }],
                        "enter": [],
                        "exit": [],
                        "focus_character_profile_id": 7,
                        "hold": false,
                        "player_knowledge": []
                    },
                    "source_span_ids": ["s1"]
                }]
            }],
            "branch_scene_lines": []
        }),
        "galgame_character_staging",
    )
    .expect_err("staging must preserve the script speaker name");

    assert!(err.to_string().contains("speaker_name 不能为空"));
}

#[test]
fn rejects_character_staging_bound_character_without_expression() {
    let err = character_staging_from_value(
        json!({
            "scene_lines": [{
                "scene_index": 1,
                "lines": [{
                    "index": 1,
                    "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                    "line_type": "dialogue",
                    "speaker_profile_id": 7,
                    "speaker_name": "林晓雨",
                    "text": "你终于来了。",
                    "presentation": {
                        "schema_version": "galgame_line_presentation_v2",
                        "visible_cast": [{
                            "character_profile_id": 7,
                            "pose": "standing",
                            "position": "left",
                            "facing": "front",
                            "layer": "front"
                        }],
                        "enter": [],
                        "exit": [],
                        "focus_character_profile_id": 7,
                        "hold": false,
                        "player_knowledge": []
                    },
                    "source_span_ids": ["s1"]
                }]
            }],
            "branch_scene_lines": []
        }),
        "galgame_character_staging",
    )
    .expect_err("bound character line should require expression");

    assert!(err.to_string().contains("missing field `expression`"));
}

#[test]
fn rejects_character_staging_line_without_source_span() {
    let err = character_staging_from_value(
        json!({
            "scene_lines": [{
                "scene_index": 1,
                "lines": [{
                    "index": 1,
                    "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                    "line_type": "narration",
                    "speaker_profile_id": null,
                    "speaker_name": "",
                    "text": "雨声压过了脚步声。",
                    "presentation": {
                        "schema_version": "galgame_line_presentation_v2",
                        "visible_cast": [],
                        "enter": [],
                        "exit": [],
                        "focus_character_profile_id": null,
                        "hold": false,
                        "player_knowledge": []
                    },
                    "source_span_ids": []
                }]
            }],
            "branch_scene_lines": []
        }),
        "galgame_character_staging",
    )
    .expect_err("staged lines must keep source evidence");

    assert!(err.to_string().contains("source_span_ids 不能为空"));
}

#[test]
fn parses_visual_plan_contract() {
    let parsed = visual_plan_from_value(
        json!({
            "backgrounds": [{
                "scene_index": 1,
                "background_note": "雨夜车站，冷色灯光",
                "scene_profile_id": null,
                "source_span_ids": ["s1"]
            }],
            "event_cgs": [{
                "scene_index": 1,
                "line_index": 2,
                "cg_note": "两人在雨中对视",
                "importance": "high",
                "source_span_ids": ["s2"]
            }]
        }),
        "galgame_visual_planning",
    )
    .expect("visual plan");

    assert_eq!(parsed.backgrounds.len(), 1);
    assert_eq!(
        parsed.event_cgs[0].importance,
        super::super::GalgameCgImportance::High
    );
}

#[test]
fn rejects_visual_plan_without_event_cgs_field() {
    let err = visual_plan_from_value(json!({"backgrounds": []}), "galgame_visual_planning")
        .expect_err("v2 visual planning must explicitly declare event_cgs");

    assert!(err.to_string().contains("event_cgs"));
}

#[test]
fn rejects_branch_design_without_choices_field() {
    let err = branch_design_from_value(json!({}), "galgame_branch_design")
        .expect_err("v2 branch design must explicitly declare choices");

    assert!(err.to_string().contains("choices"));
}

#[test]
fn rejects_branch_design_with_single_option() {
    let err = branch_design_from_value(
        json!({
            "choices": [{
                "after_beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 3},
                "question": "怎么回应？",
                "options": [{
                    "branch_id": "silent",
                    "option_text": "沉默",
                    "branch_title": "沉默",
                    "branch_objective": "避免暴露态度",
                    "dramatic_action": "角色移开视线",
                    "reaction": "对方停顿观察",
                    "player_knowledge": ["对方注意到沉默"],
                    "returns_to_scene_index": 2
                }]
            }]
        }),
        "galgame_branch_design",
    )
    .expect_err("option count");

    assert!(err.to_string().contains("2 到 3"));
}

#[test]
fn rejects_branch_design_option_without_return_scene() {
    let err = branch_design_from_value(
        json!({
            "choices": [{
                "after_beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 3},
                "question": "怎么回应？",
                "options": [
                    {
                        "branch_id": "silent",
                        "option_text": "沉默",
                        "branch_title": "沉默",
                        "branch_objective": "避免暴露态度",
                        "dramatic_action": "角色移开视线",
                        "reaction": "对方停顿观察",
                        "player_knowledge": ["对方注意到沉默"]
                    },
                    {
                        "branch_id": "ask",
                        "option_text": "追问",
                        "branch_title": "追问",
                        "branch_objective": "确认信息",
                        "dramatic_action": "角色向前一步追问",
                        "reaction": "对方下意识后退",
                        "player_knowledge": ["对方在回避问题"],
                        "returns_to_scene_index": 2
                    }
                ]
            }]
        }),
        "galgame_branch_design",
    )
    .expect_err("missing returns_to_scene_index");

    assert!(
        err.to_string()
            .contains("missing field `returns_to_scene_index`")
    );
}
