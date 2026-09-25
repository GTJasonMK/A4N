use a4n_db::{
    character::*,
    project::{CreateProjectRecordInput, create_project_record},
};
use a4n_db_models::models::{
    chapter::chapter,
    character::{
        attribute_event, character_portrait, protagonist_deletion_mark, protagonist_profile,
    },
};
use a4n_domain::character::CharacterAttributeValueChangeInput;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde_json::json;

#[tokio::test]
async fn reanalysis_rewinds_observations_without_deleting_identity_or_manual_records() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "重分析保留资料".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        chapter_number: Set(1),
        content: Set("林岚站在门口。".into()),
        word_count: Set(8),
        status: Set("draft".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let profile = create_profile(&db, project.id, "林岚", json!({
        "explicit_attributes": {"first_seen_chapter":1}, "implicit_attributes": {}, "social_attributes": {}
    })).await.unwrap();
    add_profile_attribute(
        &db,
        CharacterAttributeValueChangeInput {
            profile_id: profile.id,
            chapter_number: 1,
            category: "explicit".into(),
            key: "location".into(),
            value: json!("手工地点"),
            change_description: None,
            cause: "作者设定".into(),
            evidence: "作者设定".into(),
        },
    )
    .await
    .unwrap();
    let observed = attribute_event::ActiveModel {
        profile_id: Set(profile.id),
        chapter_number: Set(1),
        source_kind: Set("text_fact".into()),
        timestamp: Set(now),
        category: Set("explicit".into()),
        key: Set("location".into()),
        operation: Set("modify".into()),
        old_value: Set(json!("手工地点")),
        new_value: Set(json!("门口")),
        change_description: Set("到达门口".into()),
        cause: Set("正文".into()),
        evidence: Set("林岚站在门口".into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let mut current_state = profile.base_attributes.clone();
    current_state["explicit_attributes"]["location"] = json!("门口");
    protagonist_profile::ActiveModel {
        id: Set(profile.id),
        current_state: Set(current_state),
        last_synced_chapter: Set(1),
        ..Default::default()
    }
    .update(&db)
    .await
    .unwrap();
    let portrait = character_portrait::ActiveModel {
        project_id: Set(project.id),
        profile_id: Set(profile.id),
        portrait_type: Set("face".into()),
        image_path: Set("portraits/author.png".into()),
        prompt: Set("作者立绘".into()),
        is_primary: Set(true),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    for kind in ["user", "ai_sync"] {
        protagonist_deletion_mark::ActiveModel {
            profile_id: Set(profile.id),
            source_kind: Set(kind.into()),
            category: Set("implicit".into()),
            key: Set(kind.into()),
            chapter_number: Set(1),
            last_marked_chapter: Set(1),
            consecutive_count: Set(1),
            is_executed: Set(false),
            reason: Set("待复核".into()),
            evidence: Set("原文依据".into()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
    }
    let tx = db.begin().await.unwrap();
    let artifacts = a4n_chapter_content_core::cleanup_chapter_derived_data_in(
        &tx, project.id, chapter.id, 1, false, false, None,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert!(artifacts.stale_profile_ids.is_empty() && artifacts.media_file_paths.is_empty());
    assert_eq!(artifacts.stale_attribute_event_ids, [observed.id]);
    let restored = profile_by_id(&db, profile.id).await.unwrap().unwrap();
    assert_eq!(restored.last_synced_chapter, 0);
    assert_eq!(
        restored.current_state["explicit_attributes"]["location"],
        "手工地点"
    );
    assert!(
        character_portrait::Entity::find_by_id(portrait.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    let events = attribute_event::Entity::find()
        .filter(attribute_event::Column::ProfileId.eq(profile.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].source_kind, "user");
    let marks = deletion_marks_by_profile(
        &db,
        profile.id,
        None,
        a4n_domain::character::DeletionMarkStatus::All,
    )
    .await
    .unwrap();
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].source_kind, "user");
}
