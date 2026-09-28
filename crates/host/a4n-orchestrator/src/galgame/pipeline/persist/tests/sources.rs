use super::*;

#[test]
fn rejects_persist_plan_when_scene_source_span_is_unknown() {
    let scenes = adaptation_projection_from_value(
        json!({
            "scenes": [directing_scene_value(1, "雨夜重逢", "missing")]
        }),
        "galgame_adaptation_projection",
    )
    .expect("scenes");
    let staging = character_staging_from_value(
        json!({
            "scene_lines": [{
                "scene_index": 1,
                "lines": [{
                    "index": 1,
                    "origin": "source", "beat_ref": beat_reference(1, 1),
                    "line_type": "narration",
                    "text": "雨声压过脚步。",
                    "presentation": narration_presentation(),
                    "source_span_ids": ["missing"]
                }]
            }],
            "branch_scene_lines": []
        }),
        "galgame_character_staging",
    )
    .expect("staging");

    let err = build_galgame_script_persist_plan(
        &scenes,
        &staging,
        &visual_plan_for_scene(1, "missing"),
        &empty_branch_design(),
        &[],
    )
    .expect_err("unknown source span");

    assert!(err.to_string().contains("source_span_id 不存在"));
}

#[test]
fn rejects_persist_plan_when_staged_line_has_no_source_span() {
    let scenes = single_adaptation_projection();
    let staging = GalgameCharacterStaging {
        scene_lines: vec![GalgamePipelineSceneLineStaged {
            scene_index: 1,
            lines: vec![GalgamePipelineLineStaged {
                index: 1,
                origin: a4n_domain::media_authoring::AdaptationOrigin::Source,
                beat_ref: beat_reference(1, 1),
                line_type: LINE_TYPE_NARRATION.to_string(),
                speaker_profile_id: None,
                speaker_name: None,
                text: "雨声压过脚步。".to_string(),
                presentation: narration_presentation(),
                source_span_ids: Vec::new(),
            }],
        }],
        branch_scene_lines: vec![],
    };
    let source_spans = single_source_span();

    let err = build_galgame_script_persist_plan(
        &scenes,
        &staging,
        &single_visual_plan(),
        &empty_branch_design(),
        &source_spans,
    )
    .expect_err("external staged lines must keep source anchor evidence");

    assert!(err.to_string().contains("行缺少 source_span_ids"));
    assert!(err.to_string().contains("line_index=1"));
}

#[test]
fn rejects_persist_plan_when_secondary_line_source_span_is_unknown() {
    let scenes = single_adaptation_projection();
    let staging = GalgameCharacterStaging {
        scene_lines: vec![GalgamePipelineSceneLineStaged {
            scene_index: 1,
            lines: vec![GalgamePipelineLineStaged {
                index: 1,
                origin: a4n_domain::media_authoring::AdaptationOrigin::Source,
                beat_ref: beat_reference(1, 1),
                line_type: LINE_TYPE_NARRATION.to_string(),
                speaker_profile_id: None,
                speaker_name: None,
                text: "雨声压过脚步。".to_string(),
                presentation: narration_presentation(),
                source_span_ids: vec!["s1".to_string(), "missing".to_string()],
            }],
        }],
        branch_scene_lines: vec![],
    };
    let source_spans = single_source_span();

    let err = build_galgame_script_persist_plan(
        &scenes,
        &staging,
        &single_visual_plan(),
        &empty_branch_design(),
        &source_spans,
    )
    .expect_err("all line source_span_ids must be validated");

    assert!(err.to_string().contains("source_span_id 不存在: missing"));
}

#[test]
fn rejects_persist_plan_when_line_source_span_crosses_scene_chapter() {
    let scenes = single_adaptation_projection();
    let staging = GalgameCharacterStaging {
        scene_lines: vec![GalgamePipelineSceneLineStaged {
            scene_index: 1,
            lines: vec![GalgamePipelineLineStaged {
                index: 1,
                origin: a4n_domain::media_authoring::AdaptationOrigin::Source,
                beat_ref: beat_reference(1, 1),
                line_type: LINE_TYPE_NARRATION.to_string(),
                speaker_profile_id: None,
                speaker_name: None,
                text: "雨声压过脚步。".to_string(),
                presentation: narration_presentation(),
                source_span_ids: vec!["s1".to_string(), "s2".to_string()],
            }],
        }],
        branch_scene_lines: vec![],
    };
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
            source_snapshot_chapter_id: 21,
            chapter_number: 2,
        },
    ];

    let err = build_galgame_script_persist_plan(
        &scenes,
        &staging,
        &single_visual_plan(),
        &empty_branch_design(),
        &source_spans,
    )
    .expect_err("line source spans must stay inside the scene snapshot chapter");

    assert!(
        err.to_string()
            .contains("行 source_span_id 不属于当前场 source_snapshot_chapter")
    );
    assert!(err.to_string().contains("source_span_id=s2"));
}

#[test]
fn historical_scene_without_text_spans_uses_its_declared_source_chapter() {
    let mut scenes = single_adaptation_projection();
    scenes.scenes[0].origin = a4n_domain::media_authoring::AdaptationOrigin::Historical;
    scenes.scenes[0].source_span_ids.clear();
    for beat in &mut scenes.scenes[0].beats {
        beat.origin = a4n_domain::media_authoring::AdaptationOrigin::Historical;
        beat.source_span_ids.clear();
    }
    let scenes =
        adaptation_projection_from_value(serde_json::to_value(scenes).unwrap(), "director scene")
            .expect("director scenes may be based on user additions");
    let mut staging = single_narration_staging();
    staging.scene_lines[0].lines[0].origin =
        a4n_domain::media_authoring::AdaptationOrigin::Historical;
    staging.scene_lines[0].lines[0].source_span_ids.clear();
    let mut visual = single_visual_plan();
    visual.backgrounds[0].source_span_ids.clear();
    let plan = build_galgame_script_persist_plan(
        &scenes,
        &staging,
        &visual,
        &empty_branch_design(),
        &single_source_span(),
    )
    .expect("historical edits keep their declared chapter without inventing evidence");
    assert_eq!(plan.scenes[0].source_snapshot_chapter_id, 20);
    assert_eq!(plan.scenes[0].lines[0].text_anchor_id, None);
}

#[test]
fn rejects_persist_plan_when_staging_references_unknown_scene() {
    let scenes = single_adaptation_projection();
    let staging = character_staging_from_value(
        json!({
            "scene_lines": [
                {
                    "scene_index": 1,
                    "lines": [{
                        "index": 1,
                        "origin": "source", "beat_ref": beat_reference(1, 1),
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
                        "origin": "source", "beat_ref": beat_reference(2, 1),
                        "line_type": "narration",
                        "text": "不存在的场。",
                        "presentation": narration_presentation(),
                        "source_span_ids": ["s1"]
                    }]
                }
            ],
            "branch_scene_lines": []
        }),
        "galgame_character_staging",
    )
    .expect("staging");
    let source_spans = single_source_span();

    let err = build_galgame_script_persist_plan(
        &scenes,
        &staging,
        &single_visual_plan(),
        &empty_branch_design(),
        &source_spans,
    )
    .expect_err("extra staged scene must fail visibly");

    assert!(
        err.to_string()
            .contains("character_staging 引用不存在的 scene_index")
    );
}
