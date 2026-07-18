use std::collections::BTreeMap;

use a4n_core::error::AppError;
use a4n_core::models::media::media_binding;
use a4n_db::galgame::{
    GalgamePersistValidationScopeReadModel, GalgamePipelinePromptContextRecord,
    GalgameScriptDetailReadModel,
};
use chrono::Utc;

use crate::media::MEDIA_BINDING_ROLE_BACKGROUND;

use super::super::{
    GALGAME_ENTITY_SCENE, GalgamePersistValidationScope, galgame_media_binding_summaries,
    galgame_pipeline_prompt_context_from_record, galgame_script_detail_from_read_model,
};
use super::fixtures::{
    test_choice, test_line_with, test_media_asset, test_profile, test_scene, test_script,
    test_snapshot_chapter, test_text_anchor,
};

#[test]
fn media_binding_summaries_reject_missing_asset_instead_of_dropping_binding() {
    let now = Utc::now().naive_utc();
    let bindings = vec![media_binding::Model {
        id: 10,
        media_asset_id: 99,
        target_kind: GALGAME_ENTITY_SCENE.to_string(),
        target_id: 100,
        role: MEDIA_BINDING_ROLE_BACKGROUND.to_string(),
        created_at: now,
    }];

    let err = galgame_media_binding_summaries(bindings, &BTreeMap::new())
        .expect_err("missing media asset must not be silently dropped");

    assert!(matches!(
        err,
        AppError::InvalidState { ref operation, .. }
            if operation == "load_galgame_media_bindings"
    ));
    assert!(err.to_string().contains("binding_id=10"));
    assert!(err.to_string().contains("media_asset_id=99"));
}

#[test]
fn pipeline_prompt_context_record_builds_prompt_context() {
    let record = GalgamePipelinePromptContextRecord {
        chapters: vec![a4n_db::galgame::GalgamePipelinePromptChapterRecord {
            chapter: test_snapshot_chapter(20, 1, "雨夜车站。她回过头。"),
            text_anchors: vec![test_text_anchor(100, 20, 1, 0, 5, "雨夜车站")],
        }],
        speaker_profiles: vec![test_profile(7, "林晓雨")],
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
fn script_detail_read_model_groups_lines_choices_refs_and_media() {
    let mut line = test_line_with(10, Some(100), Some("smile"), None, Some("center"));
    line.scene_id = 30;
    let read_model = GalgameScriptDetailReadModel {
        script: test_script(1),
        speaker_profiles: vec![test_profile(7, "林晓雨")],
        text_anchors: vec![test_text_anchor(100, 20, 1, 0, 5, "雨夜车站")],
        media_bindings: a4n_db::galgame::GalgameMediaBindingReadModel {
            bindings: vec![media_binding::Model {
                id: 400,
                media_asset_id: 300,
                target_kind: GALGAME_ENTITY_SCENE.to_string(),
                target_id: 30,
                role: MEDIA_BINDING_ROLE_BACKGROUND.to_string(),
                created_at: Utc::now().naive_utc(),
            }],
            assets: vec![test_media_asset(300)],
        },
        scenes: vec![test_scene(30, 1)],
        lines: vec![line],
        choices: vec![test_choice(500, 10, 1, "留下")],
    };

    let detail = galgame_script_detail_from_read_model(read_model).expect("script detail");

    assert_eq!(detail.script.id, 1);
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
        source_snapshot_chapters: vec![test_snapshot_chapter(20, 1, "雨夜车站。")],
        speaker_profiles: vec![test_profile(7, "林晓雨")],
        text_anchors: vec![test_text_anchor(100, 20, 1, 0, 5, "雨夜车站")],
    };

    let scope = GalgamePersistValidationScope::from_read_model(read_model);

    assert!(scope.source_snapshot_chapters.contains_key(&20));
    assert!(scope.speaker_profiles.contains_key(&7));
    assert!(scope.text_anchors.contains_key(&100));
}
