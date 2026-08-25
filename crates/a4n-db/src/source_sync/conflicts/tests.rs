use super::{SourceSyncConflictRecordInput, insert_source_sync_conflict_records};
use crate::source_sync::SourceSyncDbError;
use a4n_core::models::{
    project::{novel_project, project_source_snapshot},
    source_sync::{source_sync_conflict, source_sync_decision, source_sync_run},
};
use a4n_domain::source_sync::{
    SourceSyncChangeKind, SourceSyncConflictSeverity, SourceSyncLocalStateKind,
    SourceSyncResolutionStrategy,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::json;

fn source_sync_run_model() -> source_sync_run::Model {
    let now = Utc::now().naive_utc();
    source_sync_run::Model {
        id: 1,
        derived_project_id: 2,
        root_project_id: 3,
        from_snapshot_id: 4,
        to_snapshot_id: 5,
        mode: "dry_run".to_string(),
        status: "planned".to_string(),
        summary_json: json!({}),
        error_message: None,
        created_at: now,
        updated_at: now,
    }
}

fn conflict_input() -> SourceSyncConflictRecordInput {
    SourceSyncConflictRecordInput {
        conflict_scope: "generation_spec".to_string(),
        target_kind: "segment".to_string(),
        target_id: 6,
        source_change_kind: SourceSyncChangeKind::Updated,
        local_state_kind: SourceSyncLocalStateKind::Draft,
        severity: SourceSyncConflictSeverity::Blocking,
        source_summary_json: json!({}),
        local_summary_json: json!({}),
        affected_refs_json: json!({}),
    }
}

#[tokio::test]
async fn conflict_insert_rejects_invalid_run_before_database_access() {
    let mut run = source_sync_run_model();
    run.id = 0;

    let error = insert_source_sync_conflict_records(
        &DatabaseConnection::Disconnected,
        &run,
        vec![conflict_input()],
    )
    .await
    .expect_err("invalid sync run id must fail before database access");

    assert!(matches!(
        error,
        SourceSyncDbError::InvalidArgument(message) if message == "sync_run_id 必须为正数"
    ));
}

#[tokio::test]
async fn auto_apply_batches_info_conflict_updates_and_decisions() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let user = crate::auth::create_user(
        &db,
        "source_sync_auto_apply",
        "source_sync_auto_apply@example.invalid",
        "source-sync-auto-apply-password",
        "user",
    )
    .await
    .expect("测试用户应可创建");
    let root = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: user.id,
            title: "源同步自动应用根项目".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("根项目应可创建");
    let now = Utc::now().naive_utc();
    let snapshot = project_source_snapshot::ActiveModel {
        root_project_id: Set(root.id),
        source_version_hash: Set("source-sync-auto-apply".to_string()),
        blueprint_version_id: Set(None),
        chapter_scope_json: Set(json!({"start_chapter": 1, "end_chapter": 1})),
        chapter_hashes_json: Set(json!([])),
        profile_hashes_json: Set(json!({})),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("源快照应可创建");
    let derived = novel_project::ActiveModel {
        user_id: Set(user.id),
        title: Set("源同步自动应用派生项目".to_string()),
        description: Set(String::new()),
        project_type: Set(root.project_type.clone()),
        project_kind: Set(crate::project::PROJECT_KIND_VIDEO.to_string()),
        root_project_id: Set(Some(root.id)),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_sync_status: Set(crate::project::SOURCE_SYNC_CURRENT.to_string()),
        status: Set("draft".to_string()),
        target_chapters: Set(1),
        current_chapter: Set(0),
        total_chapters: Set(Some(1)),
        chapters_per_part: Set(root.chapters_per_part),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        raw_blueprint: Set(None),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("派生项目应可创建");
    let run = crate::source_sync::insert_source_sync_dry_run_record(
        &db,
        crate::source_sync::CreateSourceSyncDryRunRecordInput {
            derived_project_id: derived.id,
            root_project_id: root.id,
            from_snapshot_id: snapshot.id,
            to_snapshot_id: snapshot.id,
            chapter_change_count: 0,
            profile_change_count: 0,
        },
    )
    .await
    .expect("源同步运行应可创建");
    let conflicts = insert_source_sync_conflict_records(
        &db,
        &run,
        [101, 102]
            .into_iter()
            .map(|target_id| SourceSyncConflictRecordInput {
                conflict_scope: "chapter".to_string(),
                target_kind: "chapter".to_string(),
                target_id,
                source_change_kind: SourceSyncChangeKind::Updated,
                local_state_kind: SourceSyncLocalStateKind::Clean,
                severity: SourceSyncConflictSeverity::Info,
                source_summary_json: json!({"target_id": target_id}),
                local_summary_json: json!({}),
                affected_refs_json: json!({}),
            })
            .collect(),
    )
    .await
    .expect("info 冲突应可批量创建");
    assert_eq!(
        conflicts
            .iter()
            .map(|conflict| conflict.target_id)
            .collect::<Vec<_>>(),
        [101, 102]
    );
    let conflict_ids = conflicts
        .iter()
        .map(|conflict| conflict.id)
        .collect::<Vec<_>>();

    let loaded = super::source_sync_conflicts_for_decisions(
        &db,
        run.id,
        &[conflict_ids[0], conflict_ids[0]],
        "测试批量读取源同步冲突",
    )
    .await
    .expect("待处理冲突应可批量读取");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[&conflict_ids[0]].sync_run_id, run.id);

    assert_eq!(
        super::apply_unconflicted_source_sync_records(&db, &run)
            .await
            .expect("无阻塞冲突应可批量自动应用"),
        2
    );
    let conflicts = source_sync_conflict::Entity::find()
        .filter(source_sync_conflict::Column::SyncRunId.eq(run.id))
        .order_by_asc(source_sync_conflict::Column::Id)
        .all(&db)
        .await
        .expect("应可读取已处理冲突");
    assert_eq!(conflicts.len(), 2);
    assert!(conflicts.iter().all(|conflict| {
        conflict.status == "resolved"
            && conflict.resolution_strategy
                == Some(SourceSyncResolutionStrategy::KeepLocal.as_str().to_string())
            && conflict
                .decision_json
                .as_ref()
                .is_some_and(|payload| payload["system_action"] == "apply_unconflicted_item")
    }));
    let decisions = source_sync_decision::Entity::find()
        .all(&db)
        .await
        .expect("应可读取自动应用决策");
    assert_eq!(decisions.len(), 2);
    assert!(decisions.iter().all(|decision| {
        decision.decided_by == "system"
            && decision.strategy == SourceSyncResolutionStrategy::KeepLocal.as_str()
            && decision.decision_payload_json["system_action"] == "apply_unconflicted_item"
    }));
}
