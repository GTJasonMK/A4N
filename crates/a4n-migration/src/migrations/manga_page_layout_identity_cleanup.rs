//! 删除漫画页面 layout 顶层重复的拍摄剧本身份。

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "manga_page_layout_identity_cleanup";

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
            "{MIGRATION_NAME} 不可逆：layout 顶层拍摄剧本身份是 source 的重复副本"
        )))
    }
}

const UP_SQL: &str = r#"
DO $$
BEGIN
    IF to_regclass(current_schema() || '.chapter_manga_prompt') IS NULL
        OR to_regclass(current_schema() || '.generation_spec') IS NULL
        OR to_regclass(current_schema() || '.generation_run') IS NULL
    THEN
        RAISE EXCEPTION
            'manga_page_layout_identity_cleanup: required table is missing';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM (
            SELECT layout::jsonb AS layout
            FROM chapter_manga_prompt
            WHERE prompt_type = 'page_generation'

            UNION ALL

            SELECT params_json::jsonb->'layout'
            FROM generation_spec
            WHERE target_kind = 'manga_page'
              AND target_entity_kind = 'chapter_manga_prompt'

            UNION ALL

            SELECT parsed_output_json::jsonb->'layout'
            FROM generation_run
            WHERE workflow = 'manga_prompt_generation'
        ) AS candidate
        WHERE candidate.layout ?| ARRAY[
            'shooting_script_id',
            'shooting_script_revision'
        ]
          AND (
              (candidate.layout ? 'shooting_script_id')
                  IS DISTINCT FROM
              (candidate.layout ? 'shooting_script_revision')
              OR jsonb_typeof(candidate.layout->'shooting_script_id')
                  IS DISTINCT FROM 'number'
              OR jsonb_typeof(candidate.layout->'shooting_script_revision')
                  IS DISTINCT FROM 'number'
              OR (candidate.layout->>'shooting_script_id' ~ '^[1-9][0-9]*$')
                  IS DISTINCT FROM TRUE
              OR (candidate.layout->>'shooting_script_revision' ~ '^[1-9][0-9]*$')
                  IS DISTINCT FROM TRUE
              OR jsonb_typeof(candidate.layout->'source')
                  IS DISTINCT FROM 'object'
              OR jsonb_typeof(candidate.layout->'source'->'shooting_script_id')
                  IS DISTINCT FROM 'number'
              OR jsonb_typeof(candidate.layout->'source'->'shooting_script_revision')
                  IS DISTINCT FROM 'number'
              OR (candidate.layout->'source'->>'shooting_script_id' ~ '^[1-9][0-9]*$')
                  IS DISTINCT FROM TRUE
              OR (candidate.layout->'source'->>'shooting_script_revision' ~ '^[1-9][0-9]*$')
                  IS DISTINCT FROM TRUE
              OR candidate.layout->>'shooting_script_id'
                  IS DISTINCT FROM
                  candidate.layout->'source'->>'shooting_script_id'
              OR candidate.layout->>'shooting_script_revision'
                  IS DISTINCT FROM
                  candidate.layout->'source'->>'shooting_script_revision'
          )
    ) THEN
        RAISE EXCEPTION
            'manga_page_layout_identity_cleanup: partial, invalid, or conflicting layout shooting script identity';
    END IF;
END $$;

UPDATE chapter_manga_prompt
SET layout = (
    layout::jsonb - ARRAY['shooting_script_id', 'shooting_script_revision']
)::json
WHERE prompt_type = 'page_generation'
  AND layout::jsonb ?& ARRAY['shooting_script_id', 'shooting_script_revision'];

UPDATE generation_spec
SET params_json = jsonb_set(
    params_json::jsonb,
    '{layout}',
    (params_json::jsonb->'layout')
        - ARRAY['shooting_script_id', 'shooting_script_revision'],
    false
)::json
WHERE target_kind = 'manga_page'
  AND target_entity_kind = 'chapter_manga_prompt'
  AND (params_json::jsonb->'layout')
      ?& ARRAY['shooting_script_id', 'shooting_script_revision'];

UPDATE generation_run
SET parsed_output_json = jsonb_set(
    parsed_output_json::jsonb,
    '{layout}',
    (parsed_output_json::jsonb->'layout')
        - ARRAY['shooting_script_id', 'shooting_script_revision'],
    false
)::json
WHERE workflow = 'manga_prompt_generation'
  AND (parsed_output_json::jsonb->'layout')
      ?& ARRAY['shooting_script_id', 'shooting_script_revision'];
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Statement;

    #[tokio::test]
    async fn migration_removes_only_consistent_layout_identity_copies() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            r#"
            DROP TABLE chapter_manga_prompt CASCADE;
            DROP TABLE generation_spec CASCADE;
            DROP TABLE generation_run CASCADE;

            CREATE TABLE chapter_manga_prompt (
                id BIGSERIAL PRIMARY KEY,
                prompt_type TEXT NOT NULL,
                layout JSON
            );
            CREATE TABLE generation_spec (
                id BIGSERIAL PRIMARY KEY,
                target_kind TEXT NOT NULL,
                target_entity_kind TEXT NOT NULL,
                params_json JSON NOT NULL
            );
            CREATE TABLE generation_run (
                id BIGSERIAL PRIMARY KEY,
                workflow TEXT NOT NULL,
                parsed_output_json JSON
            );

            INSERT INTO chapter_manga_prompt (prompt_type, layout)
            VALUES (
                'page_generation',
                '{
                    "shooting_script_id":71,
                    "shooting_script_revision":2,
                    "source":{
                        "shooting_script_id":71,
                        "shooting_script_revision":2
                    }
                }'
            ), (
                'page_generation',
                '{
                    "source":{
                        "shooting_script_id":72,
                        "shooting_script_revision":3
                    }
                }'
            );
            INSERT INTO generation_spec (
                target_kind,
                target_entity_kind,
                params_json
            ) VALUES (
                'manga_page',
                'chapter_manga_prompt',
                '{
                    "layout":{
                        "shooting_script_id":71,
                        "shooting_script_revision":2,
                        "source":{
                            "shooting_script_id":71,
                            "shooting_script_revision":2
                        }
                    }
                }'
            );
            INSERT INTO generation_run (workflow, parsed_output_json)
            VALUES (
                'manga_prompt_generation',
                '{
                    "layout":{
                        "shooting_script_id":71,
                        "shooting_script_revision":2,
                        "source":{
                            "shooting_script_id":71,
                            "shooting_script_revision":2
                        }
                    }
                }'
            );
            "#,
        )
        .await
        .unwrap();

        Migration.up(&SchemaManager::new(&db)).await.unwrap();

        let row = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                r#"
                SELECT
                    NOT EXISTS (
                        SELECT 1
                        FROM chapter_manga_prompt
                        WHERE layout::jsonb ?| ARRAY[
                            'shooting_script_id',
                            'shooting_script_revision'
                        ]
                    ) AS prompt_clean,
                    NOT EXISTS (
                        SELECT 1
                        FROM generation_spec
                        WHERE params_json::jsonb->'layout' ?| ARRAY[
                            'shooting_script_id',
                            'shooting_script_revision'
                        ]
                    ) AS spec_clean,
                    NOT EXISTS (
                        SELECT 1
                        FROM generation_run
                        WHERE parsed_output_json::jsonb->'layout' ?| ARRAY[
                            'shooting_script_id',
                            'shooting_script_revision'
                        ]
                    ) AS run_clean,
                    (
                        SELECT layout::jsonb->'source'->>'shooting_script_id'
                        FROM chapter_manga_prompt
                        ORDER BY id
                        LIMIT 1
                    ) = '71' AS source_preserved
                "#
                .to_string(),
            ))
            .await
            .unwrap()
            .unwrap();
        for column in [
            "prompt_clean",
            "spec_clean",
            "run_clean",
            "source_preserved",
        ] {
            assert!(row.try_get::<bool>("", column).unwrap(), "{column}");
        }

        db.execute_unprepared(
            r#"
            INSERT INTO chapter_manga_prompt (prompt_type, layout)
            VALUES (
                'page_generation',
                '{
                    "shooting_script_id":71,
                    "source":{
                        "shooting_script_id":71,
                        "shooting_script_revision":2
                    }
                }'
            );
            "#,
        )
        .await
        .unwrap();
        let error = Migration
            .up(&SchemaManager::new(&db))
            .await
            .expect_err("partial identity must fail explicitly");
        assert!(
            error
                .to_string()
                .contains("partial, invalid, or conflicting")
        );

        db.execute_unprepared(
            r#"
            UPDATE chapter_manga_prompt
            SET layout = jsonb_set(
                layout::jsonb,
                '{shooting_script_revision}',
                '3'::jsonb,
                true
            )::json
            WHERE id = (SELECT MAX(id) FROM chapter_manga_prompt);
            "#,
        )
        .await
        .unwrap();
        let error = Migration
            .up(&SchemaManager::new(&db))
            .await
            .expect_err("conflicting identity must fail explicitly");
        assert!(
            error
                .to_string()
                .contains("partial, invalid, or conflicting")
        );

        let error = Migration
            .down(&SchemaManager::new(&db))
            .await
            .expect_err("migration is irreversible");
        assert!(error.to_string().contains("不可逆"));
    }
}
