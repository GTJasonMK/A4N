//! Repair pending page projections written by chapter-level artifact deletion.

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "manga_page_slots_reset_repair requires PostgreSQL".into(),
            ));
        }
        manager.get_connection().execute_unprepared(UP_SQL).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "manga_page_slots_reset_repair 不可逆：不能将页面恢复为无效 slots 状态".into(),
        ))
    }
}

const UP_SQL: &str = r#"
UPDATE chapter_manga_prompt
SET layout = jsonb_set(layout::jsonb, '{pipeline,stage}', '"content"'::jsonb, false)::json,
    status = CASE WHEN generated_image_id IS NULL THEN 'pending' ELSE status END,
    updated_at = CURRENT_TIMESTAMP
WHERE prompt_type = 'page_generation'
  AND layout->>'schema_version' = 'manga_page_generation_v4'
  AND layout#>>'{pipeline,stage}' = 'slots'
  AND layout#>>'{pipeline,status}' = 'pending'
  AND COALESCE(layout::jsonb#>'{pipeline,error}', 'null'::jsonb) = 'null'::jsonb;
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Statement, TransactionTrait};

    #[tokio::test]
    async fn repairs_only_pending_slots_preserving_content_identity_and_images() {
        let db = crate::test_support::connect_test_database().await;
        let tx = db.begin().await.unwrap();
        tx.execute_unprepared(r#"
CREATE TEMP TABLE chapter_manga_prompt (
    id BIGINT PRIMARY KEY, prompt_type TEXT NOT NULL, layout JSON,
    status TEXT NOT NULL, generated_image_id BIGINT, updated_at TIMESTAMP NOT NULL
) ON COMMIT DROP;
INSERT INTO chapter_manga_prompt VALUES
(1,'page_generation','{"schema_version":"manga_page_generation_v4","page_id":"keep-page","pipeline":{"stage":"slots","status":"pending","page_content":{"summary":"keep content"},"error":null},"reference_bindings":[{"keep":"binding"}]}','failed',NULL,'2020-01-01'),
(2,'page_generation','{"schema_version":"manga_page_generation_v4","pipeline":{"stage":"content","status":"pending"}}','pending',NULL,'2020-01-01'),
(3,'page_generation','{"schema_version":"manga_page_generation_v3","pipeline":{"stage":"slots","status":"pending"}}','pending',NULL,'2020-01-01'),
(4,'page_generation','{"schema_version":"manga_page_generation_v4","pipeline":{"stage":"slots","status":"failed","error":{"stage":"slots"}}}','failed',NULL,'2020-01-01'),
(5,'page_generation','{"schema_version":"manga_page_generation_v4","pipeline":{"stage":"slots","status":"pending","error":null}}','completed',99,'2020-01-01');
"#).await.unwrap();
        let snapshot = || {
            Statement::from_string(
                DbBackend::Postgres,
                "SELECT id, layout::jsonb AS layout, status, generated_image_id, updated_at::text AS updated_at FROM chapter_manga_prompt ORDER BY id",
            )
        };
        let before = tx.query_all(snapshot()).await.unwrap();
        let manager = SchemaManager::new(&tx);
        Migration.up(&manager).await.unwrap();
        let after = tx.query_all(snapshot()).await.unwrap();
        for (old, new) in before.iter().zip(after.iter()) {
            let id = old.try_get::<i64>("", "id").unwrap();
            let mut expected = old.try_get::<serde_json::Value>("", "layout").unwrap();
            if [1, 5].contains(&id) {
                expected["pipeline"]["stage"] = serde_json::json!("content");
            } else {
                assert_eq!(
                    old.try_get::<String>("", "updated_at").unwrap(),
                    new.try_get::<String>("", "updated_at").unwrap()
                );
            }
            assert_eq!(
                new.try_get::<serde_json::Value>("", "layout").unwrap(),
                expected
            );
            assert_eq!(
                new.try_get::<Option<i64>>("", "generated_image_id")
                    .unwrap(),
                old.try_get::<Option<i64>>("", "generated_image_id")
                    .unwrap()
            );
            assert_eq!(
                new.try_get::<String>("", "status").unwrap(),
                if id == 1 {
                    "pending".into()
                } else {
                    old.try_get::<String>("", "status").unwrap()
                }
            );
        }
        Migration.up(&manager).await.unwrap();
        let repeated = tx.query_all(snapshot()).await.unwrap();
        for (first, second) in after.iter().zip(repeated.iter()) {
            assert_eq!(
                first.try_get::<serde_json::Value>("", "layout").unwrap(),
                second.try_get::<serde_json::Value>("", "layout").unwrap()
            );
            assert_eq!(
                first.try_get::<String>("", "updated_at").unwrap(),
                second.try_get::<String>("", "updated_at").unwrap()
            );
        }
        tx.rollback().await.unwrap();
    }
}
