//! 删除漫画参考图绑定中与 bound_asset 重复的 status 状态。

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "manga_reference_binding_contract_cleanup";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP_SQL).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(format!(
            "{MIGRATION_NAME} 不可逆：status 是 bound_asset 存在性的重复状态"
        )))
    }
}

const UP_SQL: &str = r#"
DO $$
BEGIN
    IF to_regclass(current_schema() || '.chapter_manga_prompt') IS NULL THEN
        RAISE EXCEPTION
            'manga_reference_binding_contract_cleanup: chapter_manga_prompt table is missing';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM chapter_manga_prompt
        WHERE prompt_type = 'page_generation'
          AND (
              json_typeof(layout) IS DISTINCT FROM 'object'
              OR json_typeof(layout->'reference_bindings') IS DISTINCT FROM 'array'
          )
    ) THEN
        RAISE EXCEPTION
            'manga_reference_binding_contract_cleanup: invalid reference_bindings container';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM chapter_manga_prompt AS prompt
        CROSS JOIN LATERAL json_array_elements(prompt.layout->'reference_bindings') AS binding
        WHERE prompt.prompt_type = 'page_generation'
          AND (
              json_typeof(binding) IS DISTINCT FROM 'object'
              OR binding->>'status' IS NULL
              OR binding->>'status' NOT IN ('missing', 'bound')
              OR (
                  binding->>'status' = 'bound'
                  AND json_typeof(binding->'bound_asset') IS DISTINCT FROM 'object'
              )
              OR (
                  binding->>'status' = 'missing'
                  AND json_typeof(binding->'bound_asset') IS NOT NULL
                  AND json_typeof(binding->'bound_asset') <> 'null'
              )
          )
    ) THEN
        RAISE EXCEPTION
            'manga_reference_binding_contract_cleanup: status and bound_asset are inconsistent';
    END IF;
END $$;

UPDATE chapter_manga_prompt AS prompt
SET layout = jsonb_set(
    prompt.layout::jsonb,
    '{reference_bindings}',
    COALESCE(
        (
            SELECT jsonb_agg(entry.value - 'status' ORDER BY entry.ordinality)
            FROM jsonb_array_elements(prompt.layout::jsonb->'reference_bindings')
                WITH ORDINALITY AS entry(value, ordinality)
        ),
        '[]'::jsonb
    ),
    false
)::json
WHERE prompt.prompt_type = 'page_generation';
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DbBackend, Statement};
    use serde_json::Value;

    #[tokio::test]
    async fn migration_removes_status_and_preserves_bound_asset() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            r#"
            DROP TABLE chapter_manga_prompt CASCADE;
            CREATE TABLE chapter_manga_prompt (
                id BIGSERIAL PRIMARY KEY,
                prompt_type TEXT NOT NULL,
                layout JSON
            );
            INSERT INTO chapter_manga_prompt (prompt_type, layout)
            VALUES (
                'page_generation',
                '{"reference_bindings":[
                    {"slot_id":"missing","status":"missing"},
                    {"slot_id":"bound","status":"bound","bound_asset":{"asset_id":7}}
                ]}'::json
            );
            "#,
        )
        .await
        .unwrap();

        Migration.up(&SchemaManager::new(&db)).await.unwrap();

        let row = db
            .query_one(Statement::from_string(
                DbBackend::Postgres,
                "SELECT layout FROM chapter_manga_prompt WHERE prompt_type = 'page_generation'",
            ))
            .await
            .unwrap()
            .unwrap();
        let layout = row.try_get::<Value>("", "layout").unwrap();
        let bindings = layout["reference_bindings"].as_array().unwrap();
        assert!(
            bindings
                .iter()
                .all(|binding| binding.get("status").is_none())
        );
        assert_eq!(bindings[1]["bound_asset"]["asset_id"], 7);
    }
}
