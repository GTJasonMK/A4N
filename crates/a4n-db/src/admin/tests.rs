use std::error::Error;

use a4n_core::models::task::task_queue;
use a4n_core::models::user::{invite_code, llm_usage_log, theme_config, user};
use chrono::{NaiveDate, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, Set};

use super::overview::find_active_global_theme;
use super::resources::{admin_user_summary, invite_code_summary, priority_rank};
use super::usage::{usage_trends_since, validate_positive_i64};
use super::{AdminDbError, admin_overview_summary};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

#[test]
fn database_operation_error_keeps_context_and_db_source() {
    let error = AdminDbError::DatabaseOperation {
        operation: "读取分析失败",
        source: DbErr::Custom("query failed".to_string()),
    };

    assert_eq!(
        error.to_string(),
        "读取分析失败: Custom Error: query failed"
    );
    assert!(error.source().is_some());
}

#[test]
fn usage_trends_since_rejects_zero_days() {
    let now = NaiveDate::from_ymd_opt(2026, 6, 5)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let err = usage_trends_since(now, 0).expect_err("zero days must be rejected");
    assert_eq!(err.to_string(), "days 必须大于 0");
}

#[test]
fn usage_trends_since_rejects_u64_to_i64_overflow() {
    let now = NaiveDate::from_ymd_opt(2026, 6, 5)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let err = usage_trends_since(now, i64::MAX as u64 + 1)
        .expect_err("u64 days above i64::MAX must not wrap to a negative duration");
    assert_eq!(err.to_string(), "days 超出可支持范围");
}

#[test]
fn usage_trends_since_subtracts_requested_days() {
    let now = NaiveDate::from_ymd_opt(2026, 6, 5)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let expected = NaiveDate::from_ymd_opt(2026, 5, 29)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    assert_eq!(usage_trends_since(now, 7).unwrap(), expected);
}

#[test]
fn positive_i64_validation_rejects_zero_and_negative_ids() {
    assert!(validate_positive_i64(1, "user_id").is_ok());
    assert_eq!(
        validate_positive_i64(0, "user_id")
            .expect_err("zero id should fail")
            .to_string(),
        "user_id 必须大于 0"
    );
    assert_eq!(
        validate_positive_i64(-7, "project_id")
            .expect_err("negative id should fail")
            .to_string(),
        "project_id 必须大于 0"
    );
}

#[test]
fn priority_rank_orders_semantic_task_priorities() {
    let mut priorities = ["urgent", "low", "medium", "high"];
    priorities.sort_by_key(|priority| priority_rank(priority));
    assert_eq!(priorities, ["high", "medium", "low", "urgent"]);
}

#[test]
fn admin_user_summary_uses_username_as_display_name() {
    let now = NaiveDate::from_ymd_opt(2026, 1, 2)
        .unwrap()
        .and_hms_opt(3, 4, 5)
        .unwrap();
    let user = user::Model {
        id: 7,
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        password_hash: "hash".to_string(),
        role: "admin".to_string(),
        oauth_provider: Some("github".to_string()),
        oauth_subject: Some("42".to_string()),
        avatar_url: Some("https://example.com/avatar.png".to_string()),
        is_active: true,
        created_at: now,
        updated_at: now,
        last_login_at: Some(now),
    };

    let summary = admin_user_summary(&user);

    assert_eq!(summary.id, 7);
    assert_eq!(summary.username, "alice");
    assert_eq!(summary.display_name.as_deref(), Some("alice"));
    assert_eq!(
        summary.last_login_at.as_deref(),
        Some("2026-01-02 03:04:05")
    );
}

#[test]
fn invite_code_summary_preserves_usage_metadata() {
    let now = NaiveDate::from_ymd_opt(2026, 1, 2)
        .unwrap()
        .and_hms_opt(3, 4, 5)
        .unwrap();
    let used_at = NaiveDate::from_ymd_opt(2026, 1, 3)
        .unwrap()
        .and_hms_opt(6, 7, 8)
        .unwrap();
    let code = invite_code::Model {
        id: 11,
        code: "INVITE".to_string(),
        created_by_user_id: Some(1),
        used_by_user_id: Some(2),
        created_at: now,
        used_at: Some(used_at),
    };

    let summary = invite_code_summary(&code);

    assert_eq!(summary.id, 11);
    assert_eq!(summary.code, "INVITE");
    assert_eq!(summary.created_by_user_id, Some(1));
    assert_eq!(summary.used_by_user_id, Some(2));
    assert_eq!(summary.created_at.as_deref(), Some("2026-01-02 03:04:05"));
    assert_eq!(summary.used_at.as_deref(), Some("2026-01-03 06:07:08"));
}

async fn insert_theme(
    db: &DatabaseConnection,
    name: &str,
    user_id: Option<i64>,
    is_system_default: bool,
    is_active: bool,
) -> theme_config::Model {
    let now = NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    theme_config::ActiveModel {
        user_id: Set(user_id),
        name: Set(name.to_string()),
        mode: Set("dark".to_string()),
        css_variables: Set(serde_json::json!({})),
        is_system_default: Set(is_system_default),
        is_active: Set(is_active),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn insert_task(db: &DatabaseConnection, id: &str) {
    let now = NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    task_queue::Entity::insert(task_queue::ActiveModel {
        id: Set(id.to_string()),
        task_type: Set("llm_call".to_string()),
        priority: Set("medium".to_string()),
        status: Set("pending".to_string()),
        project_id: Set(None),
        root_project_id: Set(None),
        source_snapshot_id: Set(None),
        task_scope: Set(None),
        workflow_run_id: Set(None),
        payload: Set(None),
        progress: Set(None),
        error_message: Set(None),
        dag_id: Set(None),
        created_at: Set(now),
        started_at: Set(None),
        completed_at: Set(None),
    })
    .exec(db)
    .await
    .unwrap();
}

async fn insert_llm_usage_log(db: &DatabaseConnection, id: i64) {
    let now = Utc::now().naive_utc();
    llm_usage_log::Entity::insert(llm_usage_log::ActiveModel {
        id: Set(id),
        user_id: Set(1),
        project_id: Set(None),
        model: Set("gpt-4o".to_string()),
        operation: Set("chat".to_string()),
        prompt_tokens: Set(10),
        completion_tokens: Set(20),
        total_tokens: Set(30),
        latency_ms: Set(100),
        is_success: Set(true),
        error_message: Set(None),
        created_at: Set(now),
    })
    .exec(db)
    .await
    .unwrap();
}

#[tokio::test]
async fn active_theme_summary_uses_active_global_theme_not_system_default() {
    let db = setup_db().await;
    let system_default = insert_theme(&db, "system-default-but-inactive", None, true, false).await;
    let active_global = insert_theme(&db, "active-global", None, false, true).await;
    let active_user = insert_theme(&db, "active-user", Some(7), false, true).await;

    let summary = find_active_global_theme(&db)
        .await
        .expect("active theme query should succeed")
        .expect("global active theme should exist");

    assert_eq!(summary.id, active_global.id);
    assert_ne!(summary.id, system_default.id);
    assert_ne!(summary.id, active_user.id);
}

#[tokio::test]
async fn admin_overview_reports_task_queue_and_real_llm_usage_counts_separately() {
    let db = setup_db().await;
    insert_task(&db, "task-a").await;
    insert_task(&db, "task-b").await;
    insert_llm_usage_log(&db, 1).await;
    let active_theme = insert_theme(&db, "active-global", None, false, true).await;

    let overview = admin_overview_summary(&db)
        .await
        .expect("admin overview should be computed");

    assert_eq!(overview.task_queue_count, 2);
    assert_eq!(overview.total_llm_calls, 1);
    assert_eq!(
        overview.active_theme.as_ref().map(|theme| theme.id),
        Some(active_theme.id)
    );
    assert_eq!(
        overview
            .active_theme
            .as_ref()
            .map(|theme| theme.name.as_str()),
        Some("active-global")
    );
    assert_eq!(overview.daily_active_users, 1);
}
