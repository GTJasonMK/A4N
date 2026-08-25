//! 角色档案同步的 workflow 起始路径必须拒绝同一角色 + 同一目标章节的并发运行。
//!
//! 同步走 begin_character_profile_sync_workflow_validated → begin_or_resume_workflow_run_validated
//! → resume_or_upsert_workflow_run（crates/a4n-orchestrator/src/context/character.rs）。
//! 若退回裸 upsert_workflow_run，第二个并发请求会静默接管同一行 workflow_run，
//! 两边各自跑完整套 LLM 后才由 DB 乐观锁淘汰一个。

use a4n_db::{
    project::{CreateProjectRecordInput, create_project_record},
    workflow::{
        UpsertWorkflowRunInput, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
        WorkflowWriteError, mark_workflow_status, resume_or_upsert_workflow_run,
    },
};
use serde_json::json;

/// 复刻 begin_character_profile_sync_workflow_validated 构造的输入。
fn character_sync_run_input(
    project_id: i64,
    business_key: &str,
    fingerprint: serde_json::Value,
) -> UpsertWorkflowRunInput {
    UpsertWorkflowRunInput {
        business_key: business_key.to_string(),
        workflow_kind: "character_profile_sync".to_string(),
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("protagonist_profile".to_string()),
        scope_id: Some(9),
        chapter_id: None,
        mode: Some("1-3".to_string()),
        input_fingerprint_json: Some(fingerprint),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some("character_sync.start".to_string()),
        active_task_id: None,
        can_resume: true,
        resume_block_reason: None,
        error_message: None,
    }
}

fn sync_fingerprint() -> serde_json::Value {
    json!({
        "profile_id": 9,
        "target_chapter": 3,
        "last_synced_chapter": 0,
        "baseline_state": {},
    })
}

async fn test_project(db: &sea_orm::DatabaseConnection, title: &str) -> i64 {
    create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: title.to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created")
    .id
}

#[tokio::test]
async fn concurrent_character_sync_for_the_same_profile_and_chapter_is_rejected() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = test_project(&db, "角色同步并发拒绝").await;
    let business_key = format!("character_profile_sync:{project_id}:3");

    // 两个并发请求读到同一份 profile，因此指纹完全一致。
    let first = resume_or_upsert_workflow_run(
        &db,
        character_sync_run_input(project_id, &business_key, sync_fingerprint()),
    )
    .await
    .expect("第一个角色同步必须能启动");
    assert_eq!(first.status, WORKFLOW_STATUS_RUNNING);

    // 第一个仍在 RUNNING（LLM 阶段耗时数十秒），第二个请求到达。
    let error = resume_or_upsert_workflow_run(
        &db,
        character_sync_run_input(project_id, &business_key, sync_fingerprint()),
    )
    .await
    .expect_err("并发同步必须在启动阶段就被拒绝，而不是跑完 LLM 再由乐观锁淘汰");

    assert!(
        matches!(
            &error,
            WorkflowWriteError::StatusConflict { current, operation }
                if current == WORKFLOW_STATUS_RUNNING && operation.contains("重复启动")
        ),
        "实际错误: {error:?}"
    );
}

/// 拒绝并发不能顺带堵死重试：失败的同步必须能沿用同一行 workflow_run 恢复，
/// 这样已完成阶段的 LLM checkpoint 才可复用。
#[tokio::test]
async fn a_failed_character_sync_can_still_be_resumed_on_the_same_run() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = test_project(&db, "角色同步失败恢复").await;
    let business_key = format!("character_profile_sync:{project_id}:3");

    let first = resume_or_upsert_workflow_run(
        &db,
        character_sync_run_input(project_id, &business_key, sync_fingerprint()),
    )
    .await
    .expect("第一个角色同步必须能启动");

    mark_workflow_status(
        &db,
        first.id,
        WORKFLOW_STATUS_FAILED,
        true,
        Some("LLM 阶段超时"),
    )
    .await
    .expect("标记失败必须成功");

    let resumed = resume_or_upsert_workflow_run(
        &db,
        character_sync_run_input(project_id, &business_key, sync_fingerprint()),
    )
    .await
    .expect("失败后重试必须能恢复");

    assert_eq!(
        resumed.id, first.id,
        "必须复用同一行 workflow_run，否则已完成阶段的 LLM checkpoint 会失效"
    );
    assert_eq!(resumed.status, WORKFLOW_STATUS_RUNNING);
    assert_eq!(resumed.error_message, None);
}
