use super::*;
use crate::DatabaseConnection;
use a4n_db_models::models::chapter::chapter;
use a4n_test_support::workflow::WorkflowRunModelBuilder;
use sea_orm::IntoActiveModel;

#[tokio::test]
async fn latest_chapter_runs_match_existing_selection_and_keep_scope() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let mut projects = Vec::new();
    for title in ["章节查询", "其他项目"] {
        projects.push(
            crate::project::create_project_record(
                &db,
                crate::project::CreateProjectRecordInput {
                    user_id: 1,
                    title: title.into(),
                    description: String::new(),
                    project_type: "novel_long".into(),
                },
            )
            .await
            .unwrap()
            .id,
        );
    }
    let now = Utc::now().naive_utc();
    let mut chapters = Vec::new();
    for (project_id, number) in [
        (projects[0], 1),
        (projects[0], 2),
        (projects[0], 3),
        (projects[1], 1),
    ] {
        chapters.push(
            chapter::ActiveModel {
                project_id: Set(project_id),
                chapter_number: Set(number),
                title: Set(Some("章节".into())),
                content: Set("正文".into()),
                word_count: Set(2),
                status: Set("draft".into()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&db)
            .await
            .unwrap()
            .id,
        );
    }
    let mut ids = Vec::new();
    for (index, (project_id, chapter_id, kind, time)) in [
        (
            projects[0],
            Some(chapters[0]),
            "chapter_analysis",
            now - chrono::Duration::hours(1),
        ),
        (projects[0], Some(chapters[0]), "chapter_analysis", now),
        (projects[0], Some(chapters[0]), "chapter_analysis", now),
        (projects[0], Some(chapters[1]), "chapter_analysis", now),
        (projects[0], Some(chapters[2]), "chapter_analysis", now),
        (projects[0], Some(chapters[0]), "chapter_content_sync", now),
        (projects[0], None, "chapter_analysis", now),
        (projects[1], Some(chapters[3]), "chapter_analysis", now),
    ]
    .into_iter()
    .enumerate()
    {
        let mut model =
            WorkflowRunModelBuilder::new(1, format!("scope-test:{index}"), kind, "completed", time)
                .with_project_scope(Some(project_id), None)
                .with_input_fingerprint(json!({"marker": index}))
                .with_progress(json!({"snapshot": {"keep": index}}))
                .build();
        model.chapter_id = chapter_id;
        model.completed_at = Some(time);
        let mut active = model.into_active_model();
        active.id = NotSet;
        ids.push(active.insert(&db).await.unwrap().id);
    }
    let selected = latest_workflow_runs_for_chapters(
        &db,
        projects[0],
        "chapter_analysis",
        &[chapters[0], chapters[1], chapters[3]],
    )
    .await
    .unwrap();
    assert_eq!(
        selected.iter().map(|run| run.id).collect::<Vec<_>>(),
        vec![ids[2], ids[3]]
    );
    let summaries = latest_workflow_run_summaries_for_chapters(
        &crate::DbHandle::new(std::sync::Arc::new(db.clone())),
        projects[0],
        "chapter_analysis",
        &[chapters[0], chapters[1], chapters[3]],
    )
    .await
    .unwrap();
    for (summary, run) in summaries.iter().zip(&selected) {
        assert_eq!(summary.id, run.id);
        assert_eq!(summary.chapter_id, run.chapter_id);
        assert_eq!(summary.status, run.status);
        assert_eq!(summary.input_fingerprint_json, run.input_fingerprint_json);
    }
    assert_eq!(summaries.len(), selected.len());
    let history = workflow_runs_by_project_kind(&db, projects[0], "chapter_analysis")
        .await
        .unwrap();
    for selected_run in &selected {
        let original = history
            .iter()
            .find(|run| run.chapter_id == selected_run.chapter_id)
            .unwrap();
        assert_eq!(
            selected_run, original,
            "keep every field of the same selected run"
        );
    }
    assert_eq!(history.len(), 6, "the history API must remain complete");
}

#[tokio::test]
async fn latest_chapter_runs_validate_inputs_without_database_access() {
    let db = DatabaseConnection::Disconnected;
    assert!(
        latest_workflow_runs_for_chapters(&db, 1, "chapter_analysis", &[])
            .await
            .unwrap()
            .is_empty()
    );
    for (project, kind, chapters) in [
        (0, "chapter_analysis", vec![1]),
        (1, "", vec![1]),
        (1, "chapter_analysis", vec![0]),
    ] {
        assert!(matches!(
            latest_workflow_runs_for_chapters(&db, project, kind, &chapters).await,
            Err(WorkflowReadError::InvalidArgument(_))
        ));
    }
}
