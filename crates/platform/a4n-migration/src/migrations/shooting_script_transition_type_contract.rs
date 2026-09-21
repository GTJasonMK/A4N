//! 收敛拍摄剧本转场类型并在数据库层拒绝自由文本分叉。

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

use super::shooting_script_transition_type_canonicalization::{
    canonicalize_shooting_script_checkpoints, canonicalize_stored_scripts,
};

const MIGRATION_NAME: &str = "shooting_script_transition_type_contract";

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
        canonicalize_stored_scripts(manager).await?;
        canonicalize_shooting_script_checkpoints(manager).await?;
        manager
            .get_connection()
            .execute_unprepared(TRANSITION_TYPE_CONSTRAINT_SQL)
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(format!(
            "{MIGRATION_NAME} 不可逆：自由文本转场类型已收敛为唯一枚举"
        )))
    }
}

const TRANSITION_TYPE_CONSTRAINT_SQL: &str = r#"
ALTER TABLE chapter_shooting_script
ADD CONSTRAINT ck_chapter_shooting_script_transition_types_v1
CHECK (
    jsonb_path_query_array(
        script_json,
        '$.scenes[*].transition_contracts[*].transition_type'
    ) <@ '["continuous","hard_cut","match_cut","whip_pan","action_wipe","sound_bridge","dissolve","fade","focus_transition","morph"]'::jsonb
)
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::shooting_script_transition_type_canonicalization::normalize_transition_types;
    use a4n_domain::shooting_script::ShootingTransitionTypeV1;
    use sea_orm::Statement;
    use serde_json::json;

    #[test]
    fn canonicalizes_all_observed_transition_aliases_and_rejects_unknown_values() {
        let mut value = json!({
            "items": [
                { "transition_type": "action_continuity" },
                { "transition_type": "cut" },
                { "transition_type": "match on action" },
                { "transition_type": "sound_cut" },
                { "transition_type": "time_ellipsis" },
                { "transition_type": "意识回归" }
            ]
        });
        assert_eq!(normalize_transition_types(&mut value).unwrap(), 6);
        assert_eq!(value["items"][0]["transition_type"], "continuous");
        assert_eq!(value["items"][1]["transition_type"], "hard_cut");
        assert_eq!(value["items"][2]["transition_type"], "match_cut");
        assert_eq!(value["items"][3]["transition_type"], "sound_bridge");
        assert_eq!(value["items"][4]["transition_type"], "dissolve");
        assert_eq!(value["items"][5]["transition_type"], "focus_transition");

        assert_eq!(
            normalize_transition_types(&mut json!({ "transition_type": "model_invented" })),
            Err("model_invented".to_string())
        );
    }

    #[test]
    fn database_constraint_lists_the_domain_enum_values() {
        for value in ShootingTransitionTypeV1::VALUES {
            assert!(TRANSITION_TYPE_CONSTRAINT_SQL.contains(&format!("\"{value}\"")));
        }
    }

    #[tokio::test]
    async fn migrated_database_rejects_noncanonical_transition_type() {
        let db = crate::test_support::connect_test_database().await;
        let result = db
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"
INSERT INTO chapter_shooting_script (
    root_project_id,
    source_chapter_id,
    source_snapshot_id,
    source_snapshot_chapter_id,
    source_revision_key,
    source_content_sha256,
    input_fingerprint_sha256,
    contract_version,
    revision,
    script_sha256,
    script_json,
    workflow_run_id,
    created_at,
    updated_at
) VALUES (
    1,
    1,
    NULL,
    NULL,
    'test:invalid-transition',
    repeat('a', 64),
    repeat('b', 64),
    'chapter_shooting_script_v1',
    1,
    repeat('c', 64),
    $1,
    NULL,
    NOW(),
    NOW()
)
"#,
                [json!({
                    "scenes": [{
                        "transition_contracts": [{ "transition_type": "cut" }]
                    }]
                })
                .into()],
            ))
            .await;

        assert!(result.is_err());
    }
}
