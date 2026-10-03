use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter};

use a4n_db_models::models::{
    ai::generated_image,
    media::{media_asset, media_binding},
    project::novel_project,
    task::cleanup_outbox,
};

use super::super::{ProjectDeleteDbError, delete_project_cascade_record};
use super::support::{
    freeform_video_project, generated_image_with_asset, set_project_gate, setup_db,
};

#[tokio::test]
async fn project_delete_removes_canvas_dependencies_only_in_its_project() {
    use a4n_db_models::models::canvas::{
        canvas_edge, canvas_graph, canvas_node, canvas_text_asset,
    };
    use sea_orm::{ActiveModelTrait, Set};
    use serde_json::json;
    let db = setup_db().await;
    let (root, project) = super::support::root_with_video_derivative(&db).await;
    let other = freeform_video_project(&db, "保留的画布").await;
    let now = chrono::Utc::now().naive_utc();
    let mut graphs = Vec::new();
    for project_id in [project.id, other.id] {
        graphs.push(
            canvas_graph::ActiveModel {
                project_id: Set(project_id),
                name: Set("画布".into()),
                graph_kind: Set("derived_video".into()),
                version: Set(1),
                viewport_json: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&db)
            .await
            .unwrap(),
        );
    }
    let asset = canvas_text_asset::ActiveModel {
        project_id: Set(project.id),
        title: Set("原文".into()),
        text_kind: Set("freeform".into()),
        content: Set("测试正文".into()),
        content_hash: Set(a4n_domain::source_text_hash("测试正文")),
        current_version: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let mut nodes = Vec::new();
    for node_kind in ["TextInputNode", "TextOutputNode"] {
        nodes.push(
            canvas_node::ActiveModel {
                graph_id: Set(graphs[0].id),
                node_kind: Set(node_kind.into()),
                ref_type: Set(None),
                ref_id: Set(None),
                config_json: Set(json!({})),
                position_json: Set(json!({"x":0,"y":0})),
                state: Set("ready".into()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&db)
            .await
            .unwrap(),
        );
    }
    let edge = canvas_edge::ActiveModel {
        graph_id: Set(graphs[0].id),
        source_node_id: Set(nodes[0].id),
        source_port: Set("text".into()),
        target_node_id: Set(nodes[1].id),
        target_port: Set("text".into()),
        role: Set("context".into()),
        config_json: Set(json!({})),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    set_project_gate(
        &db,
        &project,
        a4n_domain::generation::ProjectLifecycleOperation::Delete,
    )
    .await;
    delete_project_cascade_record(&db, project.id)
        .await
        .unwrap();
    assert!(
        canvas_edge::Entity::find_by_id(edge.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    for node in nodes {
        assert!(
            canvas_node::Entity::find_by_id(node.id)
                .one(&db)
                .await
                .unwrap()
                .is_none()
        );
    }
    assert!(
        canvas_graph::Entity::find_by_id(graphs[0].id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        canvas_text_asset::Entity::find_by_id(asset.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        canvas_graph::Entity::find_by_id(graphs[1].id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        novel_project::Entity::find_by_id(root.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn project_cascade_rolls_back_when_final_project_delete_fails() {
    let db = setup_db().await;
    let project = freeform_video_project(&db, "项目删除事务回滚测试").await;
    let (image, asset) = generated_image_with_asset(&db, project.id, "rollback-image.png").await;
    let function_name = format!("a4n_test_block_project_delete_{}", project.id);
    let trigger_name = format!("a4n_test_block_project_delete_trigger_{}", project.id);
    db.execute_unprepared(&format!(
        "DROP TRIGGER IF EXISTS {trigger_name} ON novel_project; DROP FUNCTION IF EXISTS {function_name}()"
    ))
    .await
    .unwrap();
    db.execute_unprepared(&format!(
        "CREATE FUNCTION {function_name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'blocked project delete'; END; $$"
    ))
    .await
    .unwrap();
    db.execute_unprepared(&format!(
        "CREATE TRIGGER {trigger_name} BEFORE DELETE ON novel_project FOR EACH ROW WHEN (OLD.id = {}) EXECUTE FUNCTION {function_name}()",
        project.id
    ))
    .await
    .unwrap();

    set_project_gate(
        &db,
        &project,
        a4n_domain::generation::ProjectLifecycleOperation::Delete,
    )
    .await;
    let error = delete_project_cascade_record(&db, project.id)
        .await
        .expect_err("late project delete failure must abort the cascade");
    assert!(matches!(
        error,
        ProjectDeleteDbError::Database { ref source }
            if source.to_string().contains("blocked project delete")
    ));

    db.execute_unprepared(&format!(
        "DROP TRIGGER {trigger_name} ON novel_project; DROP FUNCTION {function_name}()"
    ))
    .await
    .unwrap();

    assert!(
        novel_project::Entity::find_by_id(project.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        generated_image::Entity::find_by_id(image.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        media_binding::Entity::find()
            .filter(media_binding::Column::TargetKind.eq("generated_image"))
            .filter(media_binding::Column::TargetId.eq(image.id))
            .count(&db)
            .await
            .unwrap(),
        1
    );
    assert!(
        media_asset::Entity::find_by_id(asset.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        cleanup_outbox::Entity::find()
            .filter(cleanup_outbox::Column::ProjectId.eq(project.id))
            .count(&db)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn project_delete_commits_cleanup_intent_with_media_paths() {
    let db = setup_db().await;
    let project = freeform_video_project(&db, "项目删除 cleanup outbox 测试").await;
    let (image, _) = generated_image_with_asset(&db, project.id, "cleanup-outbox.png").await;

    set_project_gate(
        &db,
        &project,
        a4n_domain::generation::ProjectLifecycleOperation::Delete,
    )
    .await;
    delete_project_cascade_record(&db, project.id)
        .await
        .unwrap();

    assert!(
        novel_project::Entity::find_by_id(project.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    let intent = cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::ProjectId.eq(project.id))
        .one(&db)
        .await
        .unwrap()
        .expect("project deletion must commit cleanup intent");
    assert_eq!(intent.status, crate::cleanup_outbox::CLEANUP_STATUS_PENDING);
    let payload: crate::cleanup_outbox::ProjectDeletionCleanupPayload =
        serde_json::from_value(intent.payload_json).unwrap();
    assert_eq!(payload.project_id, project.id);
    assert_eq!(payload.media_paths, vec![image.file_path]);
}
