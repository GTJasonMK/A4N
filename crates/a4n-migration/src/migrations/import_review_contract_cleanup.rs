//! 收敛正文导入审核为单一 JSON 合同和四态状态机。

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "import_review_contract_cleanup";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(format!(
                "A4N {MIGRATION_NAME} migration only supports PostgreSQL"
            )));
        }
        manager.get_connection().execute_unprepared(UP_SQL).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(format!(
            "{MIGRATION_NAME} 不可逆：旧导入统计列、状态和 JSON 别名已永久移除"
        )))
    }
}

const UP_SQL: &str = r#"
DO $$
BEGIN
    IF to_regclass(current_schema() || '.import_review_queue') IS NULL THEN
        RAISE EXCEPTION
            'import_review_contract_cleanup: import_review_queue table is missing';
    END IF;

    IF EXISTS (
        SELECT 1 FROM import_review_queue WHERE analysis_result IS NULL
    ) THEN
        RAISE EXCEPTION
            'import_review_contract_cleanup: analysis_result contains NULL rows';
    END IF;
END $$;

UPDATE import_review_queue
SET review_status = 'pending'
WHERE review_status IN ('analyzing', 'reviewing');

UPDATE import_review_queue
SET analysis_result = (
    (
        analysis_result::jsonb
        || jsonb_strip_nulls(jsonb_build_object(
            'chapter_title', COALESCE(
                analysis_result::jsonb->'modified_title',
                analysis_result::jsonb->'chapter_title'
            ),
            'content', COALESCE(
                analysis_result::jsonb->'modified_content',
                analysis_result::jsonb->'content'
            ),
            'summary', COALESCE(
                analysis_result::jsonb->'modified_summary',
                analysis_result::jsonb->'summary'
            )
        ))
    ) - ARRAY[
        'modified_title',
        'modified_content',
        'modified_summary',
        'modified_characters',
        'modified_world_setting',
        'content_preview',
        'content_hash'
    ]
)::json;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM import_review_queue
        WHERE review_status NOT IN ('pending', 'accepted', 'rejected', 'modified')
    ) THEN
        RAISE EXCEPTION
            'import_review_contract_cleanup: unsupported review_status remains';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM import_review_queue
        WHERE json_typeof(analysis_result) IS DISTINCT FROM 'object'
           OR COALESCE(analysis_result->>'chapter_number', '') !~ '^[1-9][0-9]*$'
           OR CASE
                WHEN COALESCE(analysis_result->>'chapter_number', '') ~ '^[1-9][0-9]*$'
                THEN (analysis_result->>'chapter_number')::numeric > 2147483647
                ELSE FALSE
              END
           OR NULLIF(BTRIM(analysis_result->>'chapter_title'), '') IS NULL
           OR NULLIF(BTRIM(analysis_result->>'content'), '') IS NULL
           OR analysis_result::jsonb ?| ARRAY[
                'modified_title',
                'modified_content',
                'modified_summary',
                'modified_characters',
                'modified_world_setting',
                'content_preview',
                'content_hash'
           ]
    ) THEN
        RAISE EXCEPTION
            'import_review_contract_cleanup: invalid canonical analysis_result remains';
    END IF;
END $$;

ALTER TABLE import_review_queue
    ALTER COLUMN analysis_result SET NOT NULL,
    DROP COLUMN IF EXISTS file_type,
    DROP COLUMN IF EXISTS detected_encoding,
    DROP COLUMN IF EXISTS parsed_chapters,
    DROP COLUMN IF EXISTS dedup_paragraphs,
    DROP COLUMN IF EXISTS dedup_rate;

ALTER TABLE import_review_queue
    DROP CONSTRAINT IF EXISTS import_review_queue_review_status_check;

ALTER TABLE import_review_queue
    ADD CONSTRAINT import_review_queue_review_status_check
    CHECK (review_status IN ('pending', 'accepted', 'rejected', 'modified'));
"#;

#[cfg(test)]
mod tests {
    use super::UP_SQL;
    use sea_orm::{ConnectionTrait, Statement};

    #[tokio::test]
    async fn migration_canonicalizes_legacy_rows_and_removes_old_contract() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            r#"
ALTER TABLE import_review_queue
    DROP CONSTRAINT IF EXISTS import_review_queue_review_status_check,
    ADD COLUMN file_type TEXT NOT NULL DEFAULT 'txt',
    ADD COLUMN detected_encoding TEXT,
    ADD COLUMN parsed_chapters INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN dedup_paragraphs INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN dedup_rate REAL;

INSERT INTO import_review_queue (
    project_id,
    filename,
    analysis_result,
    review_status,
    created_at,
    updated_at
) VALUES (
    42,
    'legacy.txt',
    '{
        "chapter_number": 1,
        "chapter_title": "旧标题",
        "content": "旧正文",
        "modified_title": "新标题",
        "modified_content": "新正文",
        "modified_summary": "新摘要",
        "modified_characters": ["旧角色"],
        "modified_world_setting": {"旧": true},
        "content_preview": "预览",
        "content_hash": "legacy"
    }'::json,
    'reviewing',
    NOW(),
    NOW()
);
"#,
        )
        .await
        .expect("legacy review fixture must be created");

        db.execute_unprepared(UP_SQL)
            .await
            .expect("cleanup migration must succeed");

        let row = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT review_status, analysis_result FROM import_review_queue WHERE filename = 'legacy.txt'"
                    .to_string(),
            ))
            .await
            .expect("canonical review query must succeed")
            .expect("canonical review must remain");
        let review_status: String = row.try_get("", "review_status").expect("review status");
        let analysis_result: serde_json::Value =
            row.try_get("", "analysis_result").expect("analysis result");
        assert_eq!(review_status, "pending");
        assert_eq!(analysis_result["chapter_title"], "新标题");
        assert_eq!(analysis_result["content"], "新正文");
        assert_eq!(analysis_result["summary"], "新摘要");
        for removed in [
            "modified_title",
            "modified_content",
            "modified_summary",
            "modified_characters",
            "modified_world_setting",
            "content_preview",
            "content_hash",
        ] {
            assert!(
                analysis_result.get(removed).is_none(),
                "legacy key remains: {removed}"
            );
        }

        let old_column_count = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                r#"
SELECT COUNT(*)::bigint AS count
FROM information_schema.columns
WHERE table_schema = current_schema()
  AND table_name = 'import_review_queue'
  AND column_name IN (
      'file_type',
      'detected_encoding',
      'parsed_chapters',
      'dedup_paragraphs',
      'dedup_rate'
  )
"#
                .to_string(),
            ))
            .await
            .expect("column inventory query must succeed")
            .expect("column inventory must return a row")
            .try_get::<i64>("", "count")
            .expect("old column count");
        assert_eq!(old_column_count, 0);

        assert!(
            db.execute_unprepared(
                "UPDATE import_review_queue SET review_status = 'reviewing' WHERE filename = 'legacy.txt'"
            )
            .await
            .is_err()
        );
        assert!(
            db.execute_unprepared(
                "UPDATE import_review_queue SET analysis_result = NULL WHERE filename = 'legacy.txt'"
            )
            .await
            .is_err()
        );
    }
}
