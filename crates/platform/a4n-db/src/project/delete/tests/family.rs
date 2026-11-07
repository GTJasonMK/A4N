use a4n_db_models::models::{ai::system_settings, chapter::chapter, project::novel_project};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use super::super::{ProjectDeleteDbError, delete_project_cascade_record};
use super::support::{root_with_video_derivative, set_project_gate, setup_db, text_project};

#[tokio::test]
async fn deleting_root_removes_historical_snapshots_after_derived_projects_are_gone() {
    use a4n_db_models::models::project::{project_source_snapshot, source_snapshot_chapter};
    let db = setup_db().await;
    let (root, derived) = root_with_video_derivative(&db).await;
    let source_id = derived.source_snapshot_id.unwrap();
    let source = project_source_snapshot::Entity::find_by_id(source_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let mut historical: project_source_snapshot::ActiveModel = source.clone().into();
    historical.id = sea_orm::NotSet;
    historical.source_version_hash = Set(a4n_domain::source_text_hash("historical snapshot"));
    let historical = historical.insert(&db).await.unwrap();
    for chapter in source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(source_id))
        .all(&db)
        .await
        .unwrap()
    {
        let mut copy: source_snapshot_chapter::ActiveModel = chapter.into();
        copy.id = sea_orm::NotSet;
        copy.snapshot_id = Set(historical.id);
        copy.insert(&db).await.unwrap();
    }
    let other = text_project(&db, "unrelated root").await;
    let mut other_snapshot: project_source_snapshot::ActiveModel = source.into();
    other_snapshot.id = sea_orm::NotSet;
    other_snapshot.root_project_id = Set(other.id);
    let other_snapshot = other_snapshot.insert(&db).await.unwrap();
    for project in [&derived, &root] {
        set_project_gate(
            &db,
            project,
            a4n_domain::generation::ProjectLifecycleOperation::Delete,
        )
        .await;
        delete_project_cascade_record(&db, project.id)
            .await
            .unwrap();
    }
    assert!(
        novel_project::Entity::find_by_id(root.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        source_snapshot_chapter::Entity::find()
            .filter(source_snapshot_chapter::Column::RootProjectId.eq(root.id))
            .count(&db)
            .await
            .unwrap(),
        0
    );
    assert!(
        project_source_snapshot::Entity::find_by_id(historical.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        project_source_snapshot::Entity::find_by_id(other_snapshot.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
}

async fn insert_setting(
    db: &impl crate::DbExecutor,
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

async fn setting_keys(db: &impl crate::DbExecutor) -> Vec<String> {
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
    let owner = a4n_test_support::database::ensure_test_user(&db, 1).await;
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
