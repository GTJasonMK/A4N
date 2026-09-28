//! 收敛 Galgame 行演出 JSON，并重新安装唯一的数据库合同。

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

use super::{
    galgame_generation_spec_contract::GALGAME_GENERATION_SPEC_FAMILY_FUNCTION_SQL,
    galgame_script_integrity_contract::GALGAME_LINE_FAMILY_FUNCTION_SQL,
    galgame_target_reverse_integrity_contract::GALGAME_TARGET_REVERSE_INTEGRITY_FUNCTION_SQL,
};

const MIGRATION_NAME: &str = "galgame_presentation_contract_cleanup";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in [
            PRESENTATION_PREFLIGHT_SQL,
            GALGAME_LINE_FAMILY_FUNCTION_SQL,
            GALGAME_GENERATION_SPEC_FAMILY_FUNCTION_SQL,
            GALGAME_TARGET_REVERSE_INTEGRITY_FUNCTION_SQL,
            COMPLETE_PRESENTATION_SQL,
            VALIDATE_PRESENTATION_SQL,
            VALIDATE_GALGAME_GENERATION_SPEC_SQL,
        ] {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(format!(
            "{MIGRATION_NAME} 不可逆：Galgame 演出与 CG 生成合同不再支持旧格式"
        )))
    }
}

const PRESENTATION_PREFLIGHT_SQL: &str = r#"
DO $$
BEGIN
    IF to_regclass(current_schema() || '.galgame_line') IS NULL
       OR to_regclass(current_schema() || '.generation_spec') IS NULL THEN
        RAISE EXCEPTION
            'galgame_presentation_contract_cleanup: required table is missing';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM galgame_line
        WHERE json_typeof(presentation_json) IS DISTINCT FROM 'object'
           OR presentation_json->>'schema_version'
              IS DISTINCT FROM 'galgame_line_presentation_v2'
    ) THEN
        RAISE EXCEPTION
            'galgame_presentation_contract_cleanup: only v2 presentation rows are supported';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM galgame_line
        WHERE presentation_json::jsonb ?| ARRAY['cg_prompt', 'event_cg_note']
    ) THEN
        RAISE EXCEPTION
            'galgame_presentation_contract_cleanup: legacy CG aliases are not supported';
    END IF;
END $$;
"#;

const COMPLETE_PRESENTATION_SQL: &str = r#"
UPDATE galgame_line
SET presentation_json = (
    jsonb_build_object(
        'focus_character_profile_id', NULL,
        'cg', NULL,
        'cg_note', NULL,
        'cg_importance', NULL,
        'transition', NULL,
        'branch', NULL,
        'branch_id', NULL,
        'returns_to_scene_index', NULL,
        'after_beat_ref', NULL
    ) || presentation_json::jsonb
)::json
WHERE NOT (presentation_json::jsonb ?& ARRAY[
    'focus_character_profile_id',
    'cg',
    'cg_note',
    'cg_importance',
    'transition',
    'branch',
    'branch_id',
    'returns_to_scene_index',
    'after_beat_ref'
]);
"#;

const VALIDATE_PRESENTATION_SQL: &str = r#"
UPDATE galgame_line
SET presentation_json = presentation_json;
"#;

const VALIDATE_GALGAME_GENERATION_SPEC_SQL: &str = r#"
UPDATE generation_spec
SET params_json = params_json
WHERE (target_kind, target_entity_kind) IN (
    ('galgame_cg', 'galgame_line'),
    ('portrait_expression', 'character_profile')
);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DbBackend, Statement};

    #[test]
    fn migration_rejects_aliases_and_installs_strict_contracts() {
        for expected in [
            "only v2 presentation rows are supported",
            "legacy CG aliases are not supported",
            "'cg_prompt', 'event_cg_note'",
            "'cg_importance', NULL",
            "SET presentation_json = presentation_json",
            "'galgame_cg', 'galgame_line'",
            "'portrait_expression', 'character_profile'",
        ] {
            assert!(
                PRESENTATION_PREFLIGHT_SQL.contains(expected)
                    || COMPLETE_PRESENTATION_SQL.contains(expected)
                    || VALIDATE_PRESENTATION_SQL.contains(expected)
                    || VALIDATE_GALGAME_GENERATION_SPEC_SQL.contains(expected),
                "missing Galgame presentation cleanup contract: {expected}"
            );
        }
    }

    #[tokio::test]
    async fn cleanup_completes_v2_nullable_fields_and_rejects_legacy_aliases() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            r#"
            DROP TABLE galgame_line CASCADE;
            DROP TABLE generation_spec CASCADE;
            CREATE TABLE galgame_line (
                id BIGINT PRIMARY KEY,
                presentation_json JSON NOT NULL
            );
            CREATE TABLE generation_spec (
                id BIGINT PRIMARY KEY,
                target_kind TEXT NOT NULL,
                target_entity_kind TEXT NOT NULL,
                params_json JSON NOT NULL
            );
            INSERT INTO galgame_line (id, presentation_json)
            VALUES (1, '{
                "schema_version":"galgame_line_presentation_v2",
                "visible_cast":[],
                "enter":[],
                "exit":[],
                "hold":false,
                "player_knowledge":[]
            }'::json);
            "#,
        )
        .await
        .unwrap();

        db.execute_unprepared(PRESENTATION_PREFLIGHT_SQL)
            .await
            .unwrap();
        db.execute_unprepared(COMPLETE_PRESENTATION_SQL)
            .await
            .unwrap();

        let row = db
            .query_one(Statement::from_string(
                DbBackend::Postgres,
                "SELECT presentation_json FROM galgame_line WHERE id = 1",
            ))
            .await
            .unwrap()
            .unwrap();
        let presentation = row
            .try_get::<serde_json::Value>("", "presentation_json")
            .unwrap();
        assert_eq!(
            presentation["schema_version"],
            "galgame_line_presentation_v2"
        );
        a4n_domain::media_authoring::legacy::upgrade_galgame_presentation(
            &presentation,
            a4n_domain::galgame::GalgamePipelineBeatRefV4 {
                source_chapter_id: 1,
                unit_id: "legacy-scene-1".into(),
                beat_id: "legacy-line-1".into(),
            },
        )
        .expect("cleanup must produce a complete v2 document accepted by the explicit upgrade");

        db.execute_unprepared(
            r#"
            UPDATE galgame_line
            SET presentation_json = (
                presentation_json::jsonb || '{"cg_prompt":"legacy"}'::jsonb
            )::json
            WHERE id = 1;
            "#,
        )
        .await
        .unwrap();
        assert!(
            db.execute_unprepared(PRESENTATION_PREFLIGHT_SQL)
                .await
                .is_err(),
            "legacy CG aliases must fail before migration writes"
        );
    }
}
