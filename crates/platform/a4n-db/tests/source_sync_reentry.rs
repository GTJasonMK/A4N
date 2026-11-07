use a4n_db::project::{
    CreateDerivedProjectParams, CreateProjectRecordInput, DerivedChapterScope,
    create_derived_project_record, create_project_record,
};
use a4n_db::source_sync::{
    PreparedSourceSyncDecisionRecordInput, SourceSyncGalgameSceneLineWriteRecordInput,
    SourceSyncGalgameSceneWriteRecordInput,
    apply_prepared_source_sync_decisions_records_transaction, create_source_sync_preview_records,
    galgame_scene_regeneration_source_record_from_records,
};
use a4n_db_models::models::{
    chapter::chapter,
    galgame::{galgame_scene, galgame_script},
    project::{novel_project, source_snapshot_chapter},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;

#[path = "support/media_authoring.rs"]
mod authoring;

fn keep(conflict_id: i64) -> PreparedSourceSyncDecisionRecordInput {
    PreparedSourceSyncDecisionRecordInput::KeepLocal {
        conflict_id,
        decision_payload_json: json!({"user_action": "keep_local"}),
    }
}

#[tokio::test]
async fn partial_sync_survives_recheck_and_kept_scenes_participate_in_later_updates() {
    let db = a4n_migration::test_support::connect_test_database().await;
    a4n_test_support::database::ensure_test_user(&db, 1).await;
    let root = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("来源连续同步 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(root.id),
        chapter_number: Set(1),
        title: Set(Some("第一章".into())),
        content: Set("原始正文。".into()),
        word_count: Set(5),
        status: Set(chapter::STATUS_COMPLETED.into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let derived = create_derived_project_record(
        &db,
        1,
        CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: "galgame".into(),
            title: "同步目标".into(),
            description: String::new(),
            chapter_scope: DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    let source = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(derived.source_snapshot_id.unwrap()))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let script = galgame_script::ActiveModel {
        project_id: Set(derived.id),
        root_project_id: Set(root.id),
        source_snapshot_id: Set(derived.source_snapshot_id.unwrap()),
        title: Set("同步剧本".into()),
        style_config_json: Set(json!({})),
        status: Set("draft".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let mut scene_ids = Vec::new();
    let initial_plan = authoring::adopt_historical_game_plan(&db, derived.id, &source).await;
    for (index, kind) in ["main", "branch"].into_iter().enumerate() {
        let scene = galgame_scene::ActiveModel {
            adaptation_plan_id: Set(Some(initial_plan.id)),
            adaptation_unit_id: Set(Some(format!("legacy-scene-{}", index + 1))),
            script_id: Set(script.id),
            source_snapshot_chapter_id: Set(source.id),
            scene_index: Set(index as i32 + 1),
            title: Set(format!("场景 {index}")),
            participants_json: Set(json!([])),
            entry_kind: Set(kind.into()),
            next_scene_id: Set(if kind == "branch" {
                Some(scene_ids[0])
            } else {
                None
            }),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        scene_ids.push(scene.id);
    }
    let mut changed: chapter::ActiveModel = chapter.clone().into();
    changed.content = Set("第一次修改后的正文。".into());
    let changed = changed.update(&db).await.unwrap();

    let first = create_source_sync_preview_records(&db, derived.id)
        .await
        .unwrap();
    let main_conflict = first
        .conflicts
        .iter()
        .find(|conflict| {
            conflict.target_kind == "galgame_scene" && conflict.target_id == scene_ids[0]
        })
        .unwrap();
    let branch_conflict = first
        .conflicts
        .iter()
        .find(|conflict| {
            conflict.target_kind == "galgame_scene" && conflict.target_id == scene_ids[1]
        })
        .unwrap();
    assert_eq!(
        first.regeneration_targets.get(&main_conflict.id),
        Some(&scene_ids[0])
    );
    assert!(!first.regeneration_targets.contains_key(&branch_conflict.id));
    let kept_id = first
        .conflicts
        .iter()
        .find(|conflict| conflict.target_kind == "chapter")
        .unwrap()
        .id;
    let partial = apply_prepared_source_sync_decisions_records_transaction(
        &db,
        first.run.id,
        vec![keep(kept_id)],
    )
    .await
    .unwrap();
    assert_eq!(partial.run.status, "applying");
    for identity in [None, Some("legacy-scene-1".to_string())] {
        galgame_scene::ActiveModel {
            id: Set(scene_ids[0]),
            adaptation_plan_id: Set(identity.as_ref().map(|_| initial_plan.id)),
            adaptation_unit_id: Set(identity.clone()),
            ..Default::default()
        }
        .update(&db)
        .await
        .unwrap();
        let capabilities = create_source_sync_preview_records(&db, derived.id)
            .await
            .unwrap();
        assert_eq!(capabilities.run.id, partial.run.id);
        assert_eq!(
            capabilities
                .regeneration_targets
                .contains_key(&main_conflict.id),
            identity.is_some()
        );
    }
    let mut newer_root: chapter::ActiveModel = changed.clone().into();
    newer_root.content = Set("计划尚未处理完时，根正文再次修改。".into());
    newer_root.update(&db).await.unwrap();
    let rechecked = create_source_sync_preview_records(&db, derived.id)
        .await
        .unwrap();
    assert_eq!(rechecked.run.id, first.run.id);
    assert_eq!(rechecked.conflicts, partial.conflicts);
    assert_eq!(
        rechecked
            .conflicts
            .iter()
            .filter(|conflict| conflict.status == "pending")
            .count(),
        2
    );
    let decisions = rechecked
        .conflicts
        .iter()
        .filter(|conflict| conflict.status == "pending")
        .map(|conflict| keep(conflict.id))
        .collect();
    let completed =
        apply_prepared_source_sync_decisions_records_transaction(&db, first.run.id, decisions)
            .await
            .unwrap();
    assert_eq!(completed.run.status, "completed");
    let after_completion = novel_project::Entity::find_by_id(derived.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after_completion.source_sync_status, "root_updated");
    assert_eq!(
        after_completion.source_snapshot_id,
        Some(first.run.to_snapshot_id)
    );

    let mut changed: chapter::ActiveModel = changed.into();
    changed.content = Set("第二次修改后的正文。".into());
    changed.update(&db).await.unwrap();
    let next = create_source_sync_preview_records(&db, derived.id)
        .await
        .unwrap();
    let next_scenes = next
        .conflicts
        .iter()
        .filter(|conflict| conflict.target_kind == "galgame_scene")
        .collect::<Vec<_>>();
    assert_eq!(next_scenes.len(), 2);
    assert_ne!(next.run.from_snapshot_id, source.snapshot_id);
    let main_conflict = next_scenes
        .into_iter()
        .find(|conflict| conflict.target_id == scene_ids[0])
        .unwrap();
    let regeneration = galgame_scene_regeneration_source_record_from_records(
        &db,
        &next.run,
        main_conflict,
        scene_ids[0],
    )
    .await
    .unwrap();
    assert_eq!(regeneration.old_snapshot_chapter.id, source.id);
    assert_eq!(
        regeneration.new_snapshot_chapter.snapshot_id,
        next.run.to_snapshot_id
    );
    assert_eq!(
        regeneration.new_snapshot_chapter.source_chapter_id,
        chapter.id
    );
    let target_chapter = regeneration.new_snapshot_chapter.id;
    let target_anchor = regeneration.anchors.first().expect("target anchor").id;
    let target_source_chapter = source_snapshot_chapter::Entity::find_by_id(target_chapter)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let mut target_snapshot: novel_project::ActiveModel = after_completion.clone().into();
    target_snapshot.source_snapshot_id = Set(Some(next.run.to_snapshot_id));
    target_snapshot.update(&db).await.unwrap();
    let target_plan =
        authoring::adopt_historical_game_plan(&db, derived.id, &target_source_chapter).await;
    let mut current_snapshot: novel_project::ActiveModel =
        novel_project::Entity::find_by_id(derived.id)
            .one(&db)
            .await
            .unwrap()
            .expect("derived project must remain available during preview")
            .into();
    current_snapshot.source_snapshot_id = Set(after_completion.source_snapshot_id);
    current_snapshot.update(&db).await.unwrap();
    let regenerated = apply_prepared_source_sync_decisions_records_transaction(
        &db,
        next.run.id,
        vec![PreparedSourceSyncDecisionRecordInput::RegenerateScene {
            conflict_id: main_conflict.id,
            galgame_scene_id: scene_ids[0],
            reason: None,
            expected_scene_index: 1,
            expected_source_snapshot_chapter_id: target_chapter,
            generated_scene: SourceSyncGalgameSceneWriteRecordInput {
                adaptation_plan_id: target_plan.id,
                adaptation_plan_revision: target_plan.revision,
                adaptation_unit_id: "legacy-scene-1".into(),
                title: "更新后的主线".into(),
                location: None,
                time_of_day: None,
                participants_json: json!([]),
                background_note: None,
                lines: vec![SourceSyncGalgameSceneLineWriteRecordInput {
                    line_index: 1,
                    line_type: "narration".into(),
                    speaker_profile_id: None,
                    text: "第二次修改后的新旁白。".into(),
                    expression: None,
                    pose: None,
                    position: None,
                    text_anchor_id: Some(target_anchor),
                    presentation_json: json!({
                        "schema_version": "galgame_line_presentation_v3", "origin": "historical",
                        "beat_ref": {"source_chapter_id": chapter.id, "unit_id": "legacy-scene-1", "beat_id": "unfinished"},
                        "visible_cast": [], "enter": [], "exit": [],
                        "focus_character_profile_id": null, "hold": false,
                        "player_knowledge": [], "cg": null, "cg_note": null,
                        "cg_importance": null, "transition": null, "branch": null,
                        "branch_id": null, "returns_to_scene_index": null, "after_beat_ref": null
                    }),
                }],
            },
            stage_outputs_json: json!({}),
            decision_payload_json: json!({"user_action": "regenerate_scene"}),
        }],
    )
    .await
    .unwrap();
    assert!(
        regenerated
            .conflicts
            .iter()
            .any(|conflict| conflict.id == main_conflict.id && conflict.status == "resolved")
    );
    let script_after = galgame_script::Entity::find_by_id(script.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        script_after.source_snapshot_id, source.snapshot_id,
        "script keeps initial provenance"
    );
    for (scene_id, snapshot_id) in [
        (scene_ids[0], next.run.to_snapshot_id),
        (scene_ids[1], source.snapshot_id),
    ] {
        let context = a4n_db::galgame::galgame_media_target_context(
            &db,
            derived.id,
            "galgame_scene",
            scene_id,
        )
        .await
        .unwrap();
        assert_eq!(context.source_snapshot_id, snapshot_id);
    }
    let detail = a4n_db::galgame::galgame_script_detail_read_model(&db, derived.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(detail.lines.len(), 1);
    assert_eq!(detail.text_anchors[0].id, target_anchor);
    assert_eq!(detail.lines[0].text, "第二次修改后的新旁白。");
}
