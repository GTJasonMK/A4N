use a4n_db_models::models::user::user;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseConnection, EntityTrait, Set};

pub async fn ensure_test_user(db: &DatabaseConnection, id: i64) -> user::Model {
    assert!(id > 0, "test user ID must be positive");
    let existing = user::Entity::find_by_id(id)
        .one(db)
        .await
        .expect("test user lookup must succeed");

    let user = match existing {
        Some(user) => user,
        None => {
            let now = Utc::now().naive_utc();
            user::ActiveModel {
                id: Set(id),
                username: Set(format!("a4n-test-user-{id}")),
                email: Set(format!("a4n-test-user-{id}@example.test")),
                password_hash: Set("test-only-password-hash".to_string()),
                role: Set("user".to_string()),
                page_cli_enabled: Set(false),
                is_active: Set(true),
                lifecycle_gate: Set("active".to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .expect("test user must be inserted")
        }
    };

    db.execute_unprepared(
        r#"SELECT setval(pg_get_serial_sequence('"user"', 'id'),
                         (SELECT MAX(id) FROM "user"), TRUE)"#,
    )
    .await
    .expect("test user sequence must follow inserted IDs");
    user
}
