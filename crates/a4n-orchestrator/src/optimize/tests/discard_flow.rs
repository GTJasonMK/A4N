use std::sync::Arc;

use a4n_core::models::{chapter::chapter, user::role::ROLE_USER};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};

use crate::test_support::TestOrchestratorBuilder;

use super::*;

#[tokio::test]
async fn discarding_pending_patch_sets_updates_patches_and_records_one_event() {
    let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
    let user = a4n_db::auth::create_user(
        database.as_ref(),
        "optimize_discard_owner",
        "optimize-discard-owner@example.invalid",
        "test-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_project_record(
        database.as_ref(),
        a4n_db::project::CreateProjectRecordInput {
            user_id: user.id,
            title: "内容优化丢弃事务".to_string(),
            description: String::new(),
            project_type: a4n_db::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("原始正文".to_string()),
        word_count: Set(4),
        status: Set(chapter::STATUS_IMPORTED.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database.as_ref())
    .await
    .unwrap();
    let permissions = OptimizeAgentPermissions {
        read_project_detail: true,
        read_blueprint: true,
        read_outlines: true,
        read_character_profiles: true,
        read_foreshadowing: true,
        automatic_writeback: false,
    };
    let session = a4n_db::optimize::create_optimization_session(
        database.as_ref(),
        OptimizationSessionCreateInput {
            project_id: project.id,
            chapter_id: chapter.id,
            name: Some("待丢弃补丁".to_string()),
            status: "waiting_user".to_string(),
            issues: None,
            agent_permissions: Some(permissions.as_value()),
            agent_instruction: None,
            fixes_applied: 0,
            summary: None,
        },
    )
    .await
    .unwrap();
    let turn = a4n_db::optimize::create_optimization_turn(
        database.as_ref(),
        OptimizationTurnCreateInput {
            session_id: session.id,
            user_text: "生成局部补丁".to_string(),
            status: "completed".to_string(),
            permissions_snapshot: Some(permissions.as_value()),
        },
    )
    .await
    .unwrap();
    let patch_set = a4n_db::optimize::create_optimization_patch_set(
        database.as_ref(),
        OptimizationPatchSetCreateInput {
            turn_id: turn.id,
            session_id: session.id,
            chapter_id: chapter.id,
            base_content_hash: "hash".to_string(),
            status: "pending".to_string(),
            summary: "待审阅".to_string(),
            patch_decision: "pending".to_string(),
            patch_apply_status: "pending".to_string(),
            patches: vec![OptimizationPatchCreateInput {
                patch_id: "patch-1".to_string(),
                issue_id: None,
                operation: "replace".to_string(),
                old_text: "原始".to_string(),
                new_text: "优化".to_string(),
                reason: "测试".to_string(),
                risk: None,
            }],
        },
    )
    .await
    .unwrap()
    .unwrap();
    let context = TestOrchestratorBuilder::new(Arc::clone(&database))
        .build()
        .await;

    context
        .discard_pending_optimize_patch_sets_for_user_validated(user.id, session.id)
        .await
        .unwrap();

    let patch_sets = a4n_db::optimize::optimization_patch_sets(database.as_ref(), session.id)
        .await
        .unwrap();
    assert_eq!(patch_sets[0].id, patch_set.id);
    assert_eq!(patch_sets[0].status, "rejected");
    let patches = a4n_db::optimize::optimization_patch_set_patches(database.as_ref(), patch_set.id)
        .await
        .unwrap();
    assert_eq!(patches[0].decision, "rejected");
    assert_eq!(patches[0].apply_status, "pending");
    let events = a4n_db::optimize::optimization_session_messages(database.as_ref(), session.id)
        .await
        .unwrap()
        .into_iter()
        .filter(|message| message.role == "agent_event")
        .collect::<Vec<_>>();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].content, "用户已丢弃当前待应用局部补丁。");
}
