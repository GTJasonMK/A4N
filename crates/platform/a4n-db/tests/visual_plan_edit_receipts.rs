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
    let same_effect =
        save_chapter_visual_plan_draft_with_operation(&db, agent.clone(), "same-effect")
            .await
            .unwrap();
    assert_eq!(same_effect.record.revision, saved_agent.record.revision);
    assert!(
        same_effect
            .edit_receipt
            .as_ref()
            .unwrap()
            .changes
            .is_empty()
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
    assert_eq!(
        a4n_db::chapter_visual_plan::visual_plan_edit_cursor(&db, Some(&first))
            .await
            .unwrap(),
        first.edit_receipt.as_ref().unwrap().sequence
    );
    assert_eq!(
        a4n_db::chapter_visual_plan::visual_plan_edit_cursor(&db, Some(&latest))
            .await
            .unwrap(),
        same_effect.edit_receipt.as_ref().unwrap().sequence
    );
    let replacement = a4n_db::project::create_derived_project_record(
        &db,
        1,
        a4n_db::project::CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: "video".into(),
            title: "Replacement snapshot".into(),
            description: String::new(),
            chapter_scope: a4n_db::project::DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    use sea_orm::{ActiveModelTrait, Set};
    let mut moved: a4n_db_models::models::project::novel_project::ActiveModel = video.into();
    moved.source_snapshot_id = Set(replacement.source_snapshot_id);
    moved.update(&db).await.unwrap();
    let mut stale_source = initial;
    stale_source.expected = Some(ExpectedVisualPlanDraft {
        record_id: latest.record.id,
        revision: latest.record.revision,
    });
    stale_source.plan = latest.plan.clone();
    stale_source.plan.points[0].label = "Must not write retired source".into();
    assert!(
        save_chapter_visual_plan_draft_with_operation(&db, stale_source, "stale-source")
            .await
            .is_err()
    );
    assert!(
        a4n_db::chapter_visual_plan::record_visual_plan_noop(
            &db,
            &latest,
            "stale-noop",
            "stale-noop"
        )
        .await
        .is_err()
    );
    assert_eq!(
        a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(&db, latest.record.id)
            .await
            .unwrap()
            .plan,
        latest.plan
    );
    db.close().await.unwrap();
}
