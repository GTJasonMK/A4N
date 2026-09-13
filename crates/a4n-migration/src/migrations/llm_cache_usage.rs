use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "ALTER TABLE llm_call_trace ADD COLUMN IF NOT EXISTS prompt_cache_write_tokens INTEGER"
        ).await?;
        manager.get_connection().execute_unprepared(
            "ALTER TABLE llm_call_trace ADD CONSTRAINT llm_cache_write_nonnegative CHECK (prompt_cache_write_tokens >= 0)"
        ).await?;
        // Older Responses/Gemini traces retained cache reads but omitted misses.
        // Repair only derivable counts; unknown write usage remains NULL.
        manager
            .get_connection()
            .execute_unprepared(
                r#"
            UPDATE llm_call_trace
            SET prompt_cache_miss_tokens = prompt_tokens - prompt_cache_hit_tokens,
                prompt_cache_hit_ratio = CASE WHEN prompt_tokens > 0
                    THEN prompt_cache_hit_tokens::double precision / prompt_tokens ELSE NULL END
            WHERE prompt_cache_miss_tokens IS NULL
                AND prompt_cache_hit_tokens >= 0
                AND prompt_tokens >= prompt_cache_hit_tokens
        "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE llm_call_trace DROP COLUMN prompt_cache_write_tokens")
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, DbBackend, Statement};

    #[tokio::test]
    async fn upgrade_repairs_derivable_cache_misses_without_inventing_write_usage() {
        let db = crate::test_support::connect_test_database().await;
        let manager = SchemaManager::new(&db);
        Migration.down(&manager).await.unwrap();
        db.execute_unprepared("INSERT INTO llm_call_trace (workflow, step_key, prompt_tokens, prompt_cache_hit_tokens, streamed, status, created_at) VALUES ('cache_migration', 'known', 10000, 9000, FALSE, 'completed', NOW()), ('cache_migration', 'unknown', 10000, NULL, FALSE, 'completed', NOW())").await.unwrap();
        Migration.up(&manager).await.unwrap();
        let rows = db.query_all(Statement::from_string(DbBackend::Postgres, "SELECT step_key, prompt_cache_miss_tokens, prompt_cache_hit_ratio, prompt_cache_write_tokens FROM llm_call_trace WHERE workflow = 'cache_migration' ORDER BY step_key".to_string())).await.unwrap();
        assert_eq!(
            rows[0]
                .try_get::<Option<i32>>("", "prompt_cache_miss_tokens")
                .unwrap(),
            Some(1000)
        );
        assert_eq!(
            rows[0]
                .try_get::<Option<f64>>("", "prompt_cache_hit_ratio")
                .unwrap(),
            Some(0.9)
        );
        for row in &rows {
            assert_eq!(
                row.try_get::<Option<i32>>("", "prompt_cache_write_tokens")
                    .unwrap(),
                None
            );
        }
        assert_eq!(
            rows[1]
                .try_get::<Option<i32>>("", "prompt_cache_miss_tokens")
                .unwrap(),
            None
        );
    }
}
