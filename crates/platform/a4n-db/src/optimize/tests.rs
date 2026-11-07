use super::*;
use a4n_db_models::models::{chapter::chapter, project::novel_project};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set, TransactionTrait};

async fn setup_db() -> DatabaseConnection {
    let db = a4n_migration::test_support::connect_test_database().await;
    a4n_test_support::database::ensure_test_user(&db, 1).await;
    db
}

#[tokio::test]
async fn empty_patch_set_returns_none_without_database_access() {
    let result = create_optimization_patch_set(
        &DatabaseConnection::Disconnected,
        OptimizationPatchSetCreateInput {
            turn_id: 1,
            session_id: 2,
            chapter_id: 3,
            base_content_hash: "hash".to_string(),
            status: "pending".to_string(),
            summary: "summary".to_string(),
            patch_decision: "pending".to_string(),
            patch_apply_status: "pending".to_string(),
            patches: Vec::new(),
        },
    )
    .await
    .expect("empty patch sets should be rejected before database access");
    assert!(result.is_none());
}

#[test]
fn optimize_database_error_keeps_variant_and_message() {
    let error = OptimizeDbError::from(DbErr::Custom("query failed".to_string()));
    assert!(matches!(
        error,
        OptimizeDbError::Database(DbErr::Custom(ref message)) if message == "query failed"
    ));
    assert_eq!(error.to_string(), "Custom Error: query failed");
}

#[tokio::test]
async fn turn_completion_and_analysis_retry_preserve_session_state() {
    let db = setup_db().await;
    let project = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "优化事务测试".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("章节正文".to_string()),
        word_count: Set(4),
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
    .unwrap();
    let session = create_optimization_session(
        &db,
        OptimizationSessionCreateInput {
            project_id: project.id,
            chapter_id: chapter.id,
            name: Some("对话".to_string()),
            status: "waiting_user".to_string(),
            issues: None,
            agent_permissions: None,
            agent_instruction: None,
            fixes_applied: 0,
            summary: None,
        },
    )
    .await
    .unwrap();
    let turn = create_optimization_turn(
        &db,
        OptimizationTurnCreateInput {
            session_id: session.id,
            agent_turn_id: None,
            user_text: "继续优化".to_string(),
            status: "running".to_string(),
            permissions_snapshot: None,
        },
    )
    .await
    .unwrap();

    let tx = db.begin().await.unwrap();
    let completed =
        complete_optimization_chat_turn(&tx, session.id, turn.id, "  继续优化  ", "  已完成优化  ")
            .await
            .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(completed.status, "completed");
    assert_eq!(completed.final_reply.as_deref(), Some("已完成优化"));
    let messages = optimization_session_messages(&db, session.id)
        .await
        .unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(
        (messages[0].role.as_str(), messages[0].content.as_str()),
        ("user", "继续优化")
    );
    assert_eq!(
        (messages[1].role.as_str(), messages[1].content.as_str()),
        ("assistant", "已完成优化"),
    );

    let tx = db.begin().await.unwrap();
    let duplicate =
        complete_optimization_chat_turn(&tx, session.id, turn.id, "继续优化", "已完成优化")
            .await
            .unwrap_err();
    tx.rollback().await.unwrap();
    assert!(matches!(
        duplicate,
        OptimizeDbError::InvalidArgument(message)
            if message == "优化 Turn 当前状态不允许完成: completed"
    ));
    assert_eq!(
        optimization_session_messages(&db, session.id)
            .await
            .unwrap()
            .len(),
        2,
    );
    let resumed = resume_optimization_analysis_session(&db, session.id)
        .await
        .unwrap();
    assert_eq!(resumed.status, "analyzing");
    assert!(
        resume_optimization_analysis_session(&db, session.id)
            .await
            .is_err()
    );
    let mut cancelled: optimization_session::ActiveModel = resumed.into();
    cancelled.status = Set("cancelled".into());
    cancelled.update(&db).await.unwrap();
    let resumed = resume_optimization_analysis_session(&db, session.id)
        .await
        .unwrap();
    assert_eq!(resumed.status, "analyzing");
    assert_eq!(
        optimization_session_messages(&db, session.id)
            .await
            .unwrap()
            .len(),
        2
    );
    let mut fixing: optimization_session::ActiveModel = resumed.into();
    fixing.status = Set("fixing".into());
    let fixing = fixing.update(&db).await.unwrap();
    assert!(
        resume_optimization_analysis_session(&db, session.id)
            .await
            .is_err()
    );
    let mut archived: optimization_session::ActiveModel = fixing.into();
    archived.status = Set("waiting_user".into());
    archived.archived_at = Set(Some(now));
    archived.update(&db).await.unwrap();
    assert!(
        resume_optimization_analysis_session(&db, session.id)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn optimization_session_creation_rejects_project_during_reset() {
    let db = setup_db().await;
    let project = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "重置时优化会话创建".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("章节正文".to_string()),
        word_count: Set(4),
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
    .unwrap();
    let mut active: novel_project::ActiveModel = novel_project::Entity::find_by_id(project.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .into();
    active.lifecycle_gate = Set("resetting".to_string());
    active.update(&db).await.unwrap();

    let error = create_optimization_session(
        &db,
        OptimizationSessionCreateInput {
            project_id: project.id,
            chapter_id: chapter.id,
            name: Some("重置中的会话".to_string()),
            status: "waiting_user".to_string(),
            issues: None,
            agent_permissions: None,
            agent_instruction: None,
            fixes_applied: 0,
            summary: None,
        },
    )
    .await
    .expect_err("reset gate must reject a new optimization session");
    assert!(matches!(error, OptimizeDbError::InvalidState { .. }));
    assert!(
        optimization_sessions(&db, project.id, None)
            .await
            .unwrap()
            .is_empty()
    );
}
