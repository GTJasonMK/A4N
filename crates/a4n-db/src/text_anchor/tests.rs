use super::*;
use a4n_core::models::{
    asset::object_profile,
    chapter::chapter,
    project::{novel_project, profile_evidence, source_snapshot_chapter, text_anchor},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};
use serde_json::json;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

async fn text_project(db: &DatabaseConnection, title: &str) -> novel_project::Model {
    crate::project::create_project_record(
        db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: title.to_string(),
            description: String::new(),
            project_type: crate::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap()
}

async fn chapter_record(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
    content: &str,
) -> chapter::Model {
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(chapter_number),
        title: Set(Some(format!("第{chapter_number}章"))),
        content: Set(content.to_string()),
        word_count: Set(content.chars().count() as i32),
        status: Set(chapter::STATUS_IMPORTED.to_string()),
        summary: Set(Some("章节摘要".to_string())),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn video_project(
    db: &DatabaseConnection,
    root_project_id: i64,
    title: &str,
) -> novel_project::Model {
    crate::project::create_derived_project_record(
        db,
        1,
        crate::project::CreateDerivedProjectParams {
            root_project_id,
            derived_kind: crate::project::PROJECT_KIND_VIDEO.to_string(),
            title: title.to_string(),
            description: String::new(),
            chapter_scope: crate::project::DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap()
}

async fn object_profile_record(db: &DatabaseConnection, project_id: i64) -> object_profile::Model {
    let now = Utc::now().naive_utc();
    object_profile::ActiveModel {
        project_id: Set(project_id),
        name: Set("青铜钥匙".to_string()),
        normalized_name: Set("青铜钥匙".to_string()),
        aliases_json: Set(json!([])),
        object_type: Set("token".to_string()),
        importance: Set("primary".to_string()),
        visual_description: Set("旧青铜钥匙".to_string()),
        material_and_state: Set("氧化青铜".to_string()),
        story_usage: Set("开启旧门".to_string()),
        visual_usage: Set("保持钥匙作为开门动作道具的画面连续性".to_string()),
        generation_usage: Set("约束旧青铜钥匙外观和氧化材质".to_string()),
        location_context: Set(Some("抽屉".to_string())),
        first_seen_chapter: Set(1),
        last_seen_chapter: Set(1),
        evidence_json: Set(json!(["他在抽屉里摸到一枚青铜钥匙"])),
        image_prompt_seed: Set("一枚旧青铜钥匙".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

#[tokio::test]
async fn rebuild_current_chapter_anchors_replaces_existing_rows() {
    let db = setup_db().await;
    let project = text_project(&db, "当前正文锚点").await;
    let chapter = chapter_record(
        &db,
        project.id,
        1,
        "他走进旧书房，抽屉里有一枚青铜钥匙。钥匙很冷。",
    )
    .await;

    let first = rebuild_chapter_content_anchor_records(&db, project.id, chapter.id)
        .await
        .unwrap();
    let object = object_profile_record(&db, project.id).await;
    merge_profile_evidence_records(
        &db,
        project.id,
        None,
        TextAnchorProfileKind::Object,
        object.id,
        vec![TextAnchorEvidenceInput {
            text_anchor_id: first[0].id,
            evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
        }],
    )
    .await
    .unwrap();
    let second = rebuild_chapter_content_anchor_records(&db, project.id, chapter.id)
        .await
        .unwrap();

    assert_eq!(first.len(), 3);
    assert_eq!(second.len(), 3);
    assert_eq!(
        text_anchor::Entity::find()
            .filter(text_anchor::Column::ProjectId.eq(project.id))
            .count(&db)
            .await
            .unwrap(),
        3
    );
    assert!(
        second
            .iter()
            .any(|anchor| anchor.quote == "抽屉里有一枚青铜钥匙")
    );
    assert_eq!(
        profile_evidence::Entity::find()
            .filter(profile_evidence::Column::ProfileId.eq(object.id))
            .count(&db)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn source_snapshot_creation_builds_chapter_anchors_with_snapshot_identity() {
    let db = setup_db().await;
    let root = text_project(&db, "快照锚点").await;
    chapter_record(&db, root.id, 1, "他推开门，风从门缝里灌进来。").await;
    let derived = video_project(&db, root.id, "视频项目").await;
    let snapshot_id = derived.source_snapshot_id.unwrap();
    let snapshot_chapter = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    let result = text_anchor::Entity::find()
        .filter(text_anchor::Column::ProjectId.eq(root.id))
        .filter(text_anchor::Column::SourceSnapshotId.eq(snapshot_id))
        .filter(text_anchor::Column::SourceSnapshotChapterId.eq(snapshot_chapter.id))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(result.len(), 2);
    assert!(
        result
            .iter()
            .all(|anchor| anchor.source_snapshot_id == Some(snapshot_id))
    );
    assert!(
        result
            .iter()
            .all(|anchor| anchor.source_snapshot_chapter_id == Some(snapshot_chapter.id))
    );
}

#[tokio::test]
async fn validate_anchor_refs_rejects_anchor_from_another_project() {
    let db = setup_db().await;
    let first_project = text_project(&db, "锚点项目 A").await;
    let second_project = text_project(&db, "锚点项目 B").await;
    let chapter = chapter_record(&db, first_project.id, 1, "他看见青铜钥匙。").await;
    let anchor = rebuild_chapter_content_anchor_records(&db, first_project.id, chapter.id)
        .await
        .unwrap()
        .remove(0);

    let error = validate_anchor_ref_records(&db, second_project.id, None, &[], &[anchor.id])
        .await
        .expect_err("cross-project anchor must fail");

    assert!(matches!(
        error,
        TextAnchorDbError::InvalidArgument(message) if message.contains("不属于文本根项目")
    ));
}

#[tokio::test]
async fn merge_profile_evidence_persists_anchor_binding() {
    let db = setup_db().await;
    let project = text_project(&db, "档案证据锚点").await;
    let chapter = chapter_record(&db, project.id, 1, "他在抽屉里摸到一枚青铜钥匙。").await;
    let object = object_profile_record(&db, project.id).await;
    let anchor = rebuild_chapter_content_anchor_records(&db, project.id, chapter.id)
        .await
        .unwrap()
        .remove(0);

    let evidence = merge_profile_evidence_records(
        &db,
        project.id,
        None,
        TextAnchorProfileKind::Object,
        object.id,
        vec![TextAnchorEvidenceInput {
            text_anchor_id: anchor.id,
            evidence_role: "first_seen".to_string(),
        }],
    )
    .await
    .unwrap();

    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].profile_kind, "object");
    assert_eq!(evidence[0].profile_id, object.id);
    assert_eq!(evidence[0].text_anchor_id, anchor.id);
}

#[tokio::test]
async fn earliest_profile_evidence_chapter_requires_the_current_chapter_source() {
    let db = setup_db().await;
    let project = text_project(&db, "档案首次出现章节").await;
    let chapter = chapter_record(&db, project.id, 7, "他在站台上举起青铜钥匙。").await;
    let object = object_profile_record(&db, project.id).await;
    let anchor = rebuild_chapter_content_anchor_records(&db, project.id, chapter.id)
        .await
        .unwrap()
        .remove(0);
    merge_profile_evidence_records(
        &db,
        project.id,
        None,
        TextAnchorProfileKind::Object,
        object.id,
        vec![TextAnchorEvidenceInput {
            text_anchor_id: anchor.id,
            evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
        }],
    )
    .await
    .unwrap();

    assert_eq!(
        earliest_current_profile_evidence_chapter(
            &db,
            project.id,
            TextAnchorProfileKind::Object,
            object.id,
        )
        .await
        .unwrap(),
        Some(7)
    );

    let mut stale_chapter: chapter::ActiveModel = chapter.into();
    stale_chapter.content = Set("正文已经改变".to_string());
    stale_chapter.update(&db).await.unwrap();
    assert_eq!(
        earliest_current_profile_evidence_chapter(
            &db,
            project.id,
            TextAnchorProfileKind::Object,
            object.id,
        )
        .await
        .unwrap(),
        None
    );
}

#[tokio::test]
async fn rebuild_current_chapter_anchors_rejects_derived_project() {
    let db = setup_db().await;
    let root = text_project(&db, "派生项目锚点拒绝").await;
    chapter_record(&db, root.id, 1, "他推开门。").await;
    let derived = video_project(&db, root.id, "视频项目").await;

    let error = rebuild_chapter_content_anchor_records(&db, derived.id, 1)
        .await
        .expect_err("derived project must fail");

    assert!(matches!(
        error,
        TextAnchorDbError::InvalidArgument(message) if message.contains("不是文本根项目")
    ));
}
