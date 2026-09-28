use a4n_db::chapter_visual_plan::{
    SaveChapterVisualPlanDraftInput, save_chapter_visual_plan_draft_with_operation,
};
use a4n_domain::chapter_visual_plan::{
    ChapterVisualPlanAuthoringMode, ChapterVisualPlanV3, ExpectedVisualPlanDraft,
};

#[tokio::test]
async fn edits_replay_merge_and_reject_overlapping_or_reused_operations() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let root = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: 1,
            title: "Edit receipts".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let chapter = a4n_db::chapter_content::import_chapter_content_record(
        &db,
        a4n_domain::chapter::ImportChapterContentInput {
            project_id: root.id,
            chapter_number: 1,
            content: "走到门前。".into(),
            title: None,
        },
        "edit receipt test",
    )
    .await
    .unwrap();
    let video = a4n_db::project::create_derived_project_record(
        &db,
        1,
        a4n_db::project::CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: "video".into(),
            title: "Edit video".into(),
            description: String::new(),
            chapter_scope: a4n_db::project::DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    let source = a4n_db::project::resolve_video_chapter_source(&db, video.id, chapter.id)
        .await
        .unwrap();
    let initial = SaveChapterVisualPlanDraftInput {
        source,
        author_user_id: 1,
        authoring_mode: ChapterVisualPlanAuthoringMode::Collaborative,
        expected: None,
        expected_source_hash: None,
        plan: ChapterVisualPlanV3::starter(),
    };
    let first = save_chapter_visual_plan_draft_with_operation(&db, initial.clone(), "create")
        .await
        .unwrap();
    let replay = save_chapter_visual_plan_draft_with_operation(&db, initial.clone(), "create")
        .await
        .unwrap();
    assert_eq!(first, replay);
    let mut agent = initial.clone();
    agent.expected = Some(ExpectedVisualPlanDraft {
        record_id: first.record.id,
        revision: first.record.revision,
    });
    agent.plan.points[0].description = "Agent description".into();
    let mut manual = agent.clone();
    manual.plan = first.plan.clone();
    manual.plan.points[0].label = "Manual label".into();
    let saved_manual = save_chapter_visual_plan_draft_with_operation(&db, manual, "manual")
        .await
        .unwrap();
    let saved_agent = save_chapter_visual_plan_draft_with_operation(&db, agent.clone(), "agent")
        .await
        .unwrap();
    assert_eq!(saved_agent.plan.points[0].label, "Manual label");
    assert_eq!(saved_agent.plan.points[0].description, "Agent description");
    assert_eq!(
        saved_agent.record.revision,
        saved_manual.record.revision + 1
    );
    assert_eq!(saved_agent.edit_receipt.as_ref().unwrap().changes.len(), 1);
    assert_eq!(
        save_chapter_visual_plan_draft_with_operation(&db, agent.clone(), "agent")
            .await
            .unwrap(),
        saved_agent
    );
    agent.plan.points[0].description = "Different payload".into();
    assert!(
        save_chapter_visual_plan_draft_with_operation(&db, agent.clone(), "agent")
            .await
            .is_err()
    );
    assert!(
        save_chapter_visual_plan_draft_with_operation(&db, agent, "conflicting-edit")
            .await
            .is_err()
    );
    let latest = a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(&db, first.record.id)
        .await
        .unwrap();
    assert_eq!(latest.plan, saved_agent.plan);
    assert_eq!(latest.record.revision, saved_agent.record.revision);
    db.close().await.unwrap();
}
