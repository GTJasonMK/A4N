use a4n_db::chapter_visual_plan::{
    ExpectedVisualPlanDraft, SaveChapterVisualPlanDraftInput, chapter_visual_plan_by_id,
    save_chapter_visual_plan_draft,
};
use a4n_db_models::models::chapter::chapter;
use a4n_domain::chapter::ImportChapterContentInput;
use a4n_domain::chapter_visual_plan::{
    ChapterVisualPlanAuthoringMode, ChapterVisualPlanSchemaVersion, ChapterVisualPlanV2,
    VisualFrameV2,
};
use sea_orm::{ActiveModelTrait, Set, TransactionTrait};
use std::time::Duration;

#[tokio::test]
async fn source_precondition_is_checked_inside_the_save_transaction() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: 1,
            title: "Agent source CAS".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let chapter = a4n_db::chapter_content::import_chapter_content_record(
        &db,
        ImportChapterContentInput {
            project_id: project.id,
            chapter_number: 1,
            content: "原文版本一".into(),
            title: None,
        },
        "Agent source CAS test",
    )
    .await
    .unwrap();
    let source = a4n_db::project::resolve_chapter_source(&db, project.id, chapter.id)
        .await
        .unwrap();
    let mut input = SaveChapterVisualPlanDraftInput {
        expected_source_hash: Some(source.content_sha256.clone()),
        source,
        author_user_id: 1,
        authoring_mode: ChapterVisualPlanAuthoringMode::Collaborative,
        expected: None,
        plan: ChapterVisualPlanV2 {
            schema_version: ChapterVisualPlanSchemaVersion::V2,
            frame: VisualFrameV2 {
                width: 1920,
                height: 1080,
                background: None,
            },
            entities: Vec::new(),
            scenes: Vec::new(),
        },
    };
    let original = save_chapter_visual_plan_draft(&db, input.clone())
        .await
        .unwrap();
    input.expected = Some(ExpectedVisualPlanDraft {
        record_id: original.record.id,
        revision: original.record.revision,
    });
    assert_eq!(
        save_chapter_visual_plan_draft(&db, input.clone())
            .await
            .unwrap()
            .record
            .revision,
        original.record.revision
    );
    input.plan.frame.width = 1280;

    // Hold a real concurrent source update open. The plan save must wait for
    // it, then reject the old hash rather than publish the stale rewrite.
    let tx = db.begin().await.unwrap();
    let mut changed: chapter::ActiveModel = chapter.into();
    changed.content = Set("原文版本二".into());
    changed.update(&tx).await.unwrap();
    let save_db = db.clone();
    let mut pending =
        tokio::spawn(async move { save_chapter_visual_plan_draft(&save_db, input).await });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut pending)
            .await
            .is_err()
    );
    tx.commit().await.unwrap();
    let error = pending.await.unwrap().unwrap_err();
    assert!(error.to_string().contains("原文版本已变化"));
    let current = chapter_visual_plan_by_id(&db, original.record.id)
        .await
        .unwrap();
    assert_eq!(current.record.revision, original.record.revision);
    assert_eq!(current.plan, original.plan);
}
