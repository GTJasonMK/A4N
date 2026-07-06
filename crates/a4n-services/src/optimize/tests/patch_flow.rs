use super::*;

#[test]
fn apply_optimization_patches_rejects_stale_hash() {
    let content = "第一段正文。主角收到旧信，决定进入旧城。";
    let patch = OptimizationPatch {
        patch_id: "patch-001".to_string(),
        issue_id: Some("issue-plot-1".to_string()),
        operation: "replace".to_string(),
        old_text: "主角收到旧信，决定进入旧城".to_string(),
        new_text: "主角收到旧信，又想起信中提到的旧城印记，决定进入旧城".to_string(),
        reason: "补足动机".to_string(),
        risk: Some("low".to_string()),
    };

    let err = apply_optimization_patches_to_content(content, "stale", &[patch])
        .expect_err("stale hash must fail");

    assert!(err.to_string().contains("hash"));
}

#[test]
fn apply_optimization_patches_requires_unique_old_text() {
    let content = "旧城。旧城。";
    let patch = OptimizationPatch {
        patch_id: "patch-001".to_string(),
        issue_id: None,
        operation: "replace".to_string(),
        old_text: "旧城".to_string(),
        new_text: "长安旧城".to_string(),
        reason: "明确地点".to_string(),
        risk: Some("low".to_string()),
    };
    let hash = optimization_content_hash(content);

    let err = apply_optimization_patches_to_content(content, &hash, &[patch])
        .expect_err("non-unique old_text must fail");

    assert!(err.to_string().contains("必须唯一匹配"));
}

#[test]
fn apply_optimization_patches_rejects_noop_replace() {
    let content = "甲。乙。";
    let hash = optimization_content_hash(content);
    let patch = OptimizationPatch {
        patch_id: "patch-noop".to_string(),
        issue_id: None,
        operation: "replace".to_string(),
        old_text: "乙。".to_string(),
        new_text: "乙。".to_string(),
        reason: "无变化".to_string(),
        risk: Some("low".to_string()),
    };

    let err = apply_optimization_patches_to_content(content, &hash, &[patch])
        .expect_err("noop replace must fail");

    assert!(err.to_string().contains("不能与 old_text 相同"));
}

#[test]
fn apply_optimization_patches_supports_delete_and_insert_operations() {
    let content = "甲。乙。丙。";
    let hash = optimization_content_hash(content);
    let patches = vec![
        OptimizationPatch {
            patch_id: "patch-delete".to_string(),
            issue_id: None,
            operation: "delete".to_string(),
            old_text: "乙。".to_string(),
            new_text: String::new(),
            reason: "删除冗余句".to_string(),
            risk: Some("low".to_string()),
        },
        OptimizationPatch {
            patch_id: "patch-before".to_string(),
            issue_id: None,
            operation: "insert_before".to_string(),
            old_text: "丙。".to_string(),
            new_text: "转折。".to_string(),
            reason: "补充转折".to_string(),
            risk: Some("low".to_string()),
        },
        OptimizationPatch {
            patch_id: "patch-after".to_string(),
            issue_id: None,
            operation: "insert_after".to_string(),
            old_text: "甲。".to_string(),
            new_text: "铺垫。".to_string(),
            reason: "补充铺垫".to_string(),
            risk: Some("low".to_string()),
        },
    ];

    let result = apply_optimization_patches_to_content(content, &hash, &patches)
        .expect("delete and insert patches should apply");

    assert_eq!(result, "甲。铺垫。转折。丙。");
}

#[test]
fn apply_optimization_patches_rejects_invalid_new_text_for_operation() {
    let content = "甲。乙。";
    let hash = optimization_content_hash(content);
    let delete_patch = OptimizationPatch {
        patch_id: "patch-delete".to_string(),
        issue_id: None,
        operation: "delete".to_string(),
        old_text: "乙。".to_string(),
        new_text: "不应存在".to_string(),
        reason: "删除冗余句".to_string(),
        risk: Some("low".to_string()),
    };
    let insert_patch = OptimizationPatch {
        patch_id: "patch-insert".to_string(),
        issue_id: None,
        operation: "insert_after".to_string(),
        old_text: "甲。".to_string(),
        new_text: String::new(),
        reason: "补充铺垫".to_string(),
        risk: Some("low".to_string()),
    };

    let delete_err =
        apply_optimization_patches_to_content(content, &hash, std::slice::from_ref(&delete_patch))
            .expect_err("delete with new_text must fail");
    let insert_err = apply_optimization_patches_to_content(content, &hash, &[insert_patch])
        .expect_err("insert without new_text must fail");

    assert!(delete_err.to_string().contains("new_text 必须为空字符串"));
    assert!(insert_err.to_string().contains("new_text 不能为空"));
}

#[tokio::test]
async fn propose_patches_persists_visible_llm_events_and_tool_proposal() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let session = insert_waiting_session_with_issues(db.as_ref(), project.id, chapter.id).await;
    let llm = StaticLlmProvider {
        reply: r#"{
                "summary": "补足主角进入旧城的动机",
                "patches": [{
                    "patch_id": "patch-001",
                    "issue_id": "issue-plot-1",
                    "operation": "replace",
                    "old_text": "主角收到旧信，决定进入旧城",
                    "new_text": "主角收到旧信，又想起信中提到的旧城印记，决定进入旧城",
                    "reason": "补足行动因果",
                    "risk": "low"
                }]
            }"#
        .to_string(),
    };
    let mut progress_events = Vec::new();
    let proposal = {
        let mut progress = |current: u32, total: u32, message: &str| -> Result<(), AppError> {
            progress_events.push((current, total, message.to_string()));
            Ok(())
        };
        service
            .propose_patches(crate::optimize::OptimizeProposePatchesInput {
                session_id: session.id,
                issue_ids: None,
                llm: &llm,
                prompt_engine: &engine,
                agent_instruction_override: Some("生成局部补丁"),
                workflow_run_id: insert_optimize_workflow_run(
                    db.clone(),
                    &session,
                    "content_optimization_patch",
                    "patch",
                )
                .await,
                progress: Some(&mut progress),
                stream: None,
            })
            .await
            .expect("patch proposal should persist visible events")
    };

    assert_eq!(proposal.patches.len(), 1);
    assert_eq!(proposal.patches[0].operation, "replace");
    assert_eq!(
        proposal.base_content_hash,
        optimization_content_hash(&chapter.content)
    );
    let messages = service.session_messages(session.id).await.unwrap();
    assert_message_contains(&messages, "user", "生成局部补丁");
    assert_message_contains(&messages, "agent_event", "准备生成局部修复补丁");
    assert_message_contains(&messages, "llm_request", "content_optimization_patch");
    assert_message_contains(&messages, "llm_response", "补足主角进入旧城的动机");
    assert_message_contains(&messages, "tool", "edit_article");
    assert_message_contains(&messages, "agent_event", "局部补丁校验通过");
    assert_eq!(progress_events.len(), 5);
    assert_eq!(progress_events[4].2, "局部补丁已生成");
}

#[tokio::test]
async fn record_patches_applied_counts_real_patch_application() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let session = insert_waiting_session_with_issues(db.as_ref(), project.id, chapter.id).await;
    let patches = vec![OptimizationPatch {
        patch_id: "patch-001".to_string(),
        issue_id: Some("issue-plot-1".to_string()),
        operation: "replace".to_string(),
        old_text: "主角收到旧信，决定进入旧城".to_string(),
        new_text: "主角收到旧信，又想起信中提到的旧城印记，决定进入旧城".to_string(),
        reason: "补足行动因果".to_string(),
        risk: Some("low".to_string()),
    }];

    service
        .record_patches_applied(
            session.id,
            chapter.id,
            33,
            &optimization_content_hash("更新后正文"),
            &patches,
        )
        .await
        .expect("applied patches should update session and visible messages");

    let messages = service.session_messages(session.id).await.unwrap();
    assert_message_contains(&messages, "tool", "apply_chapter_patches");
    assert_message_contains(&messages, "agent_event", "已应用 1 个局部补丁");
    let updated = service.get_session(session.id).await.unwrap();
    assert_eq!(updated.fixes_applied, 1);
    assert_eq!(
        updated.summary.as_deref(),
        Some("已应用 1 个局部补丁，当前字数 33")
    );
}
