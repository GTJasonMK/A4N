use a4n_db::agent::{
    create_thread, create_turn_with_run, executions_by_runs, inbox_by_thread,
    latest_events_by_thread, list_memory, messages_by_thread, run_budget_snapshot, runs_by_thread,
    thread_by_id, thread_history, turns_by_thread,
};
use a4n_db_models::models::agent::{run, turn};
use a4n_domain::agent::{AgentMessageInput, AgentThreadCreateInput, AgentTurnInput};
use chrono::Utc;
use sea_orm::{AccessMode, ActiveModelTrait, EntityTrait, IsolationLevel, Set, TransactionTrait};
use serde_json::json;
use std::time::Instant;

const RUN_COUNT: usize = 32;
const SAMPLES: usize = 15;

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * percentile).round() as usize]
}

async fn history_fixture(db: &sea_orm::DatabaseConnection) -> i64 {
    let thread = create_thread(
        db,
        AgentThreadCreateInput {
            owner_user_id: 1,
            thread_kind: "main".into(),
            profile_id: format!("history-benchmark-{}", uuid::Uuid::new_v4()),
            title: Some("history benchmark".into()),
            project_id: None,
            resource_type: None,
            resource_id: None,
            canonical_binding_key: None,
            scope_json: json!({}),
            default_model_config_id: None,
            is_default: false,
        },
    )
    .await
    .expect("history benchmark thread must be created");

    for index in 0..RUN_COUNT {
        let (turn, run, _) = create_turn_with_run(
            db,
            AgentTurnInput {
                thread_id: thread.id,
                client_message_id: format!("history-benchmark-message-{index}"),
                message: AgentMessageInput {
                    thread_id: thread.id,
                    turn_id: None,
                    run_id: None,
                    role: "user".into(),
                    sender_kind: "user".into(),
                    sender_thread_id: None,
                    client_message_id: Some(format!("history-benchmark-message-{index}")),
                    content: format!("历史消息 {index}"),
                    status: "complete".into(),
                    payload_json: None,
                },
                execution_mode: "autonomous".into(),
                model_override_json: None,
                budget_json: json!({"max_steps": 100, "max_model_calls": 20}),
                runtime_snapshot_required: false,
                trigger: "user".into(),
                goal: Some(format!("历史任务 {index}")),
            },
        )
        .await
        .expect("history benchmark turn must be created");
        let now = Utc::now().naive_utc();
        let mut run_model: run::ActiveModel = run::Entity::find_by_id(run.id)
            .one(db)
            .await
            .expect("run lookup must succeed")
            .expect("run must exist")
            .into();
        run_model.status = Set("completed".into());
        run_model.is_active = Set(false);
        run_model.completed_at = Set(Some(now));
        run_model.updated_at = Set(now);
        run_model.update(db).await.expect("run must complete");
        let mut turn_model: turn::ActiveModel = turn::Entity::find_by_id(turn.id)
            .one(db)
            .await
            .expect("turn lookup must succeed")
            .expect("turn must exist")
            .into();
        turn_model.status = Set("complete".into());
        turn_model.completed_at = Set(Some(now));
        turn_model.updated_at = Set(now);
        turn_model.update(db).await.expect("turn must complete");
    }
    thread.id
}

async fn thread_history_with_previous_budget_reads(
    db: &sea_orm::DatabaseConnection,
    owner_user_id: i64,
    thread_id: i64,
    limit: u64,
) -> Result<a4n_db::agent::AgentThreadHistoryRecord, a4n_db::agent::AgentDbError> {
    let tx = db
        .begin_with_config(
            Some(IsolationLevel::RepeatableRead),
            Some(AccessMode::ReadOnly),
        )
        .await?;
    let thread = thread_by_id(&tx, thread_id)
        .await?
        .ok_or_else(|| a4n_db::agent::AgentDbError::NotFound("thread missing".into()))?;
    let messages = messages_by_thread(&tx, thread_id, None, limit).await?;
    let turns = turns_by_thread(&tx, thread_id, limit).await?;
    let mut runs = runs_by_thread(&tx, thread_id, limit).await?;
    let executions = executions_by_runs(&tx, runs.iter().map(|row| row.id).collect()).await?;
    let delegations = a4n_db::agent::delegations_by_thread(&tx, thread_id).await?;
    let approvals = a4n_db::agent::approvals_by_thread(&tx, thread_id).await?;
    let inbox = inbox_by_thread(&tx, thread_id).await?;
    let memories = list_memory(
        &tx,
        owner_user_id,
        Some("thread"),
        Some(&thread_id.to_string()),
    )
    .await?;
    let events = latest_events_by_thread(&tx, thread_id, limit).await?;
    for run in &mut runs {
        run.usage_json = run_budget_snapshot(&tx, run.id).await?.usage;
    }
    tx.commit().await?;
    Ok(a4n_db::agent::AgentThreadHistoryRecord {
        thread,
        messages,
        turns,
        runs,
        executions,
        delegations,
        approvals,
        inbox,
        memories,
        events,
    })
}

#[tokio::test]
#[ignore = "manual PostgreSQL performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_agent_thread_history_budget_reads() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let thread_id = history_fixture(&db).await;
    let mut previous_samples = Vec::with_capacity(SAMPLES);
    let mut current_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let previous = thread_history_with_previous_budget_reads(&db, 1, thread_id, 64)
                .await
                .expect("previous history shape must succeed");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(previous.runs.len(), RUN_COUNT);

            let started = Instant::now();
            let current = thread_history(&db, 1, thread_id, 64)
                .await
                .expect("batched history must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(current, previous);
        } else {
            let started = Instant::now();
            let current = thread_history(&db, 1, thread_id, 64)
                .await
                .expect("batched history must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(current.runs.len(), RUN_COUNT);

            let started = Instant::now();
            let previous = thread_history_with_previous_budget_reads(&db, 1, thread_id, 64)
                .await
                .expect("previous history shape must succeed");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(previous, current);
        }
    }
    let previous_p50 = percentile(&previous_samples, 0.50);
    let previous_p95 = percentile(&previous_samples, 0.95);
    let current_p50 = percentile(&current_samples, 0.50);
    let current_p95 = percentile(&current_samples, 0.95);
    eprintln!(
        "agent_thread_history runs={RUN_COUNT} previous_p50_p95_ms={previous_p50:.3}/{previous_p95:.3} current_p50_p95_ms={current_p50:.3}/{current_p95:.3} improvement={:.1}%/{:.1}%",
        (1.0 - current_p50 / previous_p50) * 100.0,
        (1.0 - current_p95 / previous_p95) * 100.0,
    );
}

#[tokio::test]
async fn thread_history_batch_preserves_budget_projection() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let thread_id = history_fixture(&db).await;
    let previous = thread_history_with_previous_budget_reads(&db, 1, thread_id, 64)
        .await
        .expect("previous history shape must succeed");
    let current = thread_history(&db, 1, thread_id, 64)
        .await
        .expect("batched history must succeed");
    assert_eq!(current, previous);
}
