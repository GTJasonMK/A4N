use super::*;

#[tokio::test]
async fn session_messages_order_tied_timestamps_by_id() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let session = insert_waiting_session_with_issues(db.as_ref(), project.id, chapter.id).await;
    let now = fixed_time();

    for (id, role, content) in [
        (30, "assistant", "第三条"),
        (10, "user", "第一条"),
        (20, "llm_request", "第二条"),
    ] {
        optimization_session_message::ActiveModel {
            id: Set(id),
            session_id: Set(session.id),
            role: Set(role.to_string()),
            content: Set(content.to_string()),
            tool_calls: Set(None),
            created_at: Set(now),
        }
        .insert(db.as_ref())
        .await
        .unwrap();
    }

    let messages = service.session_messages(session.id).await.unwrap();
    let ordered = messages
        .iter()
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ordered, vec!["第一条", "第二条", "第三条"]);
}

#[tokio::test]
async fn delete_session_removes_session_and_messages() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let session = insert_waiting_session_with_issues(db.as_ref(), project.id, chapter.id).await;

    service
        .save_message(session.id, "agent_event", "准备删除的消息")
        .await
        .unwrap();
    assert_eq!(service.session_messages(session.id).await.unwrap().len(), 1);

    service
        .delete_session(session.id)
        .await
        .expect("session deletion should delete messages first");

    let err = service.get_session(session.id).await.unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)));
    assert!(
        service
            .session_messages(session.id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn list_sessions_order_tied_updates_by_id_desc() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let now = fixed_time();

    for (id, summary) in [(100, "旧会话"), (101, "新会话")] {
        optimization_session::ActiveModel {
            id: Set(id),
            project_id: Set(project.id),
            chapter_id: Set(chapter.id),
            name: Set(Some(summary.to_string())),
            status: Set("waiting_user".to_string()),
            issues: Set(None),
            agent_permissions: Set(Some(OptimizeAgentPermissions::default().as_value())),
            agent_instruction: Set(None),
            fixes_applied: Set(0),
            summary: Set(Some(summary.to_string())),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db.as_ref())
        .await
        .unwrap();
    }

    let sessions = service.list_sessions(project.id, None).await.unwrap();
    assert_eq!(sessions[0].id, 101);
    assert_eq!(sessions[1].id, 100);
}

#[tokio::test]
async fn list_sessions_filters_by_chapter_id() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, first_chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let second_chapter = insert_test_chapter(db.as_ref(), project.id, 2, "旧城回声").await;
    let service = OptimizeService::new(db.clone());
    let now = fixed_time();

    for (id, chapter_id, name) in [
        (100, first_chapter.id, "第一章会话"),
        (101, second_chapter.id, "第二章会话"),
    ] {
        optimization_session::ActiveModel {
            id: Set(id),
            project_id: Set(project.id),
            chapter_id: Set(chapter_id),
            name: Set(Some(name.to_string())),
            status: Set("waiting_user".to_string()),
            issues: Set(None),
            agent_permissions: Set(Some(OptimizeAgentPermissions::default().as_value())),
            agent_instruction: Set(None),
            fixes_applied: Set(0),
            summary: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db.as_ref())
        .await
        .unwrap();
    }

    let first_chapter_sessions = service
        .list_sessions(project.id, Some(first_chapter.id))
        .await
        .unwrap();
    let second_chapter_sessions = service
        .list_sessions(project.id, Some(second_chapter.id))
        .await
        .unwrap();

    assert_eq!(
        first_chapter_sessions
            .iter()
            .map(|session| session.name.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("第一章会话")]
    );
    assert_eq!(
        second_chapter_sessions
            .iter()
            .map(|session| session.name.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("第二章会话")]
    );
}

#[tokio::test]
async fn new_optimization_sessions_require_user_supplied_name() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let variables = analysis_variables(Some("检查本章逻辑"));

    let chat_err = service
        .start_chat_session(project.id, chapter.id, "  ", &variables)
        .await
        .expect_err("chat session without a user name must fail");
    let analysis_err = service
        .create_analysis_session(
            project.id,
            chapter.id,
            "",
            &chapter.content,
            &engine,
            &variables,
        )
        .await
        .expect_err("analysis session without a user name must fail");

    assert!(
        matches!(chat_err, AppError::InvalidArgument(message) if message == "会话名称不能为空")
    );
    assert!(
        matches!(analysis_err, AppError::InvalidArgument(message) if message == "会话名称不能为空")
    );
    assert!(
        service
            .list_sessions(project.id, None)
            .await
            .expect("sessions should be readable")
            .is_empty()
    );
}

#[tokio::test]
async fn new_optimization_sessions_reject_duplicate_name_within_chapter() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, chapter) = insert_test_project_and_chapter(db.as_ref()).await;
    let second_chapter = insert_test_chapter(db.as_ref(), project.id, 2, "旧城回声").await;
    let service = OptimizeService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let variables = analysis_variables(Some("检查本章逻辑"));

    service
        .start_chat_session(project.id, chapter.id, "重复会话", &variables)
        .await
        .unwrap();
    let chat_err = service
        .start_chat_session(project.id, chapter.id, "重复会话", &variables)
        .await
        .expect_err("duplicate chat session name in the same chapter must fail");
    let analysis_err = service
        .create_analysis_session(
            project.id,
            chapter.id,
            "重复会话",
            &chapter.content,
            &engine,
            &variables,
        )
        .await
        .expect_err("duplicate analysis session name in the same chapter must fail");

    assert!(
        matches!(chat_err, AppError::InvalidArgument(message) if message == "本章已存在名为「重复会话」的内容优化会话")
    );
    assert!(
        matches!(analysis_err, AppError::InvalidArgument(message) if message == "本章已存在名为「重复会话」的内容优化会话")
    );
    service
        .start_chat_session(project.id, second_chapter.id, "重复会话", &variables)
        .await
        .expect("same session name should be allowed in another chapter");
}

#[tokio::test]
async fn start_chat_session_requires_initial_context_before_insert() {
    let db = setup_db().await;
    let db = Arc::new(db);
    let (project, _) = insert_test_project_and_chapter(db.as_ref()).await;
    let service = OptimizeService::new(db.clone());
    let variables = std::collections::HashMap::from([(
        "agent_permissions".to_string(),
        OptimizeAgentPermissions::default().as_value(),
    )]);

    let err = service
        .start_chat_session(project.id, 999_999, "测试会话", &variables)
        .await
        .expect_err("chat session should fail without chapter context");

    assert!(matches!(err, AppError::NotFound(_)));
    assert!(
        service
            .list_sessions(project.id, None)
            .await
            .expect("sessions should be readable")
            .is_empty()
    );
}
