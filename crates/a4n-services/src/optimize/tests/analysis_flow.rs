use super::*;

#[tokio::test]
async fn run_analysis_session_persists_visible_llm_events_and_issues() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let llm = SequenceLlmProvider::new(vec![
        serde_json::to_string(&json!({
            "thought": "先形成分析计划并确认需要哪些观察",
            "public_update": "我会先确认分析所需的章节信息。",
            "action": "think",
            "arguments": {
                "focus": "围绕章节分析任务确定首要信息来源",
                "known_facts": ["用户要求优先检查主角动机"],
                "information_gaps": ["需要当前章节正文"],
                "next_step": "读取 current_chapter"
            }
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "thought": "需要先读取当前章节正文和元信息",
            "public_update": "我需要读取当前章节正文，作为逻辑分析依据。",
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
            "thought": "观察完成，可以输出问题清单",
            "action": "final",
            "arguments": {
                "issues": model_final_test_issues()
            }
        }))
        .unwrap(),
    ]);
    let session = service
        .create_analysis_session(
            project.id,
            chapter.id,
            "测试分析",
            &chapter.content,
            &engine,
            &analysis_variables(Some("优先检查主角动机")),
        )
        .await
        .expect("analysis session should persist visible prompt inputs");
    let mut progress_events = Vec::new();
    {
        let mut progress = |current: u32, total: u32, message: &str| -> Result<(), AppError> {
            progress_events.push((current, total, message.to_string()));
            Ok(())
        };
        service
            .run_analysis_session(
                session.id,
                &llm,
                insert_optimize_workflow_run(
                    db.clone(),
                    &session,
                    "content_optimization_analysis",
                    "analysis",
                )
                .await,
                Some(&mut progress),
                None,
            )
            .await
            .expect("analysis should persist LLM call events and issues");
    }

    let messages = service.session_messages(session.id).await.unwrap();
    assert_message_contains(&messages, "system", "显式 loop");
    assert_message_contains(&messages, "context", "current_chapter");
    assert_message_contains(&messages, "user", "优先检查主角动机");
    assert_message_contains(&messages, "agent_event", "分析会话已创建");
    assert_message_contains(&messages, "agent_event", "启动显式 Agent Loop");
    assert_message_contains(&messages, "agent_event", "我会先确认分析所需的章节信息");
    assert_message_contains(&messages, "agent_event", "我需要读取当前章节正文");
    assert_message_contains(&messages, "agent_event", "Agent 第 3 步：final");
    assert_message_contains(&messages, "agent_event", "Agent 思考第 1 步");
    assert_message_contains(&messages, "llm_request", "内容优化 Agent Loop #1");
    assert_message_contains(&messages, "llm_request", "内容优化 Agent Loop #2");
    assert_message_contains(&messages, "llm_request", "内容优化 Agent Loop #3");
    assert_message_contains(&messages, "llm_request", "json_standard");
    assert_message_contains(&messages, "llm_request", "content_optimization_agent_loop");
    assert_message_contains(&messages, "llm_response", "think");
    assert_message_contains(&messages, "llm_response", "get_project_information");
    assert_message_contains(&messages, "llm_response", "run_tool");
    assert_message_contains(&messages, "llm_response", "final");
    assert_message_contains(&messages, "tool", "get_project_information");
    assert_message_contains(&messages, "tool", "current_chapter");
    assert_message_contains(&messages, "tool", "第一段正文");
    assert_message_contains(&messages, "llm_response", "issue-plot-1");
    assert_message_contains(&messages, "assistant", "issue-plot-1");
    assert_message_contains(&messages, "agent_event", "Agent 已返回 final");
    assert_message_contains(
        &messages,
        "agent_event",
        "分析结果已解析并保存，共发现 1 个问题",
    );

    let updated = service.get_session(session.id).await.unwrap();
    assert_eq!(updated.status, "waiting_user");
    let issues = updated
        .issues
        .as_ref()
        .and_then(serde_json::Value::as_array)
        .expect("issues should be saved as array");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0]["arguments"]["evidence"], "决定进入旧城");
    assert!(issues[0]["arguments"].get("evidence_span_id").is_none());
    assert_eq!(llm.requests().len(), 3);
    assert!(progress_events.len() >= 7);
    assert_eq!(progress_events[0].2, "正在读取分析会话上下文");
    assert_eq!(
        progress_events.last().map(|event| event.2.as_str()),
        Some("优化分析结果已保存")
    );
}

#[tokio::test]
async fn run_analysis_session_allows_multi_step_exploration_before_final() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let llm = SequenceLlmProvider::new(vec![
        serde_json::to_string(&json!({
            "thought": "先确认需要读取正文",
            "public_update": "我会先确认本次分析需要读取哪些正文信息。",
            "action": "think",
            "arguments": {
                "focus": "分析正文前置条件",
                "known_facts": [],
                "information_gaps": ["当前章节正文"],
                "next_step": "读取 current_chapter"
            }
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "thought": "读取当前章节正文",
            "public_update": "我先读取当前章节正文，确认具体文本。",
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
            "thought": "需要围绕用户关注点再定位相关段落",
            "public_update": "我已经读到正文，还需要定位与用户关注点相关的段落。",
            "action": "think",
            "arguments": {
                "focus": "定位局部证据",
                "known_facts": ["已读取当前章节正文"],
                "information_gaps": ["与第一段相关的局部证据"],
                "next_step": "搜索当前章节"
            }
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "thought": "搜索当前章节中的相关证据",
            "public_update": "我会在当前章节中搜索相关段落，补齐局部证据。",
            "action": "run_tool",
            "arguments": {
                "tool": "get_project_information",
                "arguments": {
                    "mode": "search",
                    "source": "current_chapter",
                    "query": "第一段",
                    "limit": 3
                }
            }
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "thought": "观察已足够，可以输出问题",
            "public_update": "我已经获得足够依据，接下来整理问题列表。",
            "action": "think",
            "arguments": {
                "focus": "汇总证据",
                "known_facts": ["已读取正文", "已定位相关段落"],
                "information_gaps": [],
                "next_step": "输出 final issues"
            }
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "thought": "必要观察已完成，输出问题清单",
            "action": "final",
            "arguments": {
                "issues": model_final_test_issues()
            }
        }))
        .unwrap(),
    ]);
    let session = service
        .create_analysis_session(
            project.id,
            chapter.id,
            "测试多步探索",
            &chapter.content,
            &engine,
            &analysis_variables(Some("先定位问题再输出")),
        )
        .await
        .expect("analysis session should be created");

    let updated = service
        .run_analysis_session(
            session.id,
            &llm,
            insert_optimize_workflow_run(
                db.clone(),
                &session,
                "content_optimization_analysis",
                "analysis",
            )
            .await,
            None,
            None,
        )
        .await
        .expect("analysis loop should allow multi-step exploration");

    assert_eq!(llm.requests().len(), 6);
    assert_eq!(updated.status, "waiting_user");
    let messages = service.session_messages(session.id).await.unwrap();
    assert_message_contains(&messages, "llm_request", "completion_policy");
    assert_message_contains(&messages, "llm_request", "latest_information_gaps");
    assert_no_message_contains(&messages, "llm_request", "remaining_steps");
    assert_no_message_contains(&messages, "llm_request", "max_steps");
    assert_message_contains(&messages, "llm_response", "final");
    assert_message_contains(&messages, "assistant", "issue-plot-1");
}

#[tokio::test]
async fn run_analysis_session_rejects_empty_llm_json_without_repair() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let llm = StaticLlmProvider {
        reply: "".to_string(),
    };
    let session = service
        .create_analysis_session(
            project.id,
            chapter.id,
            "测试空响应",
            &chapter.content,
            &engine,
            &analysis_variables(Some("触发空响应")),
        )
        .await
        .expect("analysis session should be created");

    let err = service
        .run_analysis_session(
            session.id,
            &llm,
            insert_optimize_workflow_run(
                db.clone(),
                &session,
                "content_optimization_analysis",
                "analysis",
            )
            .await,
            None,
            None,
        )
        .await
        .expect_err("empty analysis response must fail visibly");

    assert!(err.to_string().contains("模型返回空内容"));
    let messages = service.session_messages(session.id).await.unwrap();
    assert_message_contains(&messages, "agent_event", "Agent Loop 决策解析失败");
    assert_no_message_contains(&messages, "llm_request", "内容优化分析 JSON 修复");
    let updated = service.get_session(session.id).await.unwrap();
    assert_eq!(updated.status, "waiting_user");
    assert!(
        updated
            .summary
            .as_deref()
            .unwrap_or_default()
            .contains("内容优化 Agent Loop失败")
    );
}

#[tokio::test]
async fn failed_analysis_releases_session_back_to_waiting_user() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let llm = StaticLlmProvider {
        reply: "{".to_string(),
    };
    let session = service
        .create_analysis_session(
            project.id,
            chapter.id,
            "测试坏 JSON",
            &chapter.content,
            &engine,
            &analysis_variables(Some("触发坏 JSON")),
        )
        .await
        .expect("analysis session should be created");

    let err = service
        .run_analysis_session(
            session.id,
            &llm,
            insert_optimize_workflow_run(
                db.clone(),
                &session,
                "content_optimization_analysis",
                "analysis",
            )
            .await,
            None,
            None,
        )
        .await
        .expect_err("invalid LLM JSON must fail visibly");
    assert!(err.to_string().contains("严格 JSON 决策对象"));

    let messages = service.session_messages(session.id).await.unwrap();
    assert_message_contains(&messages, "llm_response", "{");
    assert_message_contains(&messages, "agent_event", "Agent Loop 决策解析失败");
    assert_no_message_contains(&messages, "llm_request", "内容优化分析 JSON 修复");
    let updated = service.get_session(session.id).await.unwrap();
    assert_eq!(updated.status, "waiting_user");
    assert!(
        updated
            .summary
            .as_deref()
            .unwrap_or_default()
            .contains("内容优化 Agent Loop失败")
    );
}

#[tokio::test]
async fn run_analysis_session_continues_after_tool_failure_observation() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let llm = SequenceLlmProvider::new(vec![
        serde_json::to_string(&json!({
            "thought": "先尝试读取当前章节，但参数缺少 mode",
            "public_update": "我先读取当前章节正文，确认具体文本。",
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
            "thought": "上一步工具返回缺少 mode，补齐参数后重试",
            "public_update": "读取参数不完整，我补齐读取模式后重新获取当前章节。",
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
            "thought": "已获得当前章节，可以输出问题清单",
            "action": "final",
            "arguments": {
                "issues": model_final_test_issues()
            }
        }))
        .unwrap(),
    ]);
    let session = service
        .create_analysis_session(
            project.id,
            chapter.id,
            "测试工具失败继续",
            &chapter.content,
            &engine,
            &analysis_variables(Some("模拟工具参数失败后继续")),
        )
        .await
        .expect("analysis session should be created");

    let updated = service
        .run_analysis_session(
            session.id,
            &llm,
            insert_optimize_workflow_run(
                db.clone(),
                &session,
                "content_optimization_analysis",
                "analysis",
            )
            .await,
            None,
            None,
        )
        .await
        .expect("tool failure observation should be handed back to the agent");

    let requests = llm.requests();
    assert_eq!(requests.len(), 3);
    let retry_prompt = request_user_content(&requests[1]);
    assert!(retry_prompt.contains(r#""success": false"#));
    assert!(retry_prompt.contains("缺少必填字符串参数: mode"));
    assert_eq!(updated.status, "waiting_user");
    let messages = service.session_messages(session.id).await.unwrap();
    assert_message_contains(&messages, "agent_event", "执行失败，已交回 Agent 决策");
    assert_message_contains(&messages, "tool", "get_project_information");
    assert_message_contains(&messages, "tool", r#""success": false"#);
    assert_message_contains(&messages, "tool", "缺少必填字符串参数: mode");
    assert_message_contains(&messages, "tool", "第一段正文");
    assert_message_contains(&messages, "assistant", "issue-plot-1");
}

#[tokio::test]
async fn cancelled_analysis_after_llm_return_keeps_raw_assistant_reply() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let llm = SequenceLlmProvider::new(vec![
        serde_json::to_string(&json!({
            "thought": "先读取当前章节",
            "public_update": "我会先读取当前章节正文。",
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
            "thought": "返回最终问题",
            "action": "final",
            "arguments": {
                "issues": model_final_test_issues()
            }
        }))
        .unwrap(),
    ]);
    let session = service
        .create_analysis_session(
            project.id,
            chapter.id,
            "测试取消分析",
            &chapter.content,
            &engine,
            &analysis_variables(Some("模拟返回后停止")),
        )
        .await
        .expect("analysis session should be created");

    let err = {
        let mut progress = |current: u32, _total: u32, _message: &str| -> Result<(), AppError> {
            if current == 3 {
                return Err(AppError::InvalidArgument("模拟停止".to_string()));
            }
            Ok(())
        };
        service
            .run_analysis_session(
                session.id,
                &llm,
                insert_optimize_workflow_run(
                    db.clone(),
                    &session,
                    "content_optimization_analysis",
                    "analysis",
                )
                .await,
                Some(&mut progress),
                None,
            )
            .await
            .expect_err("cancel after LLM return should stop parsing")
    };
    assert!(err.to_string().contains("模拟停止"));

    let messages = service.session_messages(session.id).await.unwrap();
    assert_message_contains(&messages, "agent_event", "Agent 已返回 final");
    assert_message_contains(&messages, "llm_response", "issue-plot-1");
    assert_message_contains(&messages, "assistant", "issue-plot-1");
    assert!(!messages.iter().any(|message| {
        message.role == "agent_event" && message.content.contains("分析结果已解析并保存")
    }));
    let updated = service.get_session(session.id).await.unwrap();
    assert_eq!(updated.status, "cancelled");
    assert!(updated.issues.is_none());
}
