use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // The old native adapter stored ordinary input in prompt_tokens and
        // cache creation in miss. llm_cache_usage may have filled an absent miss
        // with input - hit, so rows satisfying that identity are ambiguous.
        // Keep their known reads/output, but do not invent input or write totals.
        // The prior migration timestamp excludes requests using the new adapter.
        manager.get_connection().execute_unprepared(r#"
            DO $repair$
            DECLARE legacy_before TIMESTAMP;
            BEGIN
                SELECT to_timestamp(applied_at) AT TIME ZONE 'UTC'
                INTO STRICT legacy_before
                FROM seaql_migrations WHERE version = 'llm_cache_usage';

                WITH legacy AS (
                    SELECT id,
                        prompt_tokens >= 0 AND prompt_cache_hit_tokens >= 0
                            AND prompt_cache_miss_tokens >= 0
                            AND prompt_tokens::bigint <> prompt_cache_hit_tokens::bigint
                                + prompt_cache_miss_tokens::bigint AS recoverable
                    FROM llm_call_trace
                    WHERE lower(btrim(provider)) IN ('anthropic', 'custom-anthropic')
                        AND created_at < legacy_before
                        AND prompt_cache_write_tokens IS NULL
                )
                UPDATE llm_call_trace AS call
                SET prompt_tokens = CASE WHEN legacy.recoverable THEN
                        call.prompt_tokens + call.prompt_cache_hit_tokens + call.prompt_cache_miss_tokens END,
                    total_tokens = CASE WHEN legacy.recoverable THEN
                        call.total_tokens + call.prompt_cache_hit_tokens + call.prompt_cache_miss_tokens END,
                    prompt_cache_write_tokens = CASE WHEN legacy.recoverable THEN call.prompt_cache_miss_tokens END,
                    prompt_cache_miss_tokens = CASE WHEN legacy.recoverable THEN
                        call.prompt_tokens + call.prompt_cache_miss_tokens END,
                    prompt_cache_hit_ratio = CASE WHEN legacy.recoverable THEN
                        call.prompt_cache_hit_tokens::double precision /
                            NULLIF(call.prompt_tokens + call.prompt_cache_hit_tokens + call.prompt_cache_miss_tokens, 0) END
                FROM legacy WHERE call.id = legacy.id;
            END $repair$;
        "#).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "claude_cache_usage_repair 不可逆：旧缓存写入与推导的未命中量不能安全混用".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, DbBackend, Statement};
    use serde_json::{Value, json};

    #[tokio::test]
    async fn repairs_legacy_usage_without_guessing_or_recounting_current_calls() {
        let db = crate::test_support::connect_test_database().await;
        let manager = SchemaManager::new(&db);
        db.execute_unprepared(r#"
            INSERT INTO llm_call_trace (workflow, step_key, provider, prompt_tokens,
                completion_tokens, total_tokens, prompt_cache_hit_tokens,
                prompt_cache_miss_tokens, prompt_cache_write_tokens, streamed, status, created_at)
            SELECT 'cache_repair', fixture.step, fixture.provider, fixture.input, 50,
                fixture.input + 50, fixture.hit, fixture.miss, fixture.write, FALSE, 'completed',
                (to_timestamp(migration.applied_at) AT TIME ZONE 'UTC') + fixture.age * INTERVAL '1 hour'
            FROM seaql_migrations migration CROSS JOIN (VALUES
                ('legacy', 'anthropic', 100, 900, 200, NULL::integer, -1),
                ('zero_write', 'anthropic', 100, 900, 0, NULL, -1),
                ('custom', ' Custom-Anthropic ', 20, 60, 20, NULL, -1),
                ('ambiguous', 'anthropic', 10, 4, 6, NULL, -1),
                ('unknown', 'anthropic', 100, NULL, NULL, NULL, -1),
                ('current_unknown', 'anthropic', 1000, 900, 100, NULL, 1),
                ('cutoff_boundary', 'anthropic', 1000, 900, 100, NULL, 0),
                ('already_corrected', 'anthropic', 1200, 900, 300, 200, -1),
                ('other_protocol', 'openai', 1000, 900, 100, NULL, -1)
            ) fixture(step, provider, input, hit, miss, write, age)
            WHERE migration.version = 'llm_cache_usage';
        "#).await.unwrap();
        let snapshot = || async {
            db.query_one(Statement::from_string(DbBackend::Postgres, r#"
                SELECT jsonb_object_agg(step_key, jsonb_build_array(prompt_tokens, total_tokens,
                    prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_cache_write_tokens)) AS usage
                FROM llm_call_trace WHERE workflow = 'cache_repair'
            "#.to_string())).await.unwrap().unwrap().try_get::<Value>("", "usage").unwrap()
        };
        Migration.up(&manager).await.unwrap();
        let usage = snapshot().await;
        assert_eq!(usage["legacy"], json!([1200, 1250, 900, 300, 200]));
        assert_eq!(usage["zero_write"], json!([1000, 1050, 900, 100, 0]));
        assert_eq!(usage["custom"], json!([100, 150, 60, 40, 20]));
        assert_eq!(usage["ambiguous"], json!([null, null, 4, null, null]));
        assert_eq!(usage["unknown"], json!([null, null, null, null, null]));
        for key in ["current_unknown", "cutoff_boundary", "other_protocol"] {
            assert_eq!(usage[key], json!([1000, 1050, 900, 100, null]));
        }
        assert_eq!(
            usage["already_corrected"],
            json!([1200, 1250, 900, 300, 200])
        );
        let ratio = db.query_one(Statement::from_string(DbBackend::Postgres,
            "SELECT prompt_cache_hit_ratio, completion_tokens FROM llm_call_trace WHERE workflow = 'cache_repair' AND step_key = 'legacy'".to_string()))
            .await.unwrap().unwrap();
        assert_eq!(
            ratio.try_get::<f64>("", "prompt_cache_hit_ratio").unwrap(),
            0.75
        );
        assert_eq!(ratio.try_get::<i32>("", "completion_tokens").unwrap(), 50);
        Migration.up(&manager).await.unwrap();
        assert_eq!(snapshot().await, usage);
        assert!(
            Migration
                .down(&manager)
                .await
                .unwrap_err()
                .to_string()
                .contains("不可逆")
        );
    }
}
