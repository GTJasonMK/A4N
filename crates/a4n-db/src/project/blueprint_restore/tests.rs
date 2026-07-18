use super::fields::RestoredBlueprintFields;
use super::snapshots::parse_blueprint_relationship_drafts;
use super::*;
use crate::project::PROJECT_KIND_TEXT;
use a4n_core::models::project::{
    blueprint_character, blueprint_relationship, project_status_history,
};
use chrono::{Duration, NaiveDate};
use sea_orm::QueryOrder;
use serde_json::json;
use std::error::Error;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

fn test_time(offset_seconds: i64) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        + Duration::seconds(offset_seconds)
}

fn valid_restore_snapshot() -> Value {
    json!({
        "title": "星潮档案",
        "genre": "科幻奇幻",
        "needs_part_outlines": false,
        "total_chapters": 30,
        "chapters_per_part": 25
    })
}

#[test]
fn restored_blueprint_fields_rejects_invalid_total_chapters() {
    let mut snapshot = valid_restore_snapshot();
    snapshot["total_chapters"] = json!(0);

    let err = RestoredBlueprintFields::from_snapshot(&snapshot, "1.0")
        .expect_err("invalid total_chapters must fail before restore");

    assert!(matches!(
        err,
        BlueprintRestoreError::InvalidSnapshot(message)
            if message.contains("5 到 10000")
    ));
}

#[test]
fn restored_blueprint_fields_rejects_non_positive_chapters_per_part() {
    let mut snapshot = valid_restore_snapshot();
    snapshot["chapters_per_part"] = json!(-1);

    let err = RestoredBlueprintFields::from_snapshot(&snapshot, "1.0")
        .expect_err("invalid chapters_per_part must fail before restore");

    assert!(matches!(
        err,
        BlueprintRestoreError::InvalidSnapshot(message)
            if message.contains("chapters_per_part 必须为正整数")
    ));
}

#[test]
fn parse_blueprint_relationship_drafts_rejects_strength_out_of_range() {
    let err = parse_blueprint_relationship_drafts(&json!([{
        "relationship_type": "friend",
        "character_a": "林澈",
        "character_b": "许灯",
        "strength": 11
    }]))
    .expect_err("relationship strength outside 1..=10 must fail");

    assert!(matches!(
        err,
        BlueprintRestoreError::InvalidSnapshot(message)
            if message.contains("strength 必须在 1 到 10 之间")
    ));
}

#[test]
fn blueprint_restore_error_preserves_nested_sources() {
    let database = BlueprintRestoreError::database(
        "读取蓝图版本失败",
        DbErr::Custom("query failed".to_string()),
    );
    assert!(matches!(
        database,
        BlueprintRestoreError::Database {
            operation: "读取蓝图版本失败",
            source: DbErr::Custom(ref message),
        } if message == "query failed"
    ));
    assert!(database.source().is_some());

    let character_sync = BlueprintRestoreError::CharacterSync {
        source: BlueprintSyncError::Database(DbErr::Custom("sync failed".to_string())),
    };
    assert!(character_sync.source().is_some());

    let status_write = BlueprintRestoreError::StatusWrite {
        source: ProjectStatusWriteError::Database {
            source: DbErr::Custom("history failed".to_string()),
        },
    };
    assert!(status_write.source().is_some());

    let serialization = BlueprintRestoreError::Serialization {
        source: serde_json::from_str::<Value>("{").unwrap_err(),
    };
    assert!(serialization.source().is_some());
}

#[tokio::test]
async fn blueprint_restore_validates_arguments_before_database_access() {
    let db = DatabaseConnection::Disconnected;

    assert!(matches!(
        restore_blueprint_version_record(&db, 0, "v1").await,
        Err(BlueprintRestoreError::InvalidArgument(message))
            if message == "project_id 必须大于 0"
    ));
    assert!(matches!(
        restore_blueprint_version_record(&db, 1, " ").await,
        Err(BlueprintRestoreError::InvalidArgument(message))
            if message == "版本号不能为空"
    ));
    let error = restore_blueprint_version_record(&db, 1, "v1")
        .await
        .expect_err("disconnected database must fail when starting the transaction");
    assert!(matches!(
        error,
        BlueprintRestoreError::Database {
            operation: "启动蓝图版本恢复事务失败",
            ..
        }
    ));
    assert!(error.source().is_some());
}

#[tokio::test]
async fn restore_blueprint_version_record_restores_rows_and_project_status() {
    let db = setup_db().await;
    let project_id = 8301;
    let old_time = test_time(0);
    novel_project::ActiveModel {
        id: Set(project_id),
        user_id: Set(77),
        title: Set("旧项目".to_string()),
        description: Set(String::new()),
        project_type: Set("novel_long".to_string()),
        project_kind: Set(PROJECT_KIND_TEXT.to_string()),
        source_mode: Set("self".to_string()),
        root_project_id: Set(None),
        parent_project_id: Set(None),
        source_snapshot_id: Set(None),
        source_sync_status: Set("current".to_string()),
        derived_project_status: Set(None),
        status: Set("draft".to_string()),
        target_chapters: Set(10),
        current_chapter: Set(1),
        total_chapters: Set(Some(10)),
        chapters_per_part: Set(None),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        raw_blueprint: Set(None),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(old_time),
        updated_at: Set(old_time),
    }
    .insert(&db)
    .await
    .unwrap();
    novel_blueprint::ActiveModel {
        project_id: Set(project_id),
        title: Set("旧蓝图".to_string()),
        genre: Set("旧类型".to_string()),
        current_version: Set("v1".to_string()),
        needs_part_outlines: Set(false),
        total_chapters: Set(Some(10)),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let kept_character = blueprint_character::ActiveModel {
        project_id: Set(project_id),
        name: Set("林烬".to_string()),
        character_type: Set("protagonist".to_string()),
        personality: Set(Some("旧性格".to_string())),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    blueprint_character::ActiveModel {
        project_id: Set(project_id),
        name: Set("旧角色".to_string()),
        character_type: Set("supporting".to_string()),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let blueprint_data = json!({
        "title": "星潮档案",
        "genre": "科幻奇幻",
        "style": "冷峻",
        "tone": "悬疑",
        "target_audience": "成人",
        "one_sentence_summary": "调查星潮真相",
        "full_synopsis": "完整主线",
        "world_setting": {"premise": "星潮复苏"},
        "needs_part_outlines": false,
        "total_chapters": 30,
        "chapters_per_part": 25,
        "characters": [
            {
                "name": "林烬",
                "character_type": "protagonist",
                "personality": "克制",
                "arc_stages": {"start": "逃避"}
            },
            {
                "name": "沈澜",
                "character_type": "antagonist"
            }
        ],
        "relationships": [
            {
                "relationship_type": "rival",
                "character_a": "林烬",
                "character_b": "沈澜",
                "description": "互相试探",
                "strength": 7
            }
        ]
    });
    blueprint_version::ActiveModel {
        project_id: Set(project_id),
        version: Set("v2".to_string()),
        change_description: Set(Some("恢复测试".to_string())),
        blueprint_data: Set(blueprint_data.clone()),
        world_setting_snapshot: Set(blueprint_data.get("world_setting").cloned()),
        characters_snapshot: Set(blueprint_data.get("characters").cloned()),
        relationships_snapshot: Set(blueprint_data.get("relationships").cloned()),
        synopsis_snapshot: Set(Some("完整主线".to_string())),
        created_at: Set(old_time),
        created_by: Set(Some("tester".to_string())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let result = restore_blueprint_version_record(&db, project_id, " v2 ")
        .await
        .unwrap();

    assert_eq!(result.version, "v2");
    assert_eq!(result.blueprint.title, "星潮档案");
    assert_eq!(result.blueprint.current_version, "v2");
    assert_eq!(result.blueprint.total_chapters, Some(30));
    let status_change = result.status_change.expect("draft should advance");
    assert_eq!(status_change.from, "draft");
    assert_eq!(status_change.to, "blueprint_ready");

    let project = novel_project::Entity::find_by_id(project_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(project.title, "星潮档案");
    assert_eq!(project.status, "blueprint_ready");
    assert_eq!(project.target_chapters, 30);
    assert_eq!(project.total_chapters, Some(30));
    assert!(
        project
            .raw_blueprint
            .as_deref()
            .is_some_and(|raw| raw.contains("\"title\": \"星潮档案\""))
    );

    let characters = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_character::Column::Id)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(characters.len(), 2);
    let restored_character = characters.iter().find(|row| row.name == "林烬").unwrap();
    assert_eq!(restored_character.id, kept_character.id);
    assert_eq!(restored_character.created_at, old_time);
    assert_eq!(restored_character.personality.as_deref(), Some("克制"));
    assert!(characters.iter().any(|row| row.name == "沈澜"));
    assert!(!characters.iter().any(|row| row.name == "旧角色"));

    let relationships = blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(relationships.len(), 1);
    assert_eq!(relationships[0].relationship_type, "rival");
    assert_eq!(relationships[0].strength, Some(7));

    let history = project_status_history::Entity::find()
        .filter(project_status_history::Column::ProjectId.eq(project_id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].from_status, "draft");
    assert_eq!(history[0].to_status, "blueprint_ready");
    assert_eq!(history[0].reason.as_deref(), Some("蓝图版本恢复"));

    let failed_blueprint_data = json!({
        "title": "不应提交的蓝图",
        "genre": "测试",
        "needs_part_outlines": false,
        "total_chapters": 40,
        "characters": [
            {"name": "重复角色", "character_type": "supporting"},
            {"name": " 重复角色 ", "character_type": "supporting"}
        ]
    });
    blueprint_version::ActiveModel {
        project_id: Set(project_id),
        version: Set("v3".to_string()),
        blueprint_data: Set(failed_blueprint_data.clone()),
        characters_snapshot: Set(failed_blueprint_data.get("characters").cloned()),
        created_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let error = restore_blueprint_version_record(&db, project_id, "v3")
        .await
        .expect_err("duplicate normalized character identity must abort restore");
    assert!(matches!(
        error,
        BlueprintRestoreError::CharacterSync {
            source: BlueprintSyncError::InvalidArgument(message),
        } if message.contains("蓝图角色重复")
    ));

    let blueprint_after_failure = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(blueprint_after_failure.title, "星潮档案");
    assert_eq!(blueprint_after_failure.current_version, "v2");

    let project_after_failure = novel_project::Entity::find_by_id(project_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(project_after_failure.title, "星潮档案");
    assert_eq!(project_after_failure.status, "blueprint_ready");

    let characters_after_failure = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(characters_after_failure.len(), 2);
    assert!(
        characters_after_failure
            .iter()
            .all(|row| row.name != "重复角色")
    );

    let history_after_failure = project_status_history::Entity::find()
        .filter(project_status_history::Column::ProjectId.eq(project_id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(history_after_failure.len(), 1);
}
