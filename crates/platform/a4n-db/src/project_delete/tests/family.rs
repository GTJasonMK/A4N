use a4n_db_models::models::{ai::system_settings, chapter::chapter, project::novel_project};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use super::super::{ProjectDeleteDbError, delete_project_cascade_record};
use super::support::{root_with_video_derivative, set_project_gate, setup_db, text_project};

async fn insert_setting(
    db: &sea_orm::DatabaseConnection,
    key: String,
    category: &str,
) -> system_settings::Model {
    system_settings::ActiveModel {
        key: Set(key),
        value: Set(serde_json::Value::String("覆盖内容".to_string())),
        category: Set(category.to_string()),
        description: Set(None),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn setting_keys(db: &sea_orm::DatabaseConnection) -> Vec<String> {
    system_settings::Entity::find()
        .order_by_asc(system_settings::Column::Key)
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|record| record.key)
        .collect()
}

#[tokio::test]
async fn project_delete_removes_only_target_prompt_override_settings() {
    let db = setup_db().await;
    let project = text_project(&db, "提示词覆盖清理项目").await;
    let deleted_key = system_settings::prompt_override_key(project.id, "greeting");
    let deleted_wrong_category_key =
        system_settings::prompt_override_key(project.id, "chapter_prompt");
    let retained_key = system_settings::prompt_override_key(project.id + 1, "greeting");

    insert_setting(
        &db,
        deleted_key.clone(),
        system_settings::PROMPT_OVERRIDE_CATEGORY,
    )
    .await;
    insert_setting(&db, deleted_wrong_category_key.clone(), "general").await;
    insert_setting(
        &db,
        retained_key.clone(),
        system_settings::PROMPT_OVERRIDE_CATEGORY,
    )
    .await;

    set_project_gate(
        &db,
        &project,
        a4n_domain::generation::ProjectLifecycleOperation::Delete,
    )
    .await;
    delete_project_cascade_record(&db, project.id)
        .await
        .unwrap();

    let keys = setting_keys(&db).await;
    assert!(!keys.contains(&deleted_key));
    assert!(!keys.contains(&deleted_wrong_category_key));
    assert!(keys.contains(&retained_key));
}

#[tokio::test]
async fn project_delete_gate_rejects_text_root_with_derivatives_without_pending_operation() {
    let db = setup_db().await;
    let owner = crate::auth::create_user(
        &db,
        "project_delete_gate_owner",
        "project-delete-gate-owner@example.invalid",
        "project-delete-gate-password",
        "user",
    )
    .await
    .unwrap();
    assert_eq!(owner.id, 1);
    let (root, _derived) = root_with_video_derivative(&db).await;

    let error = crate::generation::begin_project_lifecycle_gate(
        &db,
        root.id,
        root.user_id,
        a4n_domain::generation::ProjectLifecycleOperation::Delete,
    )
    .await
    .expect_err("root delete gate should reject projects with derivatives");

    assert!(
        matches!(
            &error,
            crate::generation::GenerationLifecycleGateError::InvalidArgument(message)
                if message.contains("存在 1 个派生项目")
        ),
        "unexpected gate error: {error:?}"
    );
    let root = novel_project::Entity::find_by_id(root.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(root.lifecycle_gate, crate::generation::PROJECT_GATE_ACTIVE);
    assert!(
        crate::generation::pending_lifecycle_operations(&db)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn project_delete_rejects_text_root_with_derivatives() {
    let db = setup_db().await;
    let (root, derived) = root_with_video_derivative(&db).await;

    set_project_gate(
        &db,
        &root,
        a4n_domain::generation::ProjectLifecycleOperation::Delete,
    )
    .await;
    let error = delete_project_cascade_record(&db, root.id)
        .await
        .expect_err("root delete should require explicit family delete");

    assert!(matches!(
        error,
        ProjectDeleteDbError::DerivedProjectsExist {
            project_id,
            derived_count: 1,
        } if project_id == root.id
    ));
    assert_eq!(
        novel_project::Entity::find_by_id(root.id)
            .count(&db)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        novel_project::Entity::find_by_id(derived.id)
            .count(&db)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn deleting_derived_project_preserves_root_text_data() {
    let db = setup_db().await;
    let (root, derived) = root_with_video_derivative(&db).await;

    set_project_gate(
        &db,
        &derived,
        a4n_domain::generation::ProjectLifecycleOperation::Delete,
    )
    .await;
    delete_project_cascade_record(&db, derived.id)
        .await
        .unwrap();

    assert_eq!(
        novel_project::Entity::find_by_id(root.id)
            .count(&db)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(root.id))
            .count(&db)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        novel_project::Entity::find_by_id(derived.id)
            .count(&db)
            .await
            .unwrap(),
        0
    );
}
