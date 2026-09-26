use super::*;
use sea_orm::{EntityTrait, TransactionTrait};
use serde_json::json;

fn payload(id: i64, name: &str) -> Value {
    json!({
        "id":id,"project_id":10,"name":name,"last_synced_chapter":3,
        "base_attributes":{"explicit_attributes":{"first_seen_chapter":1,"role":"旧正文属性"},"implicit_attributes":{},"social_attributes":{}},
        "current_state":{"explicit_attributes":{"role":"旧生成状态"},"implicit_attributes":{},"social_attributes":{}},
        "created_at":"2026-01-01T00:00:00","updated_at":"2026-01-03T00:00:00"
    })
}

#[tokio::test]
async fn snapshot_repair_restores_identity_without_old_facts_or_overwriting_live_profiles() {
    let db = crate::test_support::connect_test_database().await;
    let tx = db.begin().await.unwrap();
    tx.execute_unprepared(r#"
        CREATE TEMP TABLE protagonist_profile (LIKE protagonist_profile INCLUDING DEFAULTS) ON COMMIT DROP;
        CREATE TEMP TABLE source_snapshot_profile (id bigint, root_project_id bigint, source_profile_id bigint,
            profile_kind text, payload_json jsonb) ON COMMIT DROP;
    "#).await.unwrap();
    let live: protagonist_profile::Model = serde_json::from_value(payload(1, "现存角色")).unwrap();
    let active: protagonist_profile::ActiveModel = live.clone().into();
    active.insert(&tx).await.unwrap();
    let mut second = payload(2, "被引用角色");
    second["current_state"]["explicit_attributes"]["role"] = json!("另一个历史状态");
    second["updated_at"] = json!("2026-01-04T00:00:00");
    for (id, profile_id, value) in [
        (1, 1, payload(1, "不得覆盖")),
        (2, 2, payload(2, "被引用角色")),
        (3, 2, second),
    ] {
        tx.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO source_snapshot_profile VALUES ($1,10,$2,'character',$3)",
            [id.into(), profile_id.into(), value.into()],
        ))
        .await
        .unwrap();
    }
    restore_snapshot_referenced_profiles(&tx).await.unwrap();
    assert_eq!(
        protagonist_profile::Entity::find_by_id(1)
            .one(&tx)
            .await
            .unwrap()
            .unwrap(),
        live
    );
    let restored = protagonist_profile::Entity::find_by_id(2)
        .one(&tx)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(restored.name, "被引用角色");
    assert_eq!(restored.project_id, 10);
    assert_eq!(restored.last_synced_chapter, 0);
    assert_eq!(
        restored.base_attributes,
        json!({"explicit_attributes":{},"implicit_attributes":{},"social_attributes":{}})
    );
    assert_eq!(restored.current_state, restored.base_attributes);
    for (id, name) in [(4, "名字 A"), (5, "名字 B")] {
        tx.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO source_snapshot_profile VALUES ($1,10,3,'character',$2)",
            [id.into(), payload(3, name).into()],
        ))
        .await
        .unwrap();
    }
    assert!(
        restore_snapshot_referenced_profiles(&tx)
            .await
            .unwrap_err()
            .to_string()
            .contains("不一致")
    );
    assert!(
        protagonist_profile::Entity::find_by_id(3)
            .one(&tx)
            .await
            .unwrap()
            .is_none()
    );
    tx.execute_unprepared("DELETE FROM source_snapshot_profile WHERE source_profile_id = 3")
        .await
        .unwrap();
    tx.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO source_snapshot_profile VALUES (6,10,4,'character',$1)",
        [payload(99, "错误身份").into()],
    ))
    .await
    .unwrap();
    assert!(
        restore_snapshot_referenced_profiles(&tx)
            .await
            .unwrap_err()
            .to_string()
            .contains("身份不一致")
    );
    assert!(
        protagonist_profile::Entity::find_by_id(4)
            .one(&tx)
            .await
            .unwrap()
            .is_none()
    );
    tx.rollback().await.unwrap();
}
