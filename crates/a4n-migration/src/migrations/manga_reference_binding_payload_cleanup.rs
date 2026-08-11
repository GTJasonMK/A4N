//! 收敛漫画角色参考图绑定，只保留角色身份和成图所需素材字段。

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "manga_reference_binding_payload_cleanup";

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
            "{MIGRATION_NAME} 不可逆：删除的绑定快照字段没有生产消费者"
        )))
    }
}

const UP_SQL: &str = r#"
DO $$
BEGIN
    IF to_regclass(current_schema() || '.chapter_manga_prompt') IS NULL THEN
        RAISE EXCEPTION
            'manga_reference_binding_payload_cleanup: chapter_manga_prompt table is missing';
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
            'manga_reference_binding_payload_cleanup: invalid reference_bindings container';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM chapter_manga_prompt AS prompt
        CROSS JOIN LATERAL json_array_elements(prompt.layout->'reference_bindings') AS binding
        WHERE prompt.prompt_type = 'page_generation'
          AND json_typeof(binding) IS DISTINCT FROM 'object'
    ) THEN
        RAISE EXCEPTION
            'manga_reference_binding_payload_cleanup: reference binding must be an object';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM chapter_manga_prompt AS prompt
        CROSS JOIN LATERAL json_array_elements(prompt.layout->'reference_bindings') AS binding
        WHERE prompt.prompt_type = 'page_generation'
          AND (
              json_typeof(binding->'slot_id') IS DISTINCT FROM 'string'
              OR btrim(binding->>'slot_id') = ''
              OR json_typeof(binding->'label') IS DISTINCT FROM 'string'
              OR btrim(binding->>'label') = ''
              OR binding->>'target_type' IS DISTINCT FROM 'character'
              OR binding->>'target_name' IS DISTINCT FROM binding->>'label'
              OR binding->>'slot_id' IS DISTINCT FROM ('character:' || (binding->>'label'))
              OR binding->>'purpose' IS DISTINCT FROM '保持本页角色外观一致'
              OR binding->>'required' IS DISTINCT FROM 'true'
              OR binding->>'asset_kind' IS NULL
              OR binding->>'asset_kind' NOT IN ('character_portrait', 'generated_image')
              OR (
                  json_typeof(binding->'bound_asset') IS NOT NULL
                  AND json_typeof(binding->'bound_asset') NOT IN ('null', 'object')
              )
              OR EXISTS (
                  SELECT 1
                  FROM json_object_keys(binding) AS binding_key
                  WHERE binding_key NOT IN (
                      'slot_id',
                      'target_type',
                      'target_name',
                      'label',
                      'purpose',
                      'required',
                      'asset_kind',
                      'bound_asset'
                  )
              )
          )
    ) THEN
        RAISE EXCEPTION
            'manga_reference_binding_payload_cleanup: unsupported reference binding contract';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM chapter_manga_prompt AS prompt
        CROSS JOIN LATERAL json_array_elements(prompt.layout->'reference_bindings') AS binding
        WHERE prompt.prompt_type = 'page_generation'
          AND json_typeof(binding->'bound_asset') = 'object'
          AND (
              json_typeof(binding->'bound_asset'->'asset_kind') IS DISTINCT FROM 'string'
              OR binding->'bound_asset'->>'asset_kind' NOT IN (
                  'character_portrait',
                  'generated_image'
              )
              OR binding->>'asset_kind' IS DISTINCT FROM binding->'bound_asset'->>'asset_kind'
              OR (
                  binding->'bound_asset'->>'asset_id' ~ '^[1-9][0-9]*$'
              ) IS DISTINCT FROM TRUE
              OR json_typeof(binding->'bound_asset'->'image_path') IS DISTINCT FROM 'string'
              OR btrim(binding->'bound_asset'->>'image_path') = ''
              OR (
                  binding->'bound_asset'->>'portrait_id' IS NOT NULL
                  AND binding->'bound_asset'->>'portrait_id'
                      IS DISTINCT FROM binding->'bound_asset'->>'asset_id'
              )
              OR (
                  binding->'bound_asset'->>'image_id' IS NOT NULL
                  AND binding->'bound_asset'->>'image_id'
                      IS DISTINCT FROM binding->'bound_asset'->>'asset_id'
              )
              OR EXISTS (
                  SELECT 1
                  FROM json_object_keys(
                      CASE
                          WHEN json_typeof(binding->'bound_asset') = 'object'
                              THEN binding->'bound_asset'
                          ELSE '{}'::json
                      END
                  ) AS asset_key
                  WHERE asset_key NOT IN (
                      'asset_kind',
                      'asset_id',
                      'label',
                      'image_path',
                      'prompt',
                      'portrait_id',
                      'profile_id',
                      'portrait_type',
                      'expression',
                      'pose',
                      'is_primary',
                      'image_id',
                      'image_type',
                      'chapter_id',
                      'panel_id',
                      'is_favorite'
                  )
              )
          )
    ) THEN
        RAISE EXCEPTION
            'manga_reference_binding_payload_cleanup: invalid bound reference asset';
    END IF;
END $$;

UPDATE chapter_manga_prompt AS prompt
SET layout = jsonb_set(
    prompt.layout::jsonb - 'reference_assets',
    '{reference_bindings}',
    COALESCE(
        (
            SELECT jsonb_agg(
                jsonb_build_object(
                    'character_name', entry.value->'label',
                    'bound_asset',
                    CASE
                        WHEN jsonb_typeof(entry.value->'bound_asset') = 'object' THEN
                            jsonb_build_object(
                                'asset_kind', entry.value->'bound_asset'->'asset_kind',
                                'asset_id', entry.value->'bound_asset'->'asset_id',
                                'image_path', entry.value->'bound_asset'->'image_path'
                            )
                        ELSE 'null'::jsonb
                    END
                )
                ORDER BY entry.ordinality
            )
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
    use serde_json::{Value, json};

    #[tokio::test]
    async fn migration_keeps_only_active_reference_binding_fields() {
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
                '{
                    "reference_assets":{"character_portraits":[{"portrait_id":7}]},
                    "reference_bindings":[
                        {
                            "slot_id":"character:林岚",
                            "target_type":"character",
                            "target_name":"林岚",
                            "label":"林岚",
                            "purpose":"保持本页角色外观一致",
                            "required":true,
                            "asset_kind":"character_portrait"
                        },
                        {
                            "slot_id":"character:周衡",
                            "target_type":"character",
                            "target_name":"周衡",
                            "label":"周衡",
                            "purpose":"保持本页角色外观一致",
                            "required":true,
                            "asset_kind":"character_portrait",
                            "bound_asset":{
                                "asset_kind":"character_portrait",
                                "asset_id":7,
                                "portrait_id":7,
                                "profile_id":3,
                                "label":"周衡",
                                "image_path":"portraits/zhou-heng.png",
                                "prompt":"青年角色",
                                "portrait_type":"正面",
                                "expression":null,
                                "pose":null,
                                "is_primary":true
                            }
                        }
                    ]
                }'::json
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
        assert!(layout.get("reference_assets").is_none());
        let bindings = layout["reference_bindings"].as_array().unwrap();
        assert!(
            bindings
                .iter()
                .all(|binding| binding.as_object().unwrap().len() == 2)
        );
        assert_eq!(bindings[0]["character_name"], "林岚");
        assert_eq!(bindings[0]["bound_asset"], Value::Null);
        assert_eq!(
            bindings[1]["bound_asset"],
            json!({
                "asset_kind": "character_portrait",
                "asset_id": 7,
                "image_path": "portraits/zhou-heng.png"
            })
        );
    }
}
