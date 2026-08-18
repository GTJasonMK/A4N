use serde_json::json;

use super::super::{
    GALGAME_PIPELINE_TOTAL_STEPS, GalgameCgImportance, GalgameCharacterStaging,
    GalgameLinePresentationSchemaVersion, GalgameLinePresentationV2, GalgamePipelineBackgroundPlan,
    GalgamePipelineBeatRefV3, GalgamePipelineEventCgPlan, GalgamePipelineLineDraft,
    GalgamePipelineLineStaged, GalgamePipelineSceneLineDraft, GalgamePipelineSceneLineStaged,
    GalgamePipelineStartStage, GalgameScriptAdaptation, GalgameVisualPlan, LINE_TYPE_NARRATION,
};

fn narration_presentation() -> GalgameLinePresentationV2 {
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
use super::checkpoints::restored_galgame_llm_stage_output;
use super::progress::galgame_progress_update;
use super::validation::{
    ensure_single_regenerated_scene_lines, ensure_single_regenerated_staged_lines,
    normalize_single_scene_visual_plan,
};

#[test]
fn restored_galgame_llm_stage_output_revalidates_parsed_output() {
    let restored = restored_galgame_llm_stage_output(
        &json!({ "value": 1 }),
        r#"{"value":1}"#,
        "scenes",
        "galgame_test",
        Ok,
    )
    .expect("restored checkpoint");

    assert_eq!(restored, json!({ "value": 1 }));
}

#[test]
fn restored_galgame_llm_stage_output_rejects_mismatched_checkpoint() {
    let error = restored_galgame_llm_stage_output(
        &json!({ "value": 2 }),
        r#"{"value":1}"#,
        "scenes",
        "galgame_test",
        Ok,
    )
    .err()
    .expect("mismatched checkpoint must fail");

    assert!(
        error
            .to_string()
            .contains("checkpoint_parsed_output_mismatch")
    );
}

#[test]
fn galgame_progress_update_uses_galgame_generation_meta_contract() {
    let update = galgame_progress_update(
        GalgamePipelineStartStage::PersistScript,
        6,
        "正在写入 Galgame 剧本",
    )
    .expect("valid progress");
    let meta = serde_json::to_value(&update.meta).expect("progress meta json");

    assert_eq!(update.current, 6);
    assert_eq!(update.total, GALGAME_PIPELINE_TOTAL_STEPS);
    assert_eq!(update.message, "正在写入 Galgame 剧本");
    assert_eq!(meta["taskKind"], "galgame_generation");
    assert_eq!(meta["stageCode"], "persist_script");
    assert_eq!(meta["stageLabel"], "写入剧本");
    assert_eq!(meta["phaseKind"], "save");
    assert_eq!(meta["detail"], "正在写入 Galgame 剧本");
}

#[test]
fn galgame_progress_update_rejects_zero_current() {
    let error = galgame_progress_update(
        GalgamePipelineStartStage::ShootingScriptProjection,
        0,
        "正在投影结构化拍摄剧本",
    )
    .expect_err("zero progress must fail");

    assert!(error.to_string().contains("必须在 1..=7"));
}

#[test]
fn source_sync_scene_regeneration_requires_one_non_empty_scene() {
    let empty = GalgameScriptAdaptation {
        scene_lines: vec![],
        branch_scene_lines: vec![],
    };
    assert!(ensure_single_regenerated_scene_lines(&empty).is_err());

    let wrong_scene = GalgameScriptAdaptation {
        scene_lines: vec![GalgamePipelineSceneLineDraft {
            scene_index: 2,
            lines: vec![GalgamePipelineLineDraft {
                index: 1,
                beat_ref: GalgamePipelineBeatRefV3 {
                    scene_index: 2,
                    shot_index: 1,
                    beat_index: 1,
                },
                line_type: LINE_TYPE_NARRATION.to_string(),
                speaker_name: None,
                text: "雨停了。".to_string(),
                source_span_ids: vec!["ch1:ta1".to_string()],
            }],
        }],
        branch_scene_lines: vec![],
    };
    assert!(ensure_single_regenerated_scene_lines(&wrong_scene).is_err());

    let valid = GalgameScriptAdaptation {
        scene_lines: vec![GalgamePipelineSceneLineDraft {
            scene_index: 1,
            lines: vec![GalgamePipelineLineDraft {
                index: 1,
                beat_ref: GalgamePipelineBeatRefV3 {
                    scene_index: 1,
                    shot_index: 1,
                    beat_index: 1,
                },
                line_type: LINE_TYPE_NARRATION.to_string(),
                speaker_name: None,
                text: "雨停了。".to_string(),
                source_span_ids: vec!["ch1:ta1".to_string()],
            }],
        }],
        branch_scene_lines: vec![],
    };
    ensure_single_regenerated_scene_lines(&valid).expect("single non-empty scene should pass");
}

#[test]
fn source_sync_scene_regeneration_requires_one_non_empty_staged_scene() {
    let empty = GalgameCharacterStaging {
        scene_lines: vec![],
        branch_scene_lines: vec![],
    };
    assert!(ensure_single_regenerated_staged_lines(&empty).is_err());

    let wrong_scene = GalgameCharacterStaging {
        scene_lines: vec![GalgamePipelineSceneLineStaged {
            scene_index: 2,
            lines: vec![GalgamePipelineLineStaged {
                index: 1,
                beat_ref: GalgamePipelineBeatRefV3 {
                    scene_index: 2,
                    shot_index: 1,
                    beat_index: 1,
                },
                line_type: LINE_TYPE_NARRATION.to_string(),
                speaker_profile_id: None,
                speaker_name: None,
                text: "雨停了。".to_string(),
                presentation: narration_presentation(),
                source_span_ids: vec!["ch1:ta1".to_string()],
            }],
        }],
        branch_scene_lines: vec![],
    };
    assert!(ensure_single_regenerated_staged_lines(&wrong_scene).is_err());

    let valid = GalgameCharacterStaging {
        scene_lines: vec![GalgamePipelineSceneLineStaged {
            scene_index: 1,
            lines: vec![GalgamePipelineLineStaged {
                index: 1,
                beat_ref: GalgamePipelineBeatRefV3 {
                    scene_index: 1,
                    shot_index: 1,
                    beat_index: 1,
                },
                line_type: LINE_TYPE_NARRATION.to_string(),
                speaker_profile_id: None,
                speaker_name: None,
                text: "雨停了。".to_string(),
                presentation: narration_presentation(),
                source_span_ids: vec!["ch1:ta1".to_string()],
            }],
        }],
        branch_scene_lines: vec![],
    };
    ensure_single_regenerated_staged_lines(&valid)
        .expect("single non-empty staged scene should pass");
}

#[test]
fn source_sync_scene_regeneration_visual_plan_rejects_extra_scene_backgrounds() {
    let plan = GalgameVisualPlan {
        backgrounds: vec![
            GalgamePipelineBackgroundPlan {
                scene_index: 1,
                background_note: "校门口雨后".to_string(),
                scene_profile_id: None,
                source_span_ids: vec!["ch1:ta1".to_string()],
            },
            GalgamePipelineBackgroundPlan {
                scene_index: 2,
                background_note: "旧场景".to_string(),
                scene_profile_id: None,
                source_span_ids: vec!["ch1:ta2".to_string()],
            },
        ],
        event_cgs: vec![],
    };

    let err = normalize_single_scene_visual_plan(plan).expect_err("extra scene must fail visibly");

    assert!(
        err.to_string()
            .contains("只返回一个 scene_index=1 背景规划")
    );
}

#[test]
fn source_sync_scene_regeneration_visual_plan_rejects_extra_scene_event_cg() {
    let plan = GalgameVisualPlan {
        backgrounds: vec![GalgamePipelineBackgroundPlan {
            scene_index: 1,
            background_note: "校门口雨后".to_string(),
            scene_profile_id: None,
            source_span_ids: vec!["ch1:ta1".to_string()],
        }],
        event_cgs: vec![
            GalgamePipelineEventCgPlan {
                scene_index: 1,
                line_index: 1,
                cg_note: "抬头看天".to_string(),
                importance: GalgameCgImportance::Normal,
                source_span_ids: vec!["ch1:ta1".to_string()],
            },
            GalgamePipelineEventCgPlan {
                scene_index: 2,
                line_index: 1,
                cg_note: "不能跨场".to_string(),
                importance: GalgameCgImportance::Normal,
                source_span_ids: vec!["ch1:ta2".to_string()],
            },
        ],
    };

    let err = normalize_single_scene_visual_plan(plan).expect_err("extra event CG scene must fail");

    assert!(err.to_string().contains("事件 CG 不能引用其他场"));
}

#[test]
fn source_sync_scene_regeneration_visual_plan_requires_background() {
    let plan = GalgameVisualPlan {
        backgrounds: vec![GalgamePipelineBackgroundPlan {
            scene_index: 2,
            background_note: "旧场景".to_string(),
            scene_profile_id: None,
            source_span_ids: vec!["ch1:ta2".to_string()],
        }],
        event_cgs: vec![],
    };

    assert!(normalize_single_scene_visual_plan(plan).is_err());
}
