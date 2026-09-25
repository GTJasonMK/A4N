use super::*;

#[test]
fn rejects_visual_plan_missing_scene_background() {
    let scenes = shooting_script_projection_from_value(
        json!({
            "scenes": [
                directing_scene_value(1, "雨夜重逢", "s1"),
                directing_scene_value(2, "站台尽头", "s2")
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
                        "text": "雨声压过脚步。",
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
                        "text": "站台尽头亮起灯。",
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
    let visual_plan = visual_plan_from_value(
        json!({
            "backgrounds": [{
                "scene_index": 1,
                "background_note": "雨夜车站",
                "source_span_ids": ["s1"]
            }],
            "event_cgs": []
        }),
        "galgame_visual_planning",
    )
    .expect("visual plan");
    let source_spans = vec![
        GalgamePipelineSourceSpan {
            id: "s1".to_string(),
            text_anchor_id: 100,
            source_snapshot_chapter_id: 20,
            chapter_number: 1,
        },
        GalgamePipelineSourceSpan {
            id: "s2".to_string(),
            text_anchor_id: 101,
            source_snapshot_chapter_id: 20,
            chapter_number: 1,
        },
    ];

    let err = build_galgame_script_persist_plan(
        &scenes,
        &staging,
        &visual_plan,
        &empty_branch_design(),
        &source_spans,
    )
    .expect_err("missing background must fail visibly");

    assert!(err.to_string().contains("缺少 scene_index=2 的 background"));
}

#[test]
fn rejects_visual_plan_event_cg_for_missing_line() {
    let scenes = single_shooting_script_projection();
    let staging = single_narration_staging();
    let visual_plan = visual_plan_from_value(
        json!({
            "backgrounds": [{
                "scene_index": 1,
                "background_note": "雨夜车站",
                "source_span_ids": ["s1"]
            }],
            "event_cgs": [{
                "scene_index": 1,
                "line_index": 2,
                "cg_note": "不存在的高潮行",
                "importance": "high",
                "source_span_ids": ["s1"]
            }]
        }),
        "galgame_visual_planning",
    )
    .expect("visual plan");
    let source_spans = single_source_span();

    let err = build_galgame_script_persist_plan(
        &scenes,
        &staging,
        &visual_plan,
        &empty_branch_design(),
        &source_spans,
    )
    .expect_err("missing CG target line must fail visibly");

    assert!(err.to_string().contains("event_cg line_index 不存在"));
}

#[test]
fn rejects_visual_plan_unknown_source_span() {
    let scenes = single_shooting_script_projection();
    let staging = single_narration_staging();
    let visual_plan = visual_plan_from_value(
        json!({
            "backgrounds": [{
                "scene_index": 1,
                "background_note": "雨夜车站",
                "source_span_ids": ["missing"]
            }],
            "event_cgs": []
        }),
        "galgame_visual_planning",
    )
    .expect("visual plan");
    let source_spans = single_source_span();

    let err = build_galgame_script_persist_plan(
        &scenes,
        &staging,
        &visual_plan,
        &empty_branch_design(),
        &source_spans,
    )
    .expect_err("unknown visual source span must fail visibly");

    assert!(err.to_string().contains("source_span_id 不存在"));
}
