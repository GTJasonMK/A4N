use a4n_db::{
    blueprint::{
        SaveManualBlueprintInput, UpdateBlueprintWorldSettingInput,
        save_manual_blueprint_records_transaction,
        update_blueprint_world_setting_records_transaction,
    },
    fact_feedback::*,
    project::{CreateProjectRecordInput, create_project_record},
};
use a4n_db_models::models::{
    chapter::chapter,
    project::{material_fact_change, novel_blueprint, novel_project},
};
use a4n_domain::fact_feedback::{
    FactChangeOperation, FactMaterialChange, FactMaterialKind, pending_fact_sources,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set, TransactionTrait,
};
use serde_json::{Value, json};

async fn project(db: &DatabaseConnection) -> i64 {
    create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "记忆之城".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap()
    .id
}

async fn workflow(db: &DatabaseConnection, project_id: i64, key: &str) -> i64 {
    a4n_db::workflow::upsert_workflow_run(
        db,
        a4n_db::workflow::UpsertWorkflowRunInput {
            business_key: format!("fact-test:{project_id}:{key}"),
            workflow_kind: "project_fact_consolidation".into(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("novel_project".into()),
            scope_id: Some(project_id),
            chapter_id: None,
            mode: None,
            input_fingerprint_json: Some(json!({"key":key})),
            status: "running".into(),
            current_step_key: Some("project_fact_consolidation".into()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        },
    )
    .await
    .unwrap()
    .id
}

async fn chapter(
    db: &DatabaseConnection,
    project_id: i64,
    number: i32,
    text: &str,
) -> chapter::Model {
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project_id), chapter_number: Set(number), title: Set(Some(format!("第 {number} 章"))),
        content: Set(text.into()), word_count: Set(text.chars().count() as i32), status: Set("draft".into()),
        analysis_data: Set(Some(json!({
            "metadata":{"characters":[],"locations":[],"items":[],"tags":[],"tone":"平静","timeline_marker":null},
            "summaries":{"compressed":text,"one_line":text,"keywords":[]}, "character_states":{},
            "key_events":[],"foreshadowing":{"planted":[],"resolved":[],"tensions":[],"progressed":[]},
            "world_facts":[{"field":"core_rules","subject":"记忆规则","statement":text,"assertion":"observed","source_span_ids":[format!("ch{number}:s1:f1")]}]
        }))), created_at: Set(now), updated_at: Set(now), ..Default::default()
    }.insert(db).await.unwrap()
}

async fn blueprint(
    db: &DatabaseConnection,
    project_id: i64,
    world: Value,
) -> novel_blueprint::Model {
    save_manual_blueprint_records_transaction(
        db,
        SaveManualBlueprintInput {
            project_id,
            genre: "奇幻".into(),
            style: None,
            tone: None,
            target_audience: None,
            one_sentence_summary: None,
            full_synopsis: Some("未来剧情由作者决定".into()),
            total_chapters: 20,
            created_by: "user:1".into(),
        },
    )
    .await
    .unwrap();
    update_blueprint_world_setting_records_transaction(
        db,
        UpdateBlueprintWorldSettingInput {
            project_id,
            world_setting: world,
            created_by: "user:1".into(),
        },
    )
    .await
    .unwrap()
    .blueprint
}

#[tokio::test]
async fn partial_feedback_is_versioned_source_fenced_and_reversible() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let id = project(&db).await;
    let mut sources = Vec::new();
    for number in 1..=5 {
        let chapter = chapter(&db, id, number, "施法会损失一段记忆。").await;
        let run = workflow(&db, id, &format!("chapter-{number}")).await;
        let tx = db.begin().await.unwrap();
        record_feedback_completion(&tx, &chapter, run)
            .await
            .unwrap();
        record_feedback_completion(&tx, &chapter, run)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        sources.push(source_revision(&chapter).unwrap());
    }
    assert_eq!(current_feedback_sources(&db, id).await.unwrap().len(), 5);
    assert_eq!(pending_fact_sources(&sources, &[]).len(), 5);
    update_feedback_interval(&db, id, 3).await.unwrap();
    assert_eq!(feedback_interval(&db, id).await.unwrap(), 3);
    assert!(update_feedback_interval(&db, id, 0).await.is_err());
    let original = blueprint(&db, id, json!({"core_rules":[],"factions":["旧城议会"]})).await;
    novel_project::ActiveModel {
        id: Set(id),
        title: Set("作者修改的书名".into()),
        target_chapters: Set(64),
        total_chapters: Set(Some(64)),
        chapters_per_part: Set(Some(8)),
        ..Default::default()
    }
    .update(&db)
    .await
    .unwrap();
    let run = workflow(&db, id, "apply").await;
    let result = apply_consolidation(
        &db,
        ConsolidationApplyInput {
            project_id: id,
            workflow_run_id: run,
            expected_blueprint_version: Some(original.current_version.clone()),
            sources: sources.clone(),
            changes: vec![FactMaterialChange {
                material_kind: FactMaterialKind::WorldSetting,
                target_id: Some(original.id),
                path: "/core_rules/-".into(),
                operation: FactChangeOperation::Add,
                before: Value::Null,
                after: json!("施法会损失一段记忆"),
                reason: "正文明确说明代价".into(),
                source_span_ids: vec!["ch5:s1:f1".into()],
            }],
        },
    )
    .await
    .unwrap();
    assert!(result.blueprint_version.is_some());
    let updated = novel_blueprint::Entity::find_by_id(original.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        updated.world_setting.as_ref().unwrap()["factions"],
        json!(["旧城议会"])
    );
    assert_eq!(updated.full_synopsis.as_deref(), Some("未来剧情由作者决定"));
    let project = novel_project::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(project.title, "作者修改的书名");
    assert_eq!(project.target_chapters, 64);
    assert_eq!(project.total_chapters, Some(64));
    assert_eq!(project.chapters_per_part, Some(8));
    let covered: Vec<a4n_domain::fact_feedback::FactSourceRevision> = serde_json::from_value(
        feedback_config(&db, id)
            .await
            .unwrap()
            .consolidated_sources_json,
    )
    .unwrap();
    assert!(pending_fact_sources(&sources, &covered).is_empty());
    assert_eq!(
        world_setting_before_chapter(&db, &updated, 5)
            .await
            .unwrap()["core_rules"],
        json!([])
    );
    assert_eq!(
        world_setting_before_chapter(&db, &updated, 6)
            .await
            .unwrap()["core_rules"],
        json!(["施法会损失一段记忆"])
    );

    let no_change = apply_consolidation(
        &db,
        ConsolidationApplyInput {
            project_id: id,
            workflow_run_id: workflow(&db, id, "no-change").await,
            expected_blueprint_version: Some(updated.current_version.clone()),
            sources: sources.clone(),
            changes: vec![],
        },
    )
    .await
    .unwrap();
    assert!(no_change.blueprint_version.is_none());
    let mut changed = chapter::Entity::find_by_id(sources[4].chapter_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .into_active_model();
    changed.content = Set("施法不再损失记忆。".into());
    changed.update(&db).await.unwrap();
    let error = apply_consolidation(
        &db,
        ConsolidationApplyInput {
            project_id: id,
            workflow_run_id: workflow(&db, id, "stale").await,
            expected_blueprint_version: Some(updated.current_version.clone()),
            sources,
            changes: vec![],
        },
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("已变化"));
    let tx = db.begin().await.unwrap();
    assert!(
        invalidate_feedback_sources(&tx, id, 5)
            .await
            .unwrap()
            .blueprint_changed
    );
    tx.commit().await.unwrap();
    let restored = novel_blueprint::Entity::find_by_id(original.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(restored.world_setting.unwrap()["core_rules"], json!([]));
    assert!(
        !material_fact_change::Entity::find()
            .filter(material_fact_change::Column::ProjectId.eq(id))
            .filter(material_fact_change::Column::Status.eq("retracted"))
            .all(&db)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn foreshadowing_keeps_identity_tracks_progress_and_retracts_removed_sources() {
    use a4n_db::project::{
        ChapterSourceSnapshot, ForeshadowingAnalysisDraft, ForeshadowingFactUpdate,
        sync_chapter_analysis_foreshadowing,
    };
    use a4n_db_models::models::character::foreshadowing_index;
    use a4n_domain::character::ForeshadowingStatus;
    let db = a4n_migration::test_support::connect_test_database().await;
    let id = project(&db).await;
    let setup = chapter(&db, id, 1, "门后有人。").await;
    let draft = |description: &str| ForeshadowingAnalysisDraft {
        existing_id: None,
        evidence: "门后有人".into(),
        description: description.into(),
        related_characters: Some(json!([])),
        importance: 3,
    };
    let first = sync_chapter_analysis_foreshadowing(
        &db,
        ChapterSourceSnapshot::from(&setup),
        vec![draft("门后的疑影")],
        vec![],
    )
    .await
    .unwrap();
    let foreshadowing_id = first.created[0].id;
    let repeated = sync_chapter_analysis_foreshadowing(
        &db,
        ChapterSourceSnapshot::from(&setup),
        vec![draft("谁躲在门后")],
        vec![],
    )
    .await
    .unwrap();
    assert!(repeated.created.is_empty());
    assert_eq!(repeated.current[0].id, foreshadowing_id);
    let reveal = chapter(&db, id, 2, "门后的人戴着父亲的帽子。").await;
    let result = sync_chapter_analysis_foreshadowing(
        &db,
        ChapterSourceSnapshot::from(&reveal),
        vec![],
        vec![ForeshadowingFactUpdate {
            id: foreshadowing_id,
            status: ForeshadowingStatus::PartiallyRevealed,
            reason: "出现身份线索，尚未完全揭示".into(),
            evidence: vec!["门后的人戴着父亲的帽子".into()],
        }],
    )
    .await
    .unwrap();
    assert_eq!(result.updated[0].status, "partially_revealed");
    assert!(result.updated[0].payoff_chapter.is_none());
    let tx = db.begin().await.unwrap();
    invalidate_feedback_sources(&tx, id, 2).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        foreshadowing_index::Entity::find_by_id(foreshadowing_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .status,
        "pending"
    );
    let tx = db.begin().await.unwrap();
    invalidate_feedback_sources(&tx, id, 1).await.unwrap();
    tx.commit().await.unwrap();
    assert!(
        foreshadowing_index::Entity::find_by_id(foreshadowing_id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn consolidation_rejects_unproven_or_invalidated_sources_and_keeps_plans_as_proposals() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let id = project(&db).await;
    let mut source = chapter(&db, id, 1, "据说施法会损失记忆。").await;
    source.analysis_data.as_mut().unwrap()["world_facts"][0]["assertion"] = json!("reported");
    let mut active: chapter::ActiveModel = source.clone().into();
    active.analysis_data = Set(source.analysis_data.clone());
    active.update(&db).await.unwrap();
    let run = workflow(&db, id, "reported").await;
    let tx = db.begin().await.unwrap();
    record_feedback_completion(&tx, &source, run).await.unwrap();
    tx.commit().await.unwrap();
    let original = blueprint(&db, id, json!({"core_rules":[]})).await;
    let mut change = FactMaterialChange {
        material_kind: FactMaterialKind::WorldSetting,
        target_id: Some(original.id),
        path: "/core_rules/-".into(),
        operation: FactChangeOperation::Add,
        before: Value::Null,
        after: json!("施法损失记忆"),
        reason: "本章提到代价".into(),
        source_span_ids: vec!["ch1:s1:f1".into()],
    };
    let error = apply_consolidation(
        &db,
        ConsolidationApplyInput {
            project_id: id,
            workflow_run_id: run,
            expected_blueprint_version: Some(original.current_version.clone()),
            sources: vec![source_revision(&source).unwrap()],
            changes: vec![change.clone()],
        },
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("传闻和假设不能写回"));
    assert!(feedback_changes(&db, id).await.unwrap().is_empty());

    change.material_kind = FactMaterialKind::BlueprintSynopsis;
    change.path = String::new();
    change.operation = FactChangeOperation::Replace;
    change.before = json!("未来剧情由作者决定");
    change.after = json!("建议调整未来剧情");
    let proposed = apply_consolidation(
        &db,
        ConsolidationApplyInput {
            project_id: id,
            workflow_run_id: run,
            expected_blueprint_version: Some(original.current_version.clone()),
            sources: vec![source_revision(&source).unwrap()],
            changes: vec![change],
        },
    )
    .await
    .unwrap();
    assert!(proposed.blueprint_version.is_none());
    assert_eq!(proposed.changes[0].status, "proposal");
    assert_eq!(
        novel_blueprint::Entity::find_by_id(original.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .full_synopsis,
        original.full_synopsis
    );

    let tx = db.begin().await.unwrap();
    invalidate_feedback_sources(&tx, id, 1).await.unwrap();
    tx.commit().await.unwrap();
    let error = apply_consolidation(
        &db,
        ConsolidationApplyInput {
            project_id: id,
            workflow_run_id: workflow(&db, id, "invalidated").await,
            expected_blueprint_version: Some(original.current_version),
            sources: vec![source_revision(&source).unwrap()],
            changes: vec![],
        },
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("已失效"));
}

#[tokio::test]
async fn reapplying_withdrawn_facts_preserves_history_and_new_application_order() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let id = project(&db).await;
    let chapter = chapter(&db, id, 1, "施法会损失记忆。").await;
    let input = |after: &str| FactChangeRecordInput {
        project_id: id,
        workflow_run_id: None,
        material_kind: "planning".into(),
        target_id: None,
        field_path: String::new(),
        operation: "add".into(),
        before: Value::Null,
        after: json!(after),
        sources: vec![source_revision(&chapter).unwrap()],
        evidence: json!([{"chapter_number":1,"quote":chapter.content}]),
        reason: "依据正文调整规划建议".into(),
        status: "proposal".into(),
    };
    let first = record_fact_change(&db, input("建议甲")).await.unwrap();
    let tx = db.begin().await.unwrap();
    invalidate_feedback_sources(&tx, id, 1).await.unwrap();
    tx.commit().await.unwrap();
    let intervening = record_fact_change(&db, input("建议乙")).await.unwrap();
    let reapplied = record_fact_change(&db, input("建议甲")).await.unwrap();
    assert!(reapplied.id > intervening.id);
    assert_eq!(
        material_fact_change::Entity::find_by_id(first.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .status,
        "retracted"
    );
    assert_eq!(
        record_fact_change(&db, input("建议甲")).await.unwrap().id,
        reapplied.id
    );
}
