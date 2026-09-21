use super::*;
use a4n_domain::shooting_script::ShootingCoverageV1;
use serde_json::Value;

pub(super) fn single_shooting_script_projection() -> GalgameShootingScriptProjection {
    shooting_script_projection_from_value(
        json!({
            "scenes": [directing_scene_value(1, "雨夜重逢", "s1")]
        }),
        "galgame_shooting_script_projection",
    )
    .expect("single directing projection")
}

pub(super) fn directing_scene_value(index: i32, title: &str, source_span_id: &str) -> Value {
    directing_scene_value_with_beats(
        index,
        title,
        source_span_id,
        vec![directing_beat(1, source_span_id)],
    )
}

pub(super) fn directing_scene_value_with_beats(
    index: i32,
    title: &str,
    source_span_id: &str,
    beats: Vec<GalgamePipelineShootingBeat>,
) -> Value {
    json!({
        "index": index,
        "title": title,
        "source_span_ids": [source_span_id],
        "shooting_script_id": 10,
        "shooting_script_revision": 1,
        "chapter_number": 1,
        "shooting_scene_index": index,
        "participants": [],
        "beats": beats
    })
}

pub(super) fn single_source_span() -> Vec<GalgamePipelineSourceSpan> {
    vec![GalgamePipelineSourceSpan {
        id: "s1".to_string(),
        text_anchor_id: 100,
        source_snapshot_chapter_id: 20,
    }]
}

pub(super) fn empty_branch_design() -> GalgameBranchDesign {
    branch_design_from_value(json!({"choices": []}), "galgame_branch_design")
        .expect("empty branch design")
}

pub(super) fn directing_beat(beat_index: u32, source_span_id: &str) -> GalgamePipelineShootingBeat {
    GalgamePipelineShootingBeat {
        shot_index: 1,
        beat_index,
        coverage: ShootingCoverageV1::MustShow,
        required: true,
        objective: "推进当前戏剧目标".to_string(),
        obstacle: "信息与行动受阻".to_string(),
        visible_action: "角色做出可见行动".to_string(),
        reaction: Some("对方产生可见反应".to_string()),
        dialogue_intent: None,
        audience_question: "行动会带来什么结果？".to_string(),
        answer_or_delay: "暂时延迟答案".to_string(),
        entry_state: "动作开始".to_string(),
        exit_state: "动作完成".to_string(),
        source_span_ids: vec![source_span_id.to_string()],
    }
}

pub(super) fn narration_presentation() -> GalgameLinePresentationV2 {
    GalgameLinePresentationV2 {
        schema_version: GalgameLinePresentationSchemaVersion::V2,
        visible_cast: Vec::new(),
        enter: Vec::new(),
        exit: Vec::new(),
        focus_character_profile_id: None,
        hold: false,
        player_knowledge: Vec::new(),
    }
}

pub(super) fn speaker_presentation(profile_id: i64, expression: &str) -> GalgameLinePresentationV2 {
    GalgameLinePresentationV2 {
        schema_version: GalgameLinePresentationSchemaVersion::V2,
        visible_cast: vec![GalgameVisibleCastMemberV2 {
            character_profile_id: profile_id,
            expression: expression.to_string(),
            pose: "standing".to_string(),
            position: GalgameStagePositionV2::Left,
            facing: GalgameFacingV2::Front,
            layer: GalgameStageLayerV2::Front,
        }],
        enter: Vec::new(),
        exit: Vec::new(),
        focus_character_profile_id: Some(profile_id),
        hold: false,
        player_knowledge: Vec::new(),
    }
}

pub(super) fn single_narration_staging() -> GalgameCharacterStaging {
    character_staging_from_value(
        json!({
            "scene_lines": [{
                "scene_index": 1,
                "lines": [{
                    "index": 1,
                    "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                    "line_type": "narration",
                    "text": "雨声压过脚步。",
                    "presentation": {
                        "schema_version": "galgame_line_presentation_v2",
                        "visible_cast": [],
                        "enter": [],
                        "exit": [],
                        "focus_character_profile_id": null,
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
    .expect("single narration staging")
}

pub(super) fn single_visual_plan() -> GalgameVisualPlan {
    visual_plan_for_scene(1, "s1")
}

pub(super) fn visual_plan_for_scene(scene_index: i32, source_span_id: &str) -> GalgameVisualPlan {
    visual_plan_from_value(
        json!({
            "backgrounds": [{
                "scene_index": scene_index,
                "background_note": format!("场 {scene_index} 背景"),
                "source_span_ids": [source_span_id]
            }],
            "event_cgs": []
        }),
        "galgame_visual_planning",
    )
    .expect("visual plan")
}

pub(super) fn two_scene_visual_plan() -> GalgameVisualPlan {
    visual_plan_from_value(
        json!({
            "backgrounds": [
                {
                    "scene_index": 1,
                    "background_note": "分岔口背景",
                    "source_span_ids": ["s1"]
                },
                {
                    "scene_index": 2,
                    "background_note": "回收点背景",
                    "source_span_ids": ["s2"]
                }
            ],
            "event_cgs": []
        }),
        "galgame_visual_planning",
    )
    .expect("two scene visual plan")
}
