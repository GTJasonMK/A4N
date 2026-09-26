use super::*;
use crate::{
    character,
    fact_feedback::*,
    project::{
        CreateProjectRecordInput, ForeshadowingAnalysisDraft,
        ProjectFamilyTextCoreRows, create_project_record, project_family_export_rows,
        sync_chapter_analysis_foreshadowing,
    },
};
use a4n_db_models::models::{
    character::{attribute_event, snapshot_reference},
    project::project_fact_feedback,
};
use a4n_domain::character::CharacterAttributeValueChangeInput;
use sea_orm::{ActiveModelTrait, PaginatorTrait, Set, TransactionTrait};
use serde_json::{Value, json};

#[tokio::test]
async fn feedback_archive_remaps_sources_events_and_snapshot_hashes() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "事实往返".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let mut chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        chapter_number: Set(1),
        title: Set(Some("初见".into())),
        content: Set("门后有人。".into()),
        word_count: Set(5),
        status: Set("draft".into()),
        analysis_data: Set(Some(
            json!({"world_facts": [], "foreshadowing": {"planted": []}}),
        )),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let foreshadowing = sync_chapter_analysis_foreshadowing(
        &db,
        crate::project::chapter_source_snapshot_from_model(&chapter),
        vec![ForeshadowingAnalysisDraft {
            existing_id: None,
            description: "门后的身份".into(),
            evidence: "门后有人".into(),
            related_characters: None,
            importance: 3,
        }],
        vec![],
    )
    .await
    .unwrap()
    .created
    .remove(0);
    chapter.analysis_data.as_mut().unwrap()["foreshadowing"]["planted"] =
        json!([{"id":foreshadowing.id.to_string()}]);
    let mut active: chapter::ActiveModel = chapter.clone().into();
    active.analysis_data = Set(chapter.analysis_data.clone());
    active.update(&db).await.unwrap();
    let profile = character::create_profile(
        &db,
        project.id,
        "林岚",
        json!({
            "explicit_attributes": {}, "implicit_attributes": {}, "social_attributes": {}
        }),
    )
    .await
    .unwrap();
    character::add_profile_attribute(
        &db,
        CharacterAttributeValueChangeInput {
            profile_id: profile.id,
            chapter_number: 1,
            category: "explicit".into(),
            key: "location".into(),
            value: json!("门外"),
            change_description: None,
            cause: "手工设定".into(),
            evidence: "手工设定".into(),
        },
    )
    .await
    .unwrap();
    let event = attribute_event::Entity::find()
        .filter(attribute_event::Column::ProfileId.eq(profile.id))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let data = json!({"project_id":project.id,"profile_id":profile.id,"synced_chapter":1,"attributes":profile.current_state,"changes":[{"id":event.id}]});
    let snapshot = snapshot_reference::ActiveModel {
        profile_id: Set(profile.id),
        chapter_number: Set(1),
        snapshot_hash: Set(a4n_domain::source_text_hash(&data.to_string())),
        reference_count: Set(1),
        data: Set(data),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let run = crate::workflow::upsert_workflow_run(
        &db,
        crate::workflow::UpsertWorkflowRunInput {
            business_key: format!("feedback-archive:{}", project.id),
            workflow_kind: "chapter_analysis".into(),
            project_id: Some(project.id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: None,
            scope_id: None,
            chapter_id: Some(chapter.id),
            mode: None,
            input_fingerprint_json: None,
            status: "running".into(),
            current_step_key: None,
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        },
    )
    .await
    .unwrap();
    update_feedback_interval(&db, project.id, 7).await.unwrap();
    let tx = db.begin().await.unwrap();
    record_feedback_completion(&tx, &chapter, run.id)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let revision = source_revision(&chapter).unwrap();
    project_fact_feedback::ActiveModel {
        project_id: Set(project.id),
        consolidated_sources_json: Set(json!([revision])),
        last_consolidated_at: Set(Some(now)),
        ..Default::default()
    }
    .update(&db)
    .await
    .unwrap();
    let exported = project_family_export_rows(&db, project.id, &[], &[])
        .await
        .unwrap();
    let mut package = ProjectFamilyImportPackage {
        root_project_id: project.id,
        projects: vec![project],
        text_core: exported.text_core,
        ..Default::default()
    };
    let imported = import_project_family_package_rows(&db, 1, &package, &mut EmptyMediaImporter)
        .await
        .unwrap()
        .remove(0);
    let current = current_feedback_sources(&db, imported.id).await.unwrap();
    assert_eq!(current.len(), 1);
    assert_ne!(current[0].chapter.id, chapter.id);
    let archive = load_feedback_archive(&db, imported.id).await.unwrap();
    assert_eq!(archive.settings.as_ref().unwrap().interval_chapters, 7);
    assert_eq!(
        archive.settings.unwrap().consolidated_sources_json,
        json!([current[0].revision])
    );
    assert_eq!(
        current[0].chapter.analysis_data.as_ref().unwrap()["foreshadowing"]["planted"][0]["id"],
        archive.foreshadowing[0].id.to_string()
    );
    assert!(
        archive
            .changes
            .iter()
            .all(|change| change.workflow_run_id.is_none())
    );
    let copied_snapshot = &archive.snapshots[0];
    assert_eq!(copied_snapshot.data["project_id"], imported.id);
    assert_eq!(
        copied_snapshot.data["profile_id"],
        copied_snapshot.profile_id
    );
    assert_eq!(
        copied_snapshot.data["changes"][0]["id"],
        archive.attribute_events[0].id
    );
    assert_eq!(
        copied_snapshot.snapshot_hash,
        a4n_domain::source_text_hash(&copied_snapshot.data.to_string())
    );
    assert_ne!(copied_snapshot.snapshot_hash, snapshot.snapshot_hash);
    assert_eq!(copied_snapshot.created_at, snapshot.created_at);

    // A package can contain saved analysis after a plain edit; it must not acquire fresh coverage on import.
    package.text_core.chapters[0].content = "门后没有人。".into();
    let stale = import_project_family_package_rows(&db, 1, &package, &mut EmptyMediaImporter)
        .await
        .unwrap()
        .remove(0);
    assert!(
        current_feedback_sources(&db, stale.id)
            .await
            .unwrap()
            .is_empty()
    );
    let changes = load_feedback_archive(&db, stale.id).await.unwrap().changes;
    assert!(
        changes.iter().any(
            |change| change.material_kind == "chapter_feedback" && change.status == "retracted"
        )
    );

    let before = novel_project::Entity::find().count(&db).await.unwrap();
    package.text_core.chapters[0]
        .analysis_data
        .as_mut()
        .unwrap()["_meta"] = json!(42);
    let error = import_project_family_package_rows(&db, 1, &package, &mut EmptyMediaImporter)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("_meta 必须是对象"));
    assert_eq!(
        novel_project::Entity::find().count(&db).await.unwrap(),
        before
    );
}

#[test]
fn legacy_package_without_feedback_uses_empty_coverage() {
    let mut legacy = serde_json::to_value(ProjectFamilyTextCoreRows::default()).unwrap();
    legacy.as_object_mut().unwrap().remove("fact_feedback");
    let decoded: ProjectFamilyTextCoreRows = serde_json::from_value(legacy).unwrap();
    assert!(decoded.fact_feedback.is_none());
    assert_eq!(
        serde_json::to_value(decoded).unwrap()["fact_feedback"],
        Value::Null
    );
}
