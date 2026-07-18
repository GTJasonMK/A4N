use super::*;

#[test]
fn rejects_branch_design_choice_after_missing_line() {
    let scenes = shooting_script_projection_from_value(
        json!({
            "scenes": [
                directing_scene_value(1, "分岔口", "s1"),
                directing_scene_value(2, "回收点", "s2")
            ]
        }),
        "galgame_shooting_script_projection",
    )
    .expect("scenes");
    let staging = character_staging_from_value(
        json!({
            "scene_lines": [
                {
                    "scene_index": 1,
                    "lines": [{
                        "index": 1,
                        "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                        "line_type": "narration",
                        "text": "路灯闪了一下。",
                        "presentation": narration_presentation(),
                        "source_span_ids": ["s1"]
                    }]
                },
                {
                    "scene_index": 2,
                    "lines": [{
                        "index": 1,
                        "beat_ref": {"scene_index": 2, "shot_index": 1, "beat_index": 1},
                        "line_type": "narration",
                        "text": "两人重新会合。",
                        "presentation": narration_presentation(),
                        "source_span_ids": ["s2"]
                    }]
                }
            ],
            "branch_scene_lines": []
        }),
        "galgame_character_staging",
    )
    .expect("staging");
    let branch = branch_design_from_value(
        json!({
            "choices": [{
                "after_beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 99},
                "question": "要走哪条路？",
                "options": [
                    {
                        "branch_id": "left_lane",
                        "option_text": "走左侧小巷",
                        "branch_title": "左侧小巷",
                        "branch_objective": "调查小巷",
                        "dramatic_action": "主角走入小巷查看痕迹",
                        "reaction": "同伴警惕地守住入口",
                        "player_knowledge": ["小巷有新鲜脚印"],
                        "returns_to_scene_index": 2
                    },
                    {
                        "branch_id": "wait_here",
                        "option_text": "留在原地",
                        "branch_title": "原地等待",
                        "branch_objective": "观察环境",
                        "dramatic_action": "主角停在路灯下观察",
                        "reaction": "同伴放慢脚步返回",
                        "player_knowledge": ["路灯闪烁有规律"],
                        "returns_to_scene_index": 2
                    }
                ]
            }]
        }),
        "galgame_branch_design",
    )
    .expect("branch");
    let source_spans = vec![
        GalgamePipelineSourceSpan {
            id: "s1".to_string(),
            text_anchor_id: 100,
            source_snapshot_chapter_id: 20,
        },
        GalgamePipelineSourceSpan {
            id: "s2".to_string(),
            text_anchor_id: 101,
            source_snapshot_chapter_id: 21,
        },
    ];

    let err = build_galgame_script_persist_plan(
        &scenes,
        &staging,
        &two_scene_visual_plan(),
        &branch,
        &source_spans,
    )
    .expect_err("branch choice after_beat_ref must reference an existing directing beat");

    assert!(err.to_string().contains("after_beat_ref 不存在"));
}

#[test]
fn inserts_choice_line_from_branch_design() {
    let scenes = shooting_script_projection_from_value(
        json!({
            "scenes": [
                directing_scene_value(1, "分岔口", "s1"),
                directing_scene_value(2, "回收点", "s2")
            ]
        }),
        "galgame_shooting_script_projection",
    )
    .expect("scenes");
    let staging = character_staging_from_value(
        json!({
            "scene_lines": [
                {
                    "scene_index": 1,
                    "lines": [{
                        "index": 1,
                        "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                        "line_type": "narration",
                        "text": "路灯闪了一下。",
                        "presentation": narration_presentation(),
                        "source_span_ids": ["s1"]
                    }]
                },
                {
                    "scene_index": 2,
                    "lines": [{
                        "index": 1,
                        "beat_ref": {"scene_index": 2, "shot_index": 1, "beat_index": 1},
                        "line_type": "narration",
                        "text": "两人重新会合。",
                        "presentation": narration_presentation(),
                        "source_span_ids": ["s2"]
                    }]
                }
            ],
            "branch_scene_lines": [
                {
                    "branch_id": "left_lane",
                    "lines": [{
                        "index": 1,
                        "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                        "line_type": "stage_direction",
                        "text": "主角走入小巷查看痕迹，同伴警惕地守住入口。",
                        "presentation": narration_presentation(),
                        "source_span_ids": ["s1"]
                    }]
                },
                {
                    "branch_id": "wait_here",
                    "lines": [{
                        "index": 1,
                        "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                        "line_type": "stage_direction",
                        "text": "主角停在路灯下观察，同伴放慢脚步返回。",
                        "presentation": narration_presentation(),
                        "source_span_ids": ["s1"]
                    }]
                }
            ]
        }),
        "galgame_character_staging",
    )
    .expect("staging");
    let branch = branch_design_from_value(
        json!({
            "choices": [{
                "after_beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                "question": "要走哪条路？",
                "options": [
                    {
                        "branch_id": "left_lane",
                        "option_text": "走左侧小巷",
                        "branch_title": "左侧小巷",
                        "branch_objective": "调查小巷",
                        "dramatic_action": "主角走入小巷查看痕迹",
                        "reaction": "同伴警惕地守住入口",
                        "player_knowledge": ["小巷有新鲜脚印"],
                        "returns_to_scene_index": 2
                    },
                    {
                        "branch_id": "wait_here",
                        "option_text": "留在原地",
                        "branch_title": "原地等待",
                        "branch_objective": "观察环境",
                        "dramatic_action": "主角停在路灯下观察",
                        "reaction": "同伴放慢脚步返回",
                        "player_knowledge": ["路灯闪烁有规律"],
                        "returns_to_scene_index": 2
                    }
                ]
            }]
        }),
        "galgame_branch_design",
    )
    .expect("branch");
    let source_spans = vec![
        GalgamePipelineSourceSpan {
            id: "s1".to_string(),
            text_anchor_id: 100,
            source_snapshot_chapter_id: 20,
        },
        GalgamePipelineSourceSpan {
            id: "s2".to_string(),
            text_anchor_id: 101,
            source_snapshot_chapter_id: 21,
        },
    ];

    let plan = build_galgame_script_persist_plan(
        &scenes,
        &staging,
        &two_scene_visual_plan(),
        &branch,
        &source_spans,
    )
    .expect("persist plan");

    assert_eq!(plan.scenes.len(), 4);
    assert_eq!(plan.scenes[0].next_scene_index, Some(2));
    assert_eq!(plan.scenes[1].next_scene_index, None);

    let choice_line = &plan.scenes[0].lines[1];
    assert_eq!(choice_line.line_index, 2);
    assert_eq!(choice_line.line_type, LINE_TYPE_CHOICE);
    assert_eq!(choice_line.text, "要走哪条路？");
    assert_eq!(choice_line.choices.len(), 2);
    assert_eq!(choice_line.choices[0].option_index, 1);
    assert_eq!(choice_line.choices[0].target_scene_index, 3);
    assert_eq!(choice_line.choices[1].target_scene_index, 4);

    assert_eq!(plan.scenes[2].scene_index, 3);
    assert_eq!(plan.scenes[2].entry_kind, SCENE_ENTRY_BRANCH);
    assert_eq!(plan.scenes[2].title, "左侧小巷");
    assert_eq!(plan.scenes[2].next_scene_index, Some(2));
    assert_eq!(
        plan.scenes[2].lines[0].text,
        "主角走入小巷查看痕迹，同伴警惕地守住入口。"
    );
}
