use super::contracts::CharacterProfileRelatedCounts;

#[test]
fn empty_related_counts_produce_zero_timeline_rows() {
    assert_eq!(CharacterProfileRelatedCounts::default().timeline_rows(), 0);
}

#[test]
fn timeline_rows_exclude_snapshots_and_portraits() {
    let counts = CharacterProfileRelatedCounts {
        attribute_events: 2,
        character_states: 3,
        behaviors: 5,
        deletion_marks: 7,
        character_snapshots: 11,
        character_portraits: 13,
    };

    assert_eq!(counts.timeline_rows(), 17);
    assert_eq!(counts.character_snapshots, 11);
    assert_eq!(counts.character_portraits, 13);
}

#[tokio::test]
async fn reset_preserves_referenced_character_identity_but_clears_text_generated_attributes() {
    use crate::character::{
        CharacterProfileDeleteError, create_profile, delete_profile_cascade_transaction,
    };
    use crate::project::{
        CreateDerivedProjectParams, CreateProjectRecordInput, DerivedChapterScope,
        create_derived_project_record, create_project_record,
    };
    use a4n_db_models::models::{
        chapter::chapter,
        character::protagonist_profile,
        galgame::{galgame_scene, galgame_script},
        project::{source_snapshot_chapter, source_snapshot_profile},
    };
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
    use serde_json::json;

    let db = a4n_migration::test_support::connect_test_database().await;
    let root = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "角色引用重置".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let base = json!({"explicit_attributes":{"first_seen_chapter":1,"role":"正文角色"},"implicit_attributes":{},"social_attributes":{}});
    let referenced = create_profile(&db, root.id, "快照角色", base.clone())
        .await
        .unwrap();
    let now = chrono::Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(root.id),
        chapter_number: Set(1),
        content: Set("正文".into()),
        word_count: Set(2),
        status: Set(chapter::STATUS_COMPLETED.into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let game = create_derived_project_record(
        &db,
        1,
        CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: "galgame".into(),
            title: "派生游戏".into(),
            description: String::new(),
            chapter_scope: DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    let unreferenced = create_profile(&db, root.id, "无引用角色", base.clone())
        .await
        .unwrap();
    let participant = create_profile(&db, root.id, "剧本角色", base.clone())
        .await
        .unwrap();
    let source = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(game.source_snapshot_id.unwrap()))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let script = galgame_script::ActiveModel {
        project_id: Set(game.id),
        root_project_id: Set(root.id),
        source_snapshot_id: Set(source.snapshot_id),
        title: Set("剧本".into()),
        style_config_json: Set(json!({})),
        status: Set("draft".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let scene = galgame_scene::ActiveModel {
        script_id: Set(script.id),
        source_snapshot_chapter_id: Set(source.id),
        scene_index: Set(1),
        title: Set("场景".into()),
        participants_json: Set(
            json!([{"character_profile_id":participant.id,"name":participant.name}]),
        ),
        entry_kind: Set("main".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    for profile_id in [referenced.id, participant.id] {
        assert!(
            matches!(delete_profile_cascade_transaction(&db, profile_id).await, Err(CharacterProfileDeleteError::Referenced(id)) if id == profile_id)
        );
    }
    let tx = db.begin().await.unwrap();
    let result = super::reset::reset_character_profiles_for_project(&tx, root.id, now)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(result.deleted_character_profiles, 1);
    assert_eq!(result.reset_character_profiles, 2);
    assert!(
        protagonist_profile::Entity::find_by_id(unreferenced.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    for profile_id in [referenced.id, participant.id] {
        let profile = protagonist_profile::Entity::find_by_id(profile_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(profile.last_synced_chapter, 0);
        assert_eq!(
            profile.base_attributes,
            json!({"explicit_attributes":{},"implicit_attributes":{},"social_attributes":{}})
        );
        assert_eq!(profile.current_state, profile.base_attributes);
    }
    let frozen = source_snapshot_profile::Entity::find()
        .filter(source_snapshot_profile::Column::SnapshotId.eq(source.snapshot_id))
        .filter(source_snapshot_profile::Column::ProfileKind.eq("character"))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(frozen.payload_json["base_attributes"], base);
    let mut edited: galgame_scene::ActiveModel = scene.into();
    edited.title = Set("仍可编辑".into());
    edited.update(&db).await.unwrap();
}
