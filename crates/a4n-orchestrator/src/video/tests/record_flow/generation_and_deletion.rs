#[tokio::test]
async fn delete_missing_video_returns_not_found_without_touching_prompt_reference() {
    let db = Arc::new(setup_db().await);
    let prompt = insert_video_prompt(db.as_ref(), 1, 1, Some(999)).await;

    let err = a4n_db::video::delete_generated_video_record(db.as_ref(), 999)
        .await
        .map_err(crate::video::video_db_error)
        .expect_err("missing video deletion must be visible");

    assert!(matches!(err, AppError::NotFound(message) if message == "视频不存在"));
    let stored = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.generated_video_id, Some(999));
    assert_eq!(stored.status, "completed");
}

#[tokio::test]
async fn delete_missing_video_references_clears_stale_prompt_lock() {
    let db = Arc::new(setup_db().await);
    let prompt = insert_video_prompt(db.as_ref(), 1, 1, Some(999)).await;

    let project_ids = a4n_db::video::missing_video_reference_project_ids(db.as_ref(), 999)
        .await
        .map_err(crate::video::video_db_error)
        .unwrap();
    assert_eq!(project_ids, vec![1]);

    let cleared = a4n_db::video::delete_missing_generated_video_references_record(db.as_ref(), 999)
        .await
        .map_err(crate::video::video_db_error)
        .unwrap();
    assert_eq!(cleared, 1);
    let stored = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.generated_video_id, None);
    assert_eq!(stored.status, "pending");
}

#[tokio::test]
async fn delete_video_marks_canvas_nodes_missing_without_deleting_them() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(project.id);
    input.video_type = "canvas_video".to_string();
    input.workflow_mode = Some("canvas_video".to_string());
    let video = service.create_video_record(input).await.unwrap();
    let now = fixed_time(3);
    let graph = canvas_graph::ActiveModel {
        project_id: Set(project.id),
        name: Set("视频画布".to_string()),
        graph_kind: Set("video_studio".to_string()),
        version: Set(1),
        viewport_json: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();
    let node = canvas_node::ActiveModel {
        graph_id: Set(graph.id),
        node_kind: Set("VideoNode".to_string()),
        ref_type: Set(Some("legacy_generated_video".to_string())),
        ref_id: Set(Some(video.id)),
        config_json: Set(json!({
            "generated_video_id": video.id,
            "title": "可删除视频引用"
        })),
        position_json: Set(json!({"x": 0, "y": 0})),
        state: Set("ready".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();

    a4n_db::video::delete_generated_video_record(db.as_ref(), video.id)
        .await
        .map_err(crate::video::video_db_error)
        .unwrap();

    let stored_node = canvas_node::Entity::find_by_id(node.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("canvas node should be preserved");
    assert_eq!(stored_node.state, "missing");
    assert_eq!(stored_node.ref_type, None);
    assert_eq!(stored_node.ref_id, None);
    assert_eq!(
        stored_node
            .config_json
            .get("missing_ref_type")
            .and_then(Value::as_str),
        Some("generated_video")
    );
    assert_eq!(
        stored_node
            .config_json
            .get("missing_ref_id")
            .and_then(Value::as_i64),
        Some(video.id)
    );
    assert!(stored_node.config_json.get("generated_video_id").is_none());
}
