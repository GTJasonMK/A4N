use super::{ImportChapterOutlineUpsertInput, upsert_import_chapter_outline_checked};
use crate::project::{
    ChapterSourceSnapshot, CreateProjectRecordInput, PROJECT_TYPE_NOVEL_LONG,
    ProjectChapterOutlineUpdateInput, RetrospectiveChapterOutlineUpsertInput,
    create_project_record,
};
use a4n_core::models::chapter::{chapter, chapter_outline, chapter_outline_provenance};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

use super::super::outline_records::{
    SOURCE_KIND_IMPORT_ANALYSIS, SOURCE_KIND_RETROSPECTIVE_ANALYSIS,
    clear_chapter_outline_provenance, upsert_retrospective_chapter_outline_checked,
};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

async fn create_project(db: &DatabaseConnection) -> i64 {
    create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "导入分析保留测试".to_string(),
            description: String::new(),
            project_type: PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .expect("test project must be created")
    .id
}

async fn create_source_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> chapter::Model {
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(chapter_number),
        title: Set(Some(format!("第{chapter_number}章"))),
        content: Set("正文".to_string()),
        word_count: Set(2),
        status: Set(chapter::STATUS_IMPORTED.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("source chapter must be inserted")
}

fn retrospective_input(
    project_id: i64,
    fingerprint: &str,
    summary: &str,
) -> RetrospectiveChapterOutlineUpsertInput {
    RetrospectiveChapterOutlineUpsertInput {
        project_id,
        chapter_number: 1,
        title: "回顾标题".to_string(),
        summary: summary.to_string(),
        key_scenes: Some(serde_json::json!(["场景"])),
        foreshadowing_setup: None,
        foreshadowing_payoff: None,
        characters_involved: Some(serde_json::json!(["角色"])),
        word_count_target: 1200,
        source_fingerprint_json: serde_json::json!({"fingerprint": fingerprint}),
    }
}

#[tokio::test]
async fn retrospective_outline_preserves_legacy_outline_without_provenance() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let source_chapter = create_source_chapter(&db, project_id, 1).await;
    let now = Utc::now().naive_utc();
    let existing = chapter_outline::ActiveModel {
        project_id: Set(project_id),
        chapter_number: Set(1),
        title: Set(Some("用户章纲".to_string())),
        summary: Set(Some("用户计划".to_string())),
        status: Set(chapter_outline::STATUS_COMPLETED.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("legacy outline must be inserted");

    let result = upsert_retrospective_chapter_outline_checked(
        &db,
        retrospective_input(project_id, "new", "AI 摘要"),
        ChapterSourceSnapshot::from(&source_chapter),
    )
    .await
    .expect("legacy outline should be preserved");

    assert_eq!(result.id, existing.id);
    assert_eq!(result.title.as_deref(), Some("用户章纲"));
    assert_eq!(result.summary.as_deref(), Some("用户计划"));
    assert!(
        chapter_outline_provenance::Entity::find_by_id(existing.id)
            .one(&db)
            .await
            .expect("provenance lookup must succeed")
            .is_none()
    );
}

#[tokio::test]
async fn retrospective_outline_is_idempotent_and_refreshes_only_on_fingerprint_change() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let source_chapter = create_source_chapter(&db, project_id, 1).await;
    let first = upsert_retrospective_chapter_outline_checked(
        &db,
        retrospective_input(project_id, "same", "旧摘要"),
        ChapterSourceSnapshot::from(&source_chapter),
    )
    .await
    .expect("first retrospective outline must be inserted");
    let unchanged = upsert_retrospective_chapter_outline_checked(
        &db,
        retrospective_input(project_id, "same", "不应写入"),
        ChapterSourceSnapshot::from(&source_chapter),
    )
    .await
    .expect("same fingerprint should be a no-op");
    assert_eq!(unchanged, first);

    let refreshed = upsert_retrospective_chapter_outline_checked(
        &db,
        retrospective_input(project_id, "changed", "新摘要"),
        ChapterSourceSnapshot::from(&source_chapter),
    )
    .await
    .expect("changed fingerprint should refresh");
    assert_eq!(refreshed.id, first.id);
    assert_eq!(refreshed.summary.as_deref(), Some("新摘要"));
    let provenance = chapter_outline_provenance::Entity::find_by_id(first.id)
        .one(&db)
        .await
        .expect("provenance lookup must succeed")
        .expect("retrospective provenance must exist");
    assert_eq!(provenance.source_kind, SOURCE_KIND_RETROSPECTIVE_ANALYSIS);
    assert_eq!(
        provenance.source_fingerprint_json,
        serde_json::json!({"fingerprint": "changed"})
    );
}

#[tokio::test]
async fn retrospective_outline_preserves_non_retrospective_provenance() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let source_chapter = create_source_chapter(&db, project_id, 1).await;
    let now = Utc::now().naive_utc();
    let existing = chapter_outline::ActiveModel {
        project_id: Set(project_id),
        chapter_number: Set(1),
        title: Set(Some("导入章纲".to_string())),
        summary: Set(Some("导入摘要".to_string())),
        status: Set(chapter_outline::STATUS_COMPLETED.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("outline must be inserted");
    chapter_outline_provenance::ActiveModel {
        outline_id: Set(existing.id),
        source_kind: Set("import_analysis".to_string()),
        source_fingerprint_json: Set(serde_json::json!({"fingerprint": "old"})),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&db)
    .await
    .expect("non-retrospective provenance must be inserted");

    let result = upsert_retrospective_chapter_outline_checked(
        &db,
        retrospective_input(project_id, "new", "不应覆盖"),
        ChapterSourceSnapshot::from(&source_chapter),
    )
    .await
    .expect("non-retrospective provenance should be preserved");
    assert_eq!(result.title.as_deref(), Some("导入章纲"));
    assert_eq!(result.summary.as_deref(), Some("导入摘要"));
    let provenance = chapter_outline_provenance::Entity::find_by_id(existing.id)
        .one(&db)
        .await
        .expect("provenance lookup must succeed")
        .expect("provenance must remain");
    assert_eq!(provenance.source_kind, "import_analysis");
}

#[tokio::test]
async fn manual_outline_update_clears_retrospective_provenance() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let source_chapter = create_source_chapter(&db, project_id, 1).await;
    let outline = upsert_retrospective_chapter_outline_checked(
        &db,
        retrospective_input(project_id, "before-edit", "自动摘要"),
        ChapterSourceSnapshot::from(&source_chapter),
    )
    .await
    .expect("retrospective outline must be inserted");

    crate::project::update_chapter_outline_record(
        &db,
        ProjectChapterOutlineUpdateInput {
            project_id,
            outline_id: outline.id,
            title: Some("用户接管标题".to_string()),
            summary: Some("用户接管摘要".to_string()),
            beats: None,
            key_scenes: None,
        },
    )
    .await
    .expect("manual update must succeed");

    let refreshed = upsert_retrospective_chapter_outline_checked(
        &db,
        retrospective_input(project_id, "after-edit", "不应覆盖"),
        ChapterSourceSnapshot::from(&source_chapter),
    )
    .await
    .expect("user-owned outline should be preserved");
    assert_eq!(refreshed.title.as_deref(), Some("用户接管标题"));
    assert_eq!(refreshed.summary.as_deref(), Some("用户接管摘要"));
    assert!(
        chapter_outline_provenance::Entity::find_by_id(outline.id)
            .one(&db)
            .await
            .expect("provenance lookup must succeed")
            .is_none()
    );

    // Keep the helper exercised directly as the ownership boundary used by formal writers.
    clear_chapter_outline_provenance(&db, outline.id)
        .await
        .expect("clearing an already-cleared provenance should be idempotent");
}

#[tokio::test]
async fn import_analysis_preserves_existing_chapter_plan() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let source_chapter = create_source_chapter(&db, project_id, 3).await;
    let now = Utc::now().naive_utc();

    let chapter_outline_model = chapter_outline::ActiveModel {
        project_id: Set(project_id),
        chapter_number: Set(3),
        title: Set(Some("用户章纲".to_string())),
        summary: Set(Some("用户计划".to_string())),
        status: Set(chapter_outline::STATUS_COMPLETED.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("chapter outline must be inserted");
    let preserved_chapter_outline = upsert_import_chapter_outline_checked(
        &db,
        ImportChapterOutlineUpsertInput {
            project_id,
            chapter_number: 3,
            title: "AI 章纲".to_string(),
            summary: "AI 摘要".to_string(),
            key_scenes: None,
            foreshadowing_setup: None,
            foreshadowing_payoff: None,
            characters_involved: None,
            word_count_target: 999,
        },
        ChapterSourceSnapshot::from(&source_chapter),
    )
    .await
    .expect("chapter outline candidate must not fail");
    assert_eq!(preserved_chapter_outline, chapter_outline_model);

    assert_eq!(
        chapter_outline::Entity::find_by_id(chapter_outline_model.id)
            .one(&db)
            .await
            .expect("chapter outline lookup must succeed")
            .expect("chapter outline must remain"),
        chapter_outline_model
    );
}

#[tokio::test]
async fn import_analysis_marks_and_refreshes_its_own_outline_projection() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let source_chapter = create_source_chapter(&db, project_id, 4).await;
    let first = upsert_import_chapter_outline_checked(
        &db,
        ImportChapterOutlineUpsertInput {
            project_id,
            chapter_number: 4,
            title: "导入标题".to_string(),
            summary: "导入摘要一".to_string(),
            key_scenes: Some(serde_json::json!(["场景一"])),
            foreshadowing_setup: None,
            foreshadowing_payoff: None,
            characters_involved: None,
            word_count_target: 900,
        },
        ChapterSourceSnapshot::from(&source_chapter),
    )
    .await
    .expect("new import outline must be inserted");
    let provenance = chapter_outline_provenance::Entity::find_by_id(first.id)
        .one(&db)
        .await
        .expect("provenance lookup must succeed")
        .expect("import outline provenance must exist");
    assert_eq!(provenance.source_kind, SOURCE_KIND_IMPORT_ANALYSIS);

    let refreshed = upsert_import_chapter_outline_checked(
        &db,
        ImportChapterOutlineUpsertInput {
            project_id,
            chapter_number: 4,
            title: "导入标题".to_string(),
            summary: "导入摘要二".to_string(),
            key_scenes: Some(serde_json::json!(["场景二"])),
            foreshadowing_setup: None,
            foreshadowing_payoff: None,
            characters_involved: None,
            word_count_target: 1200,
        },
        ChapterSourceSnapshot::from(&source_chapter),
    )
    .await
    .expect("owned import outline must refresh");
    assert_eq!(refreshed.id, first.id);
    assert_eq!(refreshed.summary.as_deref(), Some("导入摘要二"));
    assert_eq!(refreshed.word_count_target, Some(1200));
}

#[tokio::test]
async fn import_analysis_repairs_stale_cross_project_chapter_outline_link() {
    let db = setup_db().await;
    let project_id = create_project(&db).await;
    let other_project_id = create_project(&db).await;
    let now = Utc::now().naive_utc();
    let stale_outline = chapter_outline::ActiveModel {
        project_id: Set(other_project_id),
        chapter_number: Set(3),
        title: Set(Some("其他项目章纲".to_string())),
        summary: Set(Some("不应被复用".to_string())),
        status: Set(chapter_outline::STATUS_COMPLETED.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("stale outline must be inserted");
    let chapter = chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(Some(stale_outline.id)),
        chapter_number: Set(3),
        title: Set(Some("第三章".to_string())),
        content: Set("正文".to_string()),
        word_count: Set(2),
        status: Set(chapter::STATUS_IMPORTED.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("chapter with stale link must be inserted");

    let repaired_outline = upsert_import_chapter_outline_checked(
        &db,
        ImportChapterOutlineUpsertInput {
            project_id,
            chapter_number: 3,
            title: "导入章纲".to_string(),
            summary: "导入摘要".to_string(),
            key_scenes: None,
            foreshadowing_setup: None,
            foreshadowing_payoff: None,
            characters_involved: None,
            word_count_target: 999,
        },
        ChapterSourceSnapshot::from(&chapter),
    )
    .await
    .expect("import outline should repair stale link");

    let refreshed_chapter = chapter::Entity::find_by_id(chapter.id)
        .one(&db)
        .await
        .expect("chapter lookup must succeed")
        .expect("chapter must remain");
    assert_eq!(refreshed_chapter.outline_id, Some(repaired_outline.id));
}
