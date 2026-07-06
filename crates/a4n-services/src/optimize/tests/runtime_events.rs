use super::*;

#[tokio::test]
async fn cancelled_chat_after_llm_return_keeps_assistant_reply() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let llm = StaticLlmProvider {
        reply: serde_json::to_string(&json!({
            "thought": "直接回复即可",
            "action": "final",
            "arguments": {
                "reply": "已经返回但用户随后停止的普通回复。"
            }
        }))
        .unwrap(),
    };
    let variables = std::collections::HashMap::from([(
        "agent_permissions".to_string(),
        OptimizeAgentPermissions::default().as_value(),
    )]);
    let session = service
        .start_chat_session(project.id, chapter.id, "测试取消对话", &variables)
        .await
        .expect("chat session should be created");

    let err = {
        let mut progress = |current: u32, _total: u32, _message: &str| -> Result<(), AppError> {
            if current == 3 {
                return Err(AppError::InvalidArgument("模拟停止".to_string()));
            }
            Ok(())
        };
        service
            .send_user_message_with_permissions(crate::optimize::OptimizeSendUserMessageInput {
                session_id: session.id,
                message: "先回答，再模拟停止",
                permissions_override: None,
                llm: &llm,
                prompt_engine: &engine,
                workflow_run_id: insert_optimize_workflow_run(
                    db.clone(),
                    &session,
                    "content_optimization_chat",
                    "chat",
                )
                .await,
                progress: Some(&mut progress),
                stream: None,
            })
            .await
            .expect_err("cancel after LLM return should stop the turn")
    };
    assert!(err.to_string().contains("模拟停止"));

    let messages = service.session_messages(session.id).await.unwrap();
    assert_message_contains(&messages, "user", "先回答，再模拟停止");
    assert_message_contains(&messages, "agent_event", "普通对话 Agent 已返回 final");
    assert_message_contains(&messages, "llm_response", "已经返回但用户随后停止");
    assert_message_contains(&messages, "assistant", "已经返回但用户随后停止");
}

#[tokio::test]
async fn command_error_releases_fixing_session_without_hiding_failure() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let session = insert_waiting_session_with_issues(db.as_ref(), project.id, chapter.id).await;
    let mut active: optimization_session::ActiveModel = session.clone().into();
    active.status = Set("fixing".to_string());
    active.update(db.as_ref()).await.unwrap();

    service
        .record_command_error(session.id, "内容优化工具检查", "工具参数无效")
        .await
        .expect("command errors should be persisted and release in-progress status");

    let messages = service.session_messages(session.id).await.unwrap();
    assert_message_contains(
        &messages,
        "agent_event",
        "内容优化工具检查失败或停止：工具参数无效",
    );
    let updated = service.get_session(session.id).await.unwrap();
    assert_eq!(updated.status, "waiting_user");
    assert_eq!(
        updated.summary.as_deref(),
        Some("内容优化工具检查失败：工具参数无效")
    );
}

#[tokio::test]
async fn command_stop_marks_in_progress_session_cancelled() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let session = insert_waiting_session_with_issues(db.as_ref(), project.id, chapter.id).await;
    let mut active: optimization_session::ActiveModel = session.clone().into();
    active.status = Set("analyzing".to_string());
    active.update(db.as_ref()).await.unwrap();

    service
        .record_command_error(session.id, "内容优化分析", "优化分析已停止")
        .await
        .expect("command stop should persist cancellation");

    let messages = service.session_messages(session.id).await.unwrap();
    assert_message_contains(
        &messages,
        "agent_event",
        "内容优化分析失败或停止：优化分析已停止",
    );
    assert_message_contains(&messages, "agent_event", "会话已取消");
    let updated = service.get_session(session.id).await.unwrap();
    assert_eq!(updated.status, "cancelled");
}

#[tokio::test]
async fn record_llm_exchange_persists_visible_request_and_response_messages() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let session = insert_waiting_session_with_issues(db.as_ref(), project.id, chapter.id).await;

    service
        .record_llm_request(session.id, "Agent 规划", "完整规划提示词")
        .await
        .expect("llm request should persist");
    service
        .record_llm_response(session.id, "Agent 规划", r#"{"action":"chat"}"#)
        .await
        .expect("llm response should persist");

    let messages = service.session_messages(session.id).await.unwrap();
    assert_message_contains(&messages, "llm_request", "完整规划提示词");
    assert_message_contains(&messages, "llm_response", "action");
    assert_message_contains(&messages, "llm_response", "chat");
}
