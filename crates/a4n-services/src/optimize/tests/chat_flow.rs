use super::*;

#[tokio::test]
async fn send_user_message_persists_visible_chat_turn_and_events() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let llm = SequenceLlmProvider::new(vec![
        serde_json::to_string(&json!({
            "thought": "先明确用户当前请求需要什么信息",
            "public_update": "我会先判断这条请求是否需要读取章节内容。",
            "action": "think",
            "arguments": {
                "focus": "判断当前用户问题是否需要读取章节正文",
                "known_facts": ["用户在普通对话中要求优化当前内容"],
                "information_gaps": ["需要当前章节正文才能定位段落问题"],
                "next_step": "读取 current_chapter"
            }
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "thought": "用户询问当前章节问题，需要先读取当前章节",
            "public_update": "这个问题需要结合当前章节正文，我先读取本章内容。",
            "action": "run_tool",
            "arguments": {
                "tool": "get_project_information",
                "arguments": {
                    "mode": "read",
                    "source": "current_chapter"
                }
            }
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "thought": "已获得章节内容，可以围绕用户问题回复",
            "action": "final",
            "arguments": {
                "reply": "这一章的主要问题是调查动机还可以再压实。"
            }
        }))
        .unwrap(),
    ]);
    let variables = std::collections::HashMap::from([(
        "agent_permissions".to_string(),
        OptimizeAgentPermissions::default().as_value(),
    )]);
    let session = service
        .start_chat_session(project.id, chapter.id, "测试普通对话", &variables)
        .await
        .expect("chat session should be created without analysis");
    let mut progress_events = Vec::new();
    let mut stream_chunks = Vec::new();
    {
        let mut progress = |current: u32, total: u32, message: &str| -> Result<(), AppError> {
            progress_events.push((current, total, message.to_string()));
            Ok(())
        };
        let mut stream = |delta: &str, index: u32| -> Result<(), AppError> {
            stream_chunks.push((index, delta.to_string()));
            Ok(())
        };
        service
            .send_user_message_with_permissions(crate::optimize::OptimizeSendUserMessageInput {
                session_id: session.id,
                message: "我感觉第二段有点问题，你优化一下吧",
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
                stream: Some(&mut stream),
            })
            .await
            .expect("chat agent loop should persist visible messages");
    }

    let messages = service
        .session_messages(session.id)
        .await
        .expect("messages should be readable");
    assert!(messages
        .iter()
        .any(|message| message.role == "context" && message.content.contains("current_chapter")));
    assert!(messages.iter().any(|message| {
        message.role == "user" && message.content == "我感觉第二段有点问题，你优化一下吧"
    }));
    assert!(
        messages
            .iter()
            .any(|message| message.role == "assistant" && message.content.contains("调查动机"))
    );
    assert!(
        messages.iter().any(
            |message| message.role == "agent_event" && message.content.contains("收到用户消息")
        )
    );
    assert!(messages.iter().any(|message| {
        message.role == "agent_event" && message.content.contains("启动普通对话 Agent Loop")
    }));
    assert_message_contains(&messages, "agent_event", "普通对话思考第 1 步");
    assert_message_contains(&messages, "llm_response", "think");
    assert_message_contains(&messages, "llm_request", "内容优化对话 Agent Loop #1");
    assert_message_contains(&messages, "llm_request", "内容优化对话 Agent Loop #2");
    assert_message_contains(&messages, "llm_request", "内容优化对话 Agent Loop #3");
    assert_message_contains(&messages, "llm_request", "我感觉第二段有点问题");
    assert_message_contains(
        &messages,
        "llm_request",
        "content_optimization_chat_agent_loop",
    );
    assert_message_contains(&messages, "llm_response", "get_project_information");
    assert_message_contains(&messages, "llm_response", "final");
    assert_message_contains(&messages, "tool", "get_project_information");
    assert_message_contains(&messages, "tool", "第一段正文");
    assert_message_contains(&messages, "llm_response", "调查动机");
    assert!(messages.iter().any(|message| {
        message.role == "agent_event" && message.content.contains("普通对话 Agent 已返回 final")
    }));

    let updated = service.get_session(session.id).await.unwrap();
    assert_eq!(updated.status, "waiting_user");
    assert!(progress_events.len() >= 5);
    assert_eq!(progress_events[0].2, "正在读取优化会话上下文");
    assert_eq!(
        progress_events.last().map(|event| event.2.as_str()),
        Some("Agent 对话已更新")
    );
    assert!(
        stream_chunks
            .iter()
            .any(|(_, chunk)| chunk.contains("final"))
    );
    assert!(
        stream_chunks
            .iter()
            .any(|(_, chunk)| chunk.contains("调查动机"))
    );
}

#[tokio::test]
async fn send_user_message_simple_greeting_does_not_send_chapter_content() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let llm = StaticLlmProvider {
        reply: serde_json::to_string(&json!({
            "thought": "用户只是问候，不需要读取章节上下文",
            "action": "final",
            "arguments": {
                "reply": "你好。你可以继续告诉我具体想看哪里。"
            }
        }))
        .unwrap(),
    };
    let variables = std::collections::HashMap::from([(
        "agent_permissions".to_string(),
        OptimizeAgentPermissions::default().as_value(),
    )]);
    let session = service
        .start_chat_session(project.id, chapter.id, "测试问候", &variables)
        .await
        .expect("chat session should be created without analysis");

    service
        .send_user_message_with_permissions(crate::optimize::OptimizeSendUserMessageInput {
            session_id: session.id,
            message: "你好",
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
            progress: None,
            stream: None,
        })
        .await
        .expect("simple greeting should stay in chat mode");

    let messages = service.session_messages(session.id).await.unwrap();
    assert!(
        messages
            .iter()
            .any(|message| message.role == "user" && message.content == "你好")
    );
    let assistant = messages
        .iter()
        .find(|message| message.role == "assistant")
        .expect("direct assistant reply should be saved");
    assert!(assistant.content.starts_with("你好。"));
    assert!(!assistant.content.contains("总体印象"));
    assert!(!assistant.content.contains("主要问题"));
    assert!(!assistant.content.contains("旧城"));
    assert_message_contains(&messages, "agent_event", "收到用户消息");
    assert_message_contains(&messages, "agent_event", "启动普通对话 Agent Loop");
    assert_message_contains(&messages, "llm_request", "内容优化对话 Agent Loop #1");
    assert_message_contains(&messages, "llm_response", "final");
    assert_no_message_contains(&messages, "tool", "get_project_information");
    assert_no_message_contains(&messages, "llm_request", "第一段正文");
    assert_no_message_contains(&messages, "llm_request", "主角收到旧信");
}

#[tokio::test]
async fn send_user_message_continues_after_tool_failure_observation() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let llm = SequenceLlmProvider::new(vec![
        serde_json::to_string(&json!({
            "thought": "用户问题需要章节正文，但第一次工具参数缺少 mode",
            "public_update": "我先读取当前章节正文，以便判断这一段怎么优化。",
            "action": "run_tool",
            "arguments": {
                "tool": "get_project_information",
                "arguments": {
                    "source": "current_chapter"
                }
            }
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "thought": "上一步工具失败，补齐 mode 后重试读取",
            "public_update": "刚才读取参数不完整，我补齐读取模式后重新获取当前章节。",
            "action": "run_tool",
            "arguments": {
                "tool": "get_project_information",
                "arguments": {
                    "mode": "read",
                    "source": "current_chapter"
                }
            }
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "thought": "已获得章节正文，可以回答用户",
            "action": "final",
            "arguments": {
                "reply": "第二段的问题是行动动机偏跳，可以补一句旧信与旧城线索之间的因果。"
            }
        }))
        .unwrap(),
    ]);
    let variables = std::collections::HashMap::from([(
        "agent_permissions".to_string(),
        OptimizeAgentPermissions::default().as_value(),
    )]);
    let session = service
        .start_chat_session(project.id, chapter.id, "测试工具重试", &variables)
        .await
        .expect("chat session should be created");

    service
        .send_user_message_with_permissions(crate::optimize::OptimizeSendUserMessageInput {
            session_id: session.id,
            message: "帮我优化一下第二段",
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
            progress: None,
            stream: None,
        })
        .await
        .expect("tool failure observation should be handed back to the chat agent");

    let requests = llm.requests();
    assert_eq!(requests.len(), 3);
    let retry_prompt = request_user_content(&requests[1]);
    assert!(retry_prompt.contains(r#""success": false"#));
    assert!(retry_prompt.contains("缺少必填字符串参数: mode"));
    let messages = service.session_messages(session.id).await.unwrap();
    assert_message_contains(&messages, "agent_event", "执行失败，已交回 Agent 决策");
    assert_message_contains(&messages, "tool", "get_project_information");
    assert_message_contains(&messages, "tool", r#""success": false"#);
    assert_message_contains(&messages, "tool", "缺少必填字符串参数: mode");
    assert_message_contains(&messages, "tool", "第一段正文");
    assert_message_contains(&messages, "assistant", "行动动机偏跳");
}

#[tokio::test]
async fn chat_edit_article_requires_confirmation_without_automatic_writeback() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let llm = SequenceLlmProvider::new(vec![
        serde_json::to_string(&json!({
            "thought": "用户要求修改第二段，需要提交局部补丁",
            "public_update": "我会直接生成一处局部补丁，补足第二段的行动动机。",
            "action": "run_tool",
            "arguments": {
                "tool": "edit_article",
                "arguments": {
                    "summary": "补足行动动机",
                    "patches": [{
                        "patch_id": "patch-001",
                        "issue_id": null,
                        "operation": "replace",
                        "old_text": "主角收到旧信，决定进入旧城",
                        "new_text": "主角收到旧信，又想起信中提到的旧城印记，决定进入旧城",
                        "reason": "补足行动因果",
                        "risk": "low"
                    }]
                }
            }
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "thought": "补丁已生成但需要用户确认",
            "action": "final",
            "arguments": {
                "reply": "已生成 1 个局部补丁，需要你确认后应用。"
            }
        }))
        .unwrap(),
    ]);
    let variables = std::collections::HashMap::from([(
        "agent_permissions".to_string(),
        permissions_with_automatic_writeback(false).as_value(),
    )]);
    let session = service
        .start_chat_session(project.id, chapter.id, "测试手动补丁", &variables)
        .await
        .expect("chat session should be created");

    service
        .send_user_message_with_permissions(crate::optimize::OptimizeSendUserMessageInput {
            session_id: session.id,
            message: "帮我优化一下第二段",
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
            progress: None,
            stream: None,
        })
        .await
        .expect("manual writeback chat should return pending patch");

    let updated_chapter = chapter::Entity::find_by_id(chapter.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_chapter.content, chapter.content);
    let updated_session = service.get_session(session.id).await.unwrap();
    assert_eq!(updated_session.fixes_applied, 0);
    let messages = service.session_messages(session.id).await.unwrap();
    assert_message_contains(&messages, "tool", "edit_article");
    assert_message_contains(&messages, "tool", "pending_confirmation");
    assert_no_message_contains(&messages, "tool", r#""mode": "applied""#);
}

#[tokio::test]
async fn chat_edit_article_batches_review_when_automatic_writeback_enabled() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let llm = SequenceLlmProvider::new(vec![
        serde_json::to_string(&json!({
            "thought": "批量审阅已启用，提交局部补丁",
            "public_update": "我会生成局部补丁，稍后由你统一接受或拒绝。",
            "action": "run_tool",
            "arguments": {
                "tool": "edit_article",
                "arguments": {
                    "summary": "补足行动动机",
                    "patches": [{
                        "patch_id": "patch-001",
                        "issue_id": null,
                        "operation": "replace",
                        "old_text": "主角收到旧信，决定进入旧城",
                        "new_text": "主角收到旧信，又想起信中提到的旧城印记，决定进入旧城",
                        "reason": "补足行动因果",
                        "risk": "low"
                    }]
                }
            }
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "thought": "补丁已生成，等待批量审阅",
            "action": "final",
            "arguments": {
                "reply": "已生成 1 个局部补丁，等待你批量接受或拒绝。"
            }
        }))
        .unwrap(),
    ]);
    let variables = std::collections::HashMap::from([(
        "agent_permissions".to_string(),
        permissions_with_automatic_writeback(true).as_value(),
    )]);
    let session = service
        .start_chat_session(project.id, chapter.id, "测试自动审阅", &variables)
        .await
        .expect("chat session should be created");

    service
        .send_user_message_with_permissions(crate::optimize::OptimizeSendUserMessageInput {
            session_id: session.id,
            message: "帮我优化一下第二段",
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
            progress: None,
            stream: None,
        })
        .await
        .expect("batch review chat should return pending patch");

    let updated_chapter = chapter::Entity::find_by_id(chapter.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_chapter.content, chapter.content);
    let updated_session = service.get_session(session.id).await.unwrap();
    assert_eq!(updated_session.fixes_applied, 0);
    let messages = service.session_messages(session.id).await.unwrap();
    assert_message_contains(&messages, "tool", "edit_article");
    assert_message_contains(&messages, "tool", "pending_confirmation");
    assert_no_message_contains(&messages, "tool", r#""mode": "applied""#);
}
