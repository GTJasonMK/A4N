use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_core::models::generation::workflow_checkpoint;
use a4n_db::galgame::{
    GalgameMediaAssetReadRecord, GalgameMediaBindingReadRecord,
    GalgamePersistValidationScopeReadModel, GalgamePersistValidationSourceSnapshotChapterRecord,
    GalgamePersistValidationSpeakerProfileRecord, GalgamePersistValidationTextAnchorRecord,
    GalgamePipelinePromptChapterSourceRecord, GalgamePipelinePromptContextRecord,
    GalgamePipelinePromptSpeakerProfileRecord, GalgamePipelinePromptTextAnchorRecord,
    GalgameScriptDetailChoiceRecord, GalgameScriptDetailReadModel, GalgameScriptDetailSceneRecord,
    GalgameScriptDetailScriptRecord,
};
use a4n_test_support::workflow::WorkflowRunModelBuilder;
use chrono::NaiveDate;
use serde_json::json;

use crate::media::MEDIA_BINDING_ROLE_BACKGROUND;

use super::super::read_models::galgame_pipeline_status_summary;
use super::super::{
    GALGAME_ENTITY_SCENE, GalgameFacingV2, GalgameLinePresentationDocumentV2,
    GalgameLinePresentationSchemaVersion, GalgamePersistValidationScope, GalgameStageLayerV2,
    GalgameStagePositionV2, GalgameVisibleCastMemberV2, galgame_media_binding_summaries,
    galgame_pipeline_prompt_context_from_record, galgame_script_detail_from_read_model,
};
use super::fixtures::{test_line_with, test_text_anchor};

#[test]
fn media_binding_summaries_reject_missing_asset_instead_of_dropping_binding() {
    let bindings = vec![GalgameMediaBindingReadRecord {
        id: 10,
        media_asset_id: 99,
        target_kind: GALGAME_ENTITY_SCENE.to_string(),
        target_id: 100,
        role: MEDIA_BINDING_ROLE_BACKGROUND.to_string(),
    }];

    let err = galgame_media_binding_summaries(bindings, &HashMap::new())
        .expect_err("missing media asset must not be silently dropped");

    assert!(matches!(
        &err,
        AppError::InvalidState { current, .. } if current == "galgame_read_model"
    ));
    assert!(err.to_string().contains("Galgame 媒体绑定 10"));
    assert!(err.to_string().contains("不存在的资产 99"));
}

#[test]
fn pipeline_prompt_context_record_builds_prompt_context() {
    let record = GalgamePipelinePromptContextRecord {
        chapters: vec![a4n_db::galgame::GalgamePipelinePromptChapterRecord {
            chapter: GalgamePipelinePromptChapterSourceRecord {
                id: 20,
                source_chapter_id: 2001,
                chapter_number: 1,
                title: Some("测试章节".to_string()),
                content: "雨夜车站。她回过头。".to_string(),
                summary: Some("章节摘要".to_string()),
                content_hash: "hash".to_string(),
            },
            text_anchors: vec![GalgamePipelinePromptTextAnchorRecord {
                id: 100,
                source_snapshot_chapter_id: Some(20),
                paragraph_index: 1,
                start_offset: 0,
                end_offset: 5,
                quote: "雨夜车站".to_string(),
            }],
        }],
        speaker_profiles: vec![GalgamePipelinePromptSpeakerProfileRecord {
            id: 7,
            name: "林晓雨".to_string(),
        }],
    };

    let context =
        galgame_pipeline_prompt_context_from_record(10, 1, 20, record).expect("prompt context");

    assert_eq!(context.project_id, 10);
    assert_eq!(context.root_project_id, 1);
    assert_eq!(context.source_snapshot_id, 20);
    assert_eq!(context.chapters[0].source_spans[0].id, "ch1:ta100");
    assert_eq!(context.source_spans[0].text_anchor_id, 100);
    assert_eq!(context.speaker_profiles[0].name, "林晓雨");
}

#[test]
fn pipeline_prompt_context_record_rejects_empty_chapters() {
    let err = galgame_pipeline_prompt_context_from_record(
        10,
        1,
        20,
        GalgamePipelinePromptContextRecord::default(),
    )
    .expect_err("empty source snapshot chapters must fail visibly");

    assert!(matches!(
        err,
        AppError::InvalidState { ref operation, .. }
            if operation == "load_galgame_pipeline_prompt_context"
    ));
    assert!(err.to_string().contains("source_snapshot_has_no_chapters"));
}

#[test]
fn pipeline_status_projects_only_the_canonical_wire_contract() {
    let created_at = NaiveDate::from_ymd_opt(2026, 8, 20)
        .expect("date")
        .and_hms_opt(10, 0, 0)
        .expect("time");
    let updated_at = NaiveDate::from_ymd_opt(2026, 8, 20)
        .expect("date")
        .and_hms_opt(10, 5, 0)
        .expect("time");
    let completed_at = NaiveDate::from_ymd_opt(2026, 8, 20)
        .expect("date")
        .and_hms_opt(10, 6, 0)
        .expect("time");
    let run = WorkflowRunModelBuilder::new(
        7,
        "hidden-business-key",
        "galgame_pipeline",
        "failed",
        created_at,
    )
    .with_project_scope(Some(10), Some(1))
    .with_source_snapshot_id(20)
    .with_scope("novel_project", 10)
    .with_chapter_id(30)
    .with_mode("full")
    .with_input_fingerprint(json!({ "hidden": true }))
    .with_current_step("visual_plan")
    .with_progress(json!({ "completed": 3, "total": 6 }))
    .with_active_task("task-7")
    .with_resumability(true, Some("retry".to_string()))
    .with_error_message("provider failed")
    .with_updated_at(updated_at)
    .with_completed_at(Some(completed_at))
    .build();
    let checkpoint = workflow_checkpoint::Model {
        id: 9,
        workflow_run_id: 7,
        step_key: "visual_plan".to_string(),
        step_order: 4,
        status: "failed".to_string(),
        input_fingerprint_sha256: Some("hidden-sha".to_string()),
        input_fingerprint_json: Some(json!({ "hidden": true })),
        checkpoint_json: Some(json!({ "scene": 1 })),
        artifacts_json: Some(json!({ "prompt_id": 12 })),
        generation_run_ids_json: Some(json!([99])),
        error_message: Some("provider failed".to_string()),
        created_at,
        updated_at,
        completed_at: Some(completed_at),
    };

    let summary = galgame_pipeline_status_summary(Some(run), vec![checkpoint]);

    assert_eq!(
        serde_json::to_value(summary).expect("status should serialize"),
        json!({
            "run": {
                "id": 7,
                "status": "failed",
                "mode": "full",
                "current_step_key": "visual_plan",
                "progress_json": { "completed": 3, "total": 6 },
                "active_task_id": "task-7",
                "can_resume": true,
                "resume_block_reason": "retry",
                "error_message": "provider failed",
                "created_at": "2026-08-20 10:00:00",
                "updated_at": "2026-08-20 10:05:00",
                "completed_at": "2026-08-20 10:06:00"
            },
            "checkpoints": [{
                "id": 9,
                "step_key": "visual_plan",
                "step_order": 4,
                "status": "failed",
                "checkpoint_json": { "scene": 1 },
                "artifacts_json": { "prompt_id": 12 },
                "error_message": "provider failed",
                "created_at": "2026-08-20 10:00:00",
                "updated_at": "2026-08-20 10:05:00",
                "completed_at": "2026-08-20 10:06:00"
            }]
        })
    );
    assert_eq!(
        serde_json::to_value(galgame_pipeline_status_summary(None, Vec::new()))
            .expect("empty status should serialize"),
        json!({ "run": null, "checkpoints": [] })
    );
}

#[test]
fn script_detail_read_model_groups_lines_choices_refs_and_media() {
    let mut line = test_line_with(
        10,
        Some(100),
        Some("smile"),
        Some("standing"),
        Some("center"),
    );
    line.scene_id = 30;
    line.speaker_profile_id = Some(7);
    line.presentation_json = serde_json::to_value(GalgameLinePresentationDocumentV2 {
        schema_version: GalgameLinePresentationSchemaVersion::V2,
        visible_cast: vec![GalgameVisibleCastMemberV2 {
            character_profile_id: 7,
            expression: "smile".to_string(),
            pose: "standing".to_string(),
            position: GalgameStagePositionV2::Center,
            facing: GalgameFacingV2::Front,
            layer: GalgameStageLayerV2::Front,
        }],
        enter: Vec::new(),
        exit: Vec::new(),
        focus_character_profile_id: Some(7),
        hold: false,
        player_knowledge: Vec::new(),
        cg: None,
        cg_note: None,
        cg_importance: None,
        transition: None,
        branch: None,
        branch_id: None,
        returns_to_scene_index: None,
        after_beat_ref: None,
    })
    .expect("presentation json");
    let read_model = GalgameScriptDetailReadModel {
        script: GalgameScriptDetailScriptRecord {
            id: 1,
            source_snapshot_id: 20,
            root_project_id: 1,
            title: "测试 Galgame".to_string(),
        },
        speaker_profiles: vec![GalgamePipelinePromptSpeakerProfileRecord {
            id: 7,
            name: "林晓雨".to_string(),
        }],
        text_anchors: vec![test_text_anchor(100, 20, 1, 0, 5, "雨夜车站")],
        media_bindings: a4n_db::galgame::GalgameMediaBindingReadModel {
            bindings: vec![GalgameMediaBindingReadRecord {
                id: 400,
                media_asset_id: 300,
                target_kind: GALGAME_ENTITY_SCENE.to_string(),
                target_id: 30,
                role: MEDIA_BINDING_ROLE_BACKGROUND.to_string(),
            }],
            assets: vec![GalgameMediaAssetReadRecord {
                id: 300,
                media_kind: "image".to_string(),
                file_path: Some("/tmp/image.png".to_string()),
                thumbnail_path: None,
            }],
        },
        scenes: vec![GalgameScriptDetailSceneRecord {
            id: 30,
            source_snapshot_chapter_id: 20,
            scene_index: 1,
            title: "雨夜车站".to_string(),
            location: Some("车站".to_string()),
            time_of_day: Some("夜晚".to_string()),
            background_note: None,
            entry_kind: "main".to_string(),
            next_scene_id: None,
        }],
        lines: vec![line],
        choices: vec![GalgameScriptDetailChoiceRecord {
            id: 500,
            line_id: 10,
            option_index: 1,
            option_text: "留下".to_string(),
            target_scene_id: Some(30),
        }],
    };

    let detail = galgame_script_detail_from_read_model(read_model).expect("script detail");

    assert_eq!(detail.id, 1);
    assert_eq!(detail.speaker_profiles[0].id, 7);
    assert_eq!(detail.text_anchors[0].id, 100);
    assert_eq!(detail.media_bindings[0].asset.id, 300);
    assert_eq!(detail.scenes[0].scene.id, 30);
    assert_eq!(detail.scenes[0].lines[0].id, 10);
    assert_eq!(detail.scenes[0].choices[0].option_text, "留下");
}

#[test]
fn persist_validation_scope_read_model_indexes_records() {
    let read_model = GalgamePersistValidationScopeReadModel {
        source_snapshot_chapters: vec![GalgamePersistValidationSourceSnapshotChapterRecord {
            id: 20,
            snapshot_id: 1,
        }],
        speaker_profiles: vec![GalgamePersistValidationSpeakerProfileRecord {
            id: 7,
            project_id: 1,
        }],
        text_anchors: vec![GalgamePersistValidationTextAnchorRecord {
            id: 100,
            project_id: 1,
            source_snapshot_id: Some(20),
            source_snapshot_chapter_id: Some(20),
        }],
    };

    let scope = GalgamePersistValidationScope::from_read_model(read_model);

    assert!(scope.source_snapshot_chapters.contains_key(&20));
    assert!(scope.speaker_profiles.contains_key(&7));
    assert!(scope.text_anchors.contains_key(&100));
}
