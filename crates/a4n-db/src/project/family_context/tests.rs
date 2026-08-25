use super::manga_sources::snapshot_chapter_manga_source;
use super::{
    contracts::ProjectFamilyDbError, derived_project_family_access_ids,
    project_or_derived_family_access_ids,
};
use a4n_core::models::{chapter::chapter, project::source_snapshot_chapter};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use uuid::Uuid;

fn snapshot_chapter(content: &str) -> source_snapshot_chapter::Model {
    source_snapshot_chapter::Model {
        id: 1,
        snapshot_id: 2,
        root_project_id: 3,
        source_chapter_id: 4,
        chapter_number: 5,
        title: Some("标题".to_string()),
        content: content.to_string(),
        summary: Some("摘要".to_string()),
        analysis_data_json: None,
        content_hash: "content-hash".to_string(),
        analysis_hash: None,
        visual_plan_id: None,
        visual_plan_version: None,
        visual_plan_sha256: None,
    }
}

#[test]
fn snapshot_manga_source_requires_content_when_requested() {
    let error = snapshot_chapter_manga_source(snapshot_chapter("  "), true)
        .expect_err("blank snapshot content must be rejected");

    assert!(matches!(
        error,
        ProjectFamilyDbError::InvalidArgument(message)
            if message == "源快照章节正文为空，无法生成整页漫画提示词"
    ));

    let source = snapshot_chapter_manga_source(snapshot_chapter("  "), false)
        .expect("content is optional for metadata-only reads");
    assert_eq!(source.chapter_id, 4);
    assert_eq!(source.chapter_number, 5);
    assert_eq!(source.status, "completed");
}

#[tokio::test]
async fn derived_family_access_ids_keep_relationship_contract() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let user = crate::auth::create_user(
        &db,
        &format!("family_access_{suffix}"),
        &format!("family_access_{suffix}@example.invalid"),
        "family-access-password",
        "user",
    )
    .await
    .expect("测试用户应可创建");
    let root = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: user.id,
            title: format!("族访问根项目 {suffix}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("根项目应可创建");
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(root.id),
        chapter_number: Set(1),
        content: Set("族访问测试正文".to_string()),
        word_count: Set(7),
        status: Set(chapter::STATUS_IMPORTED.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("根项目章节应可创建");
    let derived = crate::project::create_derived_project_record(
        &db,
        user.id,
        crate::project::CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: crate::project::PROJECT_KIND_VIDEO.to_string(),
            title: format!("族访问派生项目 {suffix}"),
            description: String::new(),
            chapter_scope: crate::project::DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .expect("派生项目应可创建");

    assert_eq!(
        derived_project_family_access_ids(&db, derived.id)
            .await
            .expect("窄访问读取应成功"),
        (derived.id, root.id)
    );
    assert_eq!(
        project_or_derived_family_access_ids(&db, derived.id)
            .await
            .expect("项目族访问读取应成功"),
        (derived.id, Some(root.id))
    );
    assert_eq!(
        project_or_derived_family_access_ids(&db, root.id)
            .await
            .expect("文本项目访问读取应成功"),
        (root.id, None)
    );
}
