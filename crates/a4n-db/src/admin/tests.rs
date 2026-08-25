use std::error::Error;

use a4n_core::models::task::task_queue;
use a4n_core::models::user::{llm_usage_log, theme_config};
use chrono::{Duration, NaiveDate, NaiveDateTime, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, Set};

use super::overview::find_active_global_theme;
use super::resources::{all_project_summaries, priority_rank};
use super::usage::{token_summary, usage_trends, usage_trends_since, validate_positive_i64};
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
        theme_document: Set(serde_json::json!({
            "version": 1,
            "tokens": {},
            "backgrounds": {}
        })),
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
    insert_llm_usage_log_at(db, id, 1, Utc::now().naive_utc(), 10, 20, 30, true).await;
}

async fn insert_llm_usage_log_at(
    db: &DatabaseConnection,
    id: i64,
    user_id: i64,
    created_at: NaiveDateTime,
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
    is_success: bool,
) {
    llm_usage_log::Entity::insert(llm_usage_log::ActiveModel {
        id: Set(id),
        user_id: Set(user_id),
        project_id: Set(None),
        model: Set("gpt-4o".to_string()),
        operation: Set("chat".to_string()),
        prompt_tokens: Set(prompt_tokens),
        completion_tokens: Set(completion_tokens),
        total_tokens: Set(total_tokens),
        latency_ms: Set(100),
        is_success: Set(is_success),
        error_message: Set(None),
        created_at: Set(created_at),
    })
    .exec(db)
    .await
    .unwrap();
}

#[tokio::test]
async fn usage_aggregates_preserve_daily_and_token_contracts() {
    let db = setup_db().await;
    let now = Utc::now().naive_utc();
    insert_llm_usage_log_at(&db, 1, 7, now - Duration::hours(1), 10, 20, 30, true).await;
    insert_llm_usage_log_at(&db, 2, 7, now - Duration::hours(2), 5, 6, 11, false).await;
    insert_llm_usage_log_at(&db, 3, 7, now - Duration::hours(26), 100, 200, 300, true).await;
    insert_llm_usage_log_at(&db, 4, 8, now - Duration::hours(1), 99, 99, 198, true).await;

    let summary = token_summary(&db, 7)
        .await
        .expect("token summary query should succeed");
    assert_eq!(summary["total_calls"], 3);
    assert_eq!(summary["total_prompt_tokens"], 115);
    assert_eq!(summary["total_completion_tokens"], 226);
    assert_eq!(summary["total_tokens"], 341);
    assert_eq!(summary["success_count"], 2);
    assert_eq!(summary["fail_count"], 1);

    let empty_summary = token_summary(&db, 99)
        .await
        .expect("empty token summary query should succeed");
    assert_eq!(empty_summary["total_calls"], 0);
    assert_eq!(empty_summary["total_tokens"], 0);

    let trends = usage_trends(&db, super::UsageTrendsInput { days: 2 })
        .await
        .expect("usage trends query should succeed");
    assert_eq!(trends.len(), 2);
    let today = now.format("%Y-%m-%d").to_string();
    let yesterday = (now - Duration::days(1)).format("%Y-%m-%d").to_string();
    let today_point = trends
        .iter()
        .find(|point| point.date == today)
        .expect("today aggregate should exist");
    assert_eq!(today_point.total_calls, 3);
    assert_eq!(today_point.total_tokens, 239);
    assert_eq!(today_point.success_count, 2);
    assert_eq!(today_point.fail_count, 1);
    assert_eq!(trends[0].date, yesterday);
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

#[tokio::test]
async fn project_summaries_keep_contract_fields_with_narrow_rows() {
    let db = setup_db().await;
    let project = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "管理员项目摘要".to_string(),
            description: "不应进入摘要投影".to_string(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();

    let summaries = all_project_summaries(&db)
        .await
        .expect("admin project summaries should be readable");
    let summary = summaries
        .iter()
        .find(|summary| summary.id == project.id)
        .expect("created project summary should be returned");
    assert_eq!(summary.title, "管理员项目摘要");
    assert_eq!(summary.user_id, 1);
    assert_eq!(summary.current_chapter, project.current_chapter);
}
