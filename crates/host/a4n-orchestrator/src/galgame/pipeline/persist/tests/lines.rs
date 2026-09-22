use super::*;

#[test]
fn generated_script_and_staging_are_checked_before_following_calls() {
    let scenes = single_shooting_script_projection();
    let mut staging = single_narration_staging();
    let line = &staging.scene_lines[0].lines[0];
    let mut script = GalgameScriptAdaptation {
        scene_lines: vec![GalgamePipelineSceneLineDraft {
            scene_index: 1,
            lines: vec![GalgamePipelineLineDraft {
                index: line.index,
                beat_ref: line.beat_ref,
                line_type: line.line_type.clone(),
                speaker_name: line.speaker_name.clone(),
                text: line.text.clone(),
                source_span_ids: line.source_span_ids.clone(),
            }],
        }],
        branch_scene_lines: Vec::new(),
    };
    assert!(
        validate_galgame_script_adaptation(
            &script,
            &scenes,
            &empty_branch_design(),
            &single_source_span()
        )
        .is_ok()
    );
    assert!(validate_galgame_staging(&staging, &script, &[]).is_ok());
    script.scene_lines[0].lines[0].beat_ref.beat_index = 99;
    assert!(
        validate_galgame_script_adaptation(
            &script,
            &scenes,
            &empty_branch_design(),
            &single_source_span()
        )
        .is_err()
    );
    script.scene_lines[0].lines[0].beat_ref.beat_index = 1;
    staging.scene_lines[0].lines[0].text = "改写的事实".into();
    assert!(validate_galgame_staging(&staging, &script, &[]).is_err());
    staging.scene_lines[0].lines[0].text = script.scene_lines[0].lines[0].text.clone();
    staging.scene_lines[0].lines[0].presentation.exit = vec![999];
    assert!(validate_galgame_staging(&staging, &script, &[]).is_err());
}

#[test]
fn rejects_persist_plan_when_dialogue_has_no_profile() {
    let scenes = shooting_script_projection_from_value(
        json!({
            "scenes": [directing_scene_value(1, "雨夜重逢", "s1")]
        }),
        "galgame_shooting_script_projection",
    )
    .expect("scenes");
    let staging = character_staging_from_value(
        json!({
            "scene_lines": [{
                "scene_index": 1,
                "lines": [{
                    "index": 1,
                    "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                    "line_type": "dialogue",
                    "speaker_name": "林晓雨",
                    "text": "你终于来了。",
                    "presentation": narration_presentation(),
                    "source_span_ids": ["s1"]
                }]
            }],
            "branch_scene_lines": []
        }),
        "galgame_character_staging",
    )
    .expect("staging");
    let source_spans = vec![GalgamePipelineSourceSpan {
        id: "s1".to_string(),
        text_anchor_id: 100,
        source_snapshot_chapter_id: 20,
    }];

    let err = build_galgame_script_persist_plan(
        &scenes,
        &staging,
        &single_visual_plan(),
        &empty_branch_design(),
        &source_spans,
    )
    .expect_err("missing profile");

    assert!(err.to_string().contains("必须绑定 speaker_profile_id"));
}

#[test]
fn rejects_persist_plan_when_dialogue_speaker_is_missing_from_visible_cast() {
    let scenes = single_shooting_script_projection();
    let staging = GalgameCharacterStaging {
        scene_lines: vec![GalgamePipelineSceneLineStaged {
            scene_index: 1,
            lines: vec![GalgamePipelineLineStaged {
                index: 1,
                beat_ref: GalgamePipelineBeatRefV3 {
                    scene_index: 1,
                    shot_index: 1,
                    beat_index: 1,
                },
                line_type: LINE_TYPE_DIALOGUE.to_string(),
                speaker_profile_id: Some(7),
                speaker_name: Some("林晓雨".to_string()),
                text: "你终于来了。".to_string(),
                presentation: narration_presentation(),
                source_span_ids: vec!["s1".to_string()],
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
    .expect_err("persist layer must require speaker in visible cast");

    assert!(err.to_string().contains("必须包含当前 speaker_profile_id"));
}

#[test]
fn rejects_persist_plan_when_expression_is_invalid() {
    let scenes = single_shooting_script_projection();
    let staging = GalgameCharacterStaging {
        scene_lines: vec![GalgamePipelineSceneLineStaged {
            scene_index: 1,
            lines: vec![GalgamePipelineLineStaged {
                index: 1,
                beat_ref: GalgamePipelineBeatRefV3 {
                    scene_index: 1,
                    shot_index: 1,
                    beat_index: 1,
                },
                line_type: LINE_TYPE_DIALOGUE.to_string(),
                speaker_profile_id: Some(7),
                speaker_name: Some("林晓雨".to_string()),
                text: "你终于来了。".to_string(),
                presentation: speaker_presentation(7, "wink"),
                source_span_ids: vec!["s1".to_string()],
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
    .expect_err("persist layer must reject invalid expression");

    assert!(err.to_string().contains("expression 无效: wink"));
}

#[test]
fn stage_direction_text_does_not_infer_transition_instruction() {
    let scenes = single_shooting_script_projection();
    let staging = GalgameCharacterStaging {
        scene_lines: vec![GalgamePipelineSceneLineStaged {
            scene_index: 1,
            lines: vec![GalgamePipelineLineStaged {
                index: 1,
                beat_ref: GalgamePipelineBeatRefV3 {
                    scene_index: 1,
                    shot_index: 1,
                    beat_index: 1,
                },
                line_type: LINE_TYPE_STAGE_DIRECTION.to_string(),
                speaker_profile_id: None,
                speaker_name: None,
                text: "转场到雨夜站台。".to_string(),
                presentation: narration_presentation(),
                source_span_ids: vec!["s1".to_string()],
            }],
        }],
        branch_scene_lines: vec![],
    };

    let plan = build_galgame_script_persist_plan(
        &scenes,
        &staging,
        &single_visual_plan(),
        &empty_branch_design(),
        &single_source_span(),
    )
    .expect("stage direction persist plan");

    assert_eq!(
        plan.scenes[0].lines[0].presentation_json["transition"],
        serde_json::Value::Null
    );
}
