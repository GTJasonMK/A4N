use super::*;

#[test]
fn builds_persist_plan_from_staged_pipeline_outputs() {
    let scenes = shooting_script_projection_from_value(
        json!({
            "scenes": [directing_scene_value_with_beats(
                1,
                "雨夜重逢",
                "s1",
                vec![directing_beat(1, "s1"), directing_beat(2, "s1")]
            )]
        }),
        "galgame_shooting_script_projection",
    )
    .expect("scenes");
    let staging = character_staging_from_value(
        json!({
            "scene_lines": [{
                "scene_index": 1,
                "lines": [
                    {
                        "index": 1,
                        "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                        "line_type": "dialogue",
                        "speaker_profile_id": 7,
                        "speaker_name": "林晓雨",
                        "text": "你终于来了。",
                        "presentation": speaker_presentation(7, "smile"),
                        "source_span_ids": ["s1"]
                    },
                    {
                        "index": 2,
                        "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 2},
                        "line_type": "narration",
                        "speaker_name": "",
                        "text": "雨声压过脚步。",
                        "presentation": narration_presentation(),
                        "source_span_ids": ["s2"]
                    }
                ]
            }],
            "branch_scene_lines": []
        }),
        "galgame_character_staging",
    )
    .expect("staging");
    let visual_plan = visual_plan_from_value(
        json!({
            "backgrounds": [{
                "scene_index": 1,
                "background_note": "雨夜车站，冷色灯光",
                "scene_profile_id": null,
                "source_span_ids": ["s1"]
            }],
            "event_cgs": [{
                "scene_index": 1,
                "line_index": 1,
                "cg_note": "少女在雨中回头",
                "importance": "high",
                "source_span_ids": ["s1"]
            }]
        }),
        "galgame_visual_planning",
    )
    .expect("visual");
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

    let plan = build_galgame_script_persist_plan(
        &scenes,
        &staging,
        &visual_plan,
        &empty_branch_design(),
        &source_spans,
    )
    .expect("persist plan");

    assert_eq!(plan.scenes.len(), 1);
    assert_eq!(plan.scenes[0].source_snapshot_chapter_id, 20);
    assert_eq!(
        plan.scenes[0].background_note.as_deref(),
        Some("雨夜车站，冷色灯光")
    );
    assert_eq!(plan.scenes[0].lines[0].text_anchor_id, Some(100));
    assert_eq!(plan.scenes[0].lines[0].presentation_json["cg"], json!(true));
    serde_json::from_value::<GalgameLinePresentationDocumentV2>(
        plan.scenes[0].lines[0].presentation_json.clone(),
    )
    .expect("persisted presentation must use the complete v2 document");
    assert_eq!(
        plan.scenes[0].participants_json,
        json!([{"character_profile_id": 7, "name": "林晓雨"}])
    );
}
