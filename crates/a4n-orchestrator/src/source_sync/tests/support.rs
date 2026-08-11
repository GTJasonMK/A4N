use serde_json::json;

use crate::galgame::{
    GalgamePersistLine, GalgamePersistScene, GalgameSceneRegenerationPlan, LINE_TYPE_DIALOGUE,
    SCENE_ENTRY_MAIN,
};

pub(super) fn valid_regenerate_scene_apply_plan() -> GalgameSceneRegenerationPlan {
    GalgameSceneRegenerationPlan {
        scene: GalgamePersistScene {
            scene_index: 7,
            source_snapshot_chapter_id: 1010,
            title: "雨夜重逢".to_string(),
            location: Some("旧车站".to_string()),
            time_of_day: Some("night".to_string()),
            participants_json: json!([{"profile_id": 7, "name": "林晓雨"}]),
            background_note: Some("旧车站的雨夜月台。".to_string()),
            entry_kind: SCENE_ENTRY_MAIN.to_string(),
            next_scene_index: None,
            lines: vec![GalgamePersistLine {
                line_index: 1,
                line_type: LINE_TYPE_DIALOGUE.to_string(),
                speaker_profile_id: Some(7),
                text: "你终于来了。".to_string(),
                expression: Some("smile".to_string()),
                pose: Some("standing".to_string()),
                position: Some("left".to_string()),
                presentation_json: json!({
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
                    "player_knowledge": [],
                    "cg": null,
                    "cg_note": null,
                    "cg_importance": null,
                    "transition": null,
                    "branch": null,
                    "branch_id": null,
                    "returns_to_scene_index": null,
                    "after_beat_ref": null
                }),
                text_anchor_id: Some(9001),
                choices: Vec::new(),
            }],
        },
        stage_outputs_json: json!({
            "shooting_script_projection": {
                "scenes": [{
                    "index": 1,
                    "title": "雨夜重逢",
                    "location": "旧车站",
                    "time_of_day": "night",
                    "participants": ["林晓雨"],
                    "summary": "旧车站的雨夜月台。",
                    "source_span_ids": ["s1"],
                    "shooting_script_id": 99,
                    "shooting_script_revision": 1,
                    "chapter_number": 1,
                    "shooting_scene_index": 1,
                    "beats": [{
                        "shot_index": 1,
                        "beat_index": 1,
                        "coverage": "must_show",
                        "required": true,
                        "objective": "确认对方到来",
                        "obstacle": "双方仍有戒心",
                        "visible_action": "林晓雨开口确认",
                        "reaction": "对方停下脚步",
                        "dialogue_intent": "确认对方是否如约抵达",
                        "audience_question": "重逢会如何发展？",
                        "answer_or_delay": "暂缓回答",
                        "entry_state": "双方隔着雨幕对视",
                        "exit_state": "林晓雨率先打破沉默",
                        "source_span_ids": ["s1"]
                    }]
                }]
            },
            "script_adaptation": {
                "scene_lines": [{
                    "scene_index": 1,
                    "lines": [{
                        "index": 1,
                        "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                        "line_type": "dialogue",
                        "speaker_name": "林晓雨",
                        "text": "你终于来了。",
                        "source_span_ids": ["s1"]
                    }]
                }],
                "branch_scene_lines": []
            },
            "character_staging": {
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
            },
            "visual_planning": {
                "backgrounds": [{
                    "scene_index": 1,
                    "background_note": "旧车站的雨夜月台。",
                    "source_span_ids": ["s1"]
                }],
                "event_cgs": []
            },
            "branch_design": {
                "choices": []
            }
        }),
    }
}
