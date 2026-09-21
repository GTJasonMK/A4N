//! 删除已退役的 manga/video 资产谱系影子账本。

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "legacy_asset_lineage_shadow_cleanup";

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
            "{MIGRATION_NAME} 不可逆：退役影子生成意图和媒体记录不会恢复"
        )))
    }
}

const UP_SQL: &str = r#"
DO $$
DECLARE
    required_table TEXT;
BEGIN
    FOREACH required_table IN ARRAY ARRAY[
        'generation_spec',
        'generation_spec_entity_ref',
        'generation_spec_media_ref',
        'generation_spec_text_anchor',
        'prompt_artifact',
        'media_asset',
        'media_provenance',
        'media_binding',
        'assembly_clip',
        'assembly_timeline',
        'video_segment'
    ] LOOP
        IF to_regclass(current_schema() || '.' || required_table) IS NULL THEN
            RAISE EXCEPTION
                'legacy_asset_lineage_shadow_cleanup: required table % is missing',
                required_table;
        END IF;
    END LOOP;
END $$;

DROP TABLE IF EXISTS pg_temp._a4n_legacy_asset_lineage_media;
DROP TABLE IF EXISTS pg_temp._a4n_legacy_asset_lineage_prompt;
DROP TABLE IF EXISTS pg_temp._a4n_legacy_asset_lineage_spec;

CREATE TEMP TABLE _a4n_legacy_asset_lineage_spec ON COMMIT DROP AS
SELECT spec.id
FROM generation_spec AS spec
WHERE COALESCE(spec.params_json::jsonb ->> 'canonical_key', '') LIKE 'legacy:%'
   OR EXISTS (
       SELECT 1
       FROM prompt_artifact AS prompt
       WHERE prompt.generation_spec_id = spec.id
         AND prompt.renderer_version = 'legacy_asset_lineage_sync_v1'
   );
ALTER TABLE _a4n_legacy_asset_lineage_spec ADD PRIMARY KEY (id);

CREATE TEMP TABLE _a4n_legacy_asset_lineage_prompt ON COMMIT DROP AS
SELECT prompt.id, prompt.generation_spec_id
FROM prompt_artifact AS prompt
WHERE prompt.renderer_version = 'legacy_asset_lineage_sync_v1';
ALTER TABLE _a4n_legacy_asset_lineage_prompt ADD PRIMARY KEY (id);

CREATE TEMP TABLE _a4n_legacy_asset_lineage_media ON COMMIT DROP AS
SELECT DISTINCT provenance.media_asset_id AS id
FROM media_provenance AS provenance
WHERE COALESCE(
          provenance.request_snapshot_json::jsonb ->> 'canonical_key',
          ''
      ) LIKE 'legacy:%'
   OR provenance.generation_spec_id IN (
       SELECT id FROM _a4n_legacy_asset_lineage_spec
   )
   OR provenance.prompt_artifact_id IN (
       SELECT id FROM _a4n_legacy_asset_lineage_prompt
   );
ALTER TABLE _a4n_legacy_asset_lineage_media ADD PRIMARY KEY (id);

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM assembly_clip AS clip
        JOIN _a4n_legacy_asset_lineage_media AS legacy
          ON legacy.id = clip.media_asset_id
    ) THEN
        RAISE EXCEPTION
            'legacy_asset_lineage_shadow_cleanup: assembly_clip references legacy media';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM assembly_timeline AS timeline
        JOIN _a4n_legacy_asset_lineage_media AS legacy
          ON legacy.id = timeline.preview_media_asset_id
    ) THEN
        RAISE EXCEPTION
            'legacy_asset_lineage_shadow_cleanup: assembly_timeline references legacy media';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM video_segment AS segment
        WHERE segment.source_generation_spec_id IN (
                  SELECT id FROM _a4n_legacy_asset_lineage_spec
              )
           OR segment.result_media_asset_id IN (
                  SELECT id FROM _a4n_legacy_asset_lineage_media
              )
    ) THEN
        RAISE EXCEPTION
            'legacy_asset_lineage_shadow_cleanup: video_segment references legacy lineage';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM media_provenance AS provenance
        WHERE provenance.media_asset_id NOT IN (
                  SELECT id FROM _a4n_legacy_asset_lineage_media
              )
          AND (
              provenance.generation_spec_id IN (
                  SELECT id FROM _a4n_legacy_asset_lineage_spec
              )
              OR provenance.prompt_artifact_id IN (
                  SELECT id FROM _a4n_legacy_asset_lineage_prompt
              )
              OR provenance.source_media_asset_id IN (
                  SELECT id FROM _a4n_legacy_asset_lineage_media
              )
          )
    ) THEN
        RAISE EXCEPTION
            'legacy_asset_lineage_shadow_cleanup: real media provenance references legacy lineage';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM generation_spec_media_ref AS reference
        WHERE reference.media_asset_id IN (
                  SELECT id FROM _a4n_legacy_asset_lineage_media
              )
          AND reference.generation_spec_id NOT IN (
                  SELECT id FROM _a4n_legacy_asset_lineage_spec
              )
    ) THEN
        RAISE EXCEPTION
            'legacy_asset_lineage_shadow_cleanup: real generation spec references legacy media';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM prompt_artifact AS prompt
        WHERE prompt.generation_spec_id IN (
                  SELECT id FROM _a4n_legacy_asset_lineage_spec
              )
          AND prompt.renderer_version IS DISTINCT FROM 'legacy_asset_lineage_sync_v1'
    ) THEN
        RAISE EXCEPTION
            'legacy_asset_lineage_shadow_cleanup: legacy generation spec has non-legacy prompt';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM generation_spec_entity_ref AS reference
        WHERE reference.generation_spec_id IN (
            SELECT id FROM _a4n_legacy_asset_lineage_spec
        )
    ) OR EXISTS (
        SELECT 1
        FROM generation_spec_text_anchor AS reference
        WHERE reference.generation_spec_id IN (
            SELECT id FROM _a4n_legacy_asset_lineage_spec
        )
    ) THEN
        RAISE EXCEPTION
            'legacy_asset_lineage_shadow_cleanup: legacy generation spec has unsupported external references';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM media_binding AS binding
        WHERE binding.media_asset_id IN (
                  SELECT id FROM _a4n_legacy_asset_lineage_media
              )
          AND binding.target_kind NOT IN (
              'chapter',
              'chapter_manga_prompt',
              'chapter_video_prompt'
          )
    ) THEN
        RAISE EXCEPTION
            'legacy_asset_lineage_shadow_cleanup: legacy media has unsupported binding target';
    END IF;
END $$;

DELETE FROM generation_spec_media_ref
WHERE generation_spec_id IN (
          SELECT id FROM _a4n_legacy_asset_lineage_spec
      )
   OR media_asset_id IN (
          SELECT id FROM _a4n_legacy_asset_lineage_media
      );

DELETE FROM media_binding
WHERE media_asset_id IN (
    SELECT id FROM _a4n_legacy_asset_lineage_media
);

DELETE FROM media_provenance
WHERE media_asset_id IN (
    SELECT id FROM _a4n_legacy_asset_lineage_media
);

DELETE FROM prompt_artifact
WHERE id IN (
    SELECT id FROM _a4n_legacy_asset_lineage_prompt
);

DELETE FROM media_asset
WHERE id IN (
    SELECT id FROM _a4n_legacy_asset_lineage_media
);

DELETE FROM generation_spec
WHERE id IN (
    SELECT id FROM _a4n_legacy_asset_lineage_spec
);

DROP TABLE _a4n_legacy_asset_lineage_media;
DROP TABLE _a4n_legacy_asset_lineage_prompt;
DROP TABLE _a4n_legacy_asset_lineage_spec;
"#;

#[cfg(test)]
mod tests {
    use super::{Migration, UP_SQL};
    use sea_orm::{ConnectionTrait, DbBackend, Statement};
    use sea_orm_migration::prelude::{MigrationTrait, SchemaManager};

    #[tokio::test]
    async fn cleanup_rejects_real_consumers_then_removes_only_shadow_rows() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            r#"
ALTER TABLE generation_spec DISABLE TRIGGER USER;
ALTER TABLE generation_spec_media_ref DISABLE TRIGGER USER;
ALTER TABLE media_asset DISABLE TRIGGER USER;
ALTER TABLE media_provenance DISABLE TRIGGER USER;
ALTER TABLE media_binding DISABLE TRIGGER USER;

INSERT INTO generation_spec (
    id, project_id, root_project_id, source_context_kind, source_snapshot_id,
    source_chapter_id, source_snapshot_chapter_id, canvas_text_asset_id,
    target_kind, target_entity_kind, target_entity_id, status,
    source_version_hash, params_json, created_at, updated_at
) VALUES
    (
        91001, 1, 1, 'freeform', NULL, NULL, NULL, NULL,
        'video_segment', 'chapter_video_prompt', 7, 'completed',
        'legacy-source', '{"canonical_key":"legacy:chapter_video_prompt:7:prompt"}',
        NOW(), NOW()
    ),
    (
        91002, 2, 2, 'freeform', NULL, NULL, NULL, NULL,
        'canvas_image', 'canvas_node', 8, 'completed',
        'canvas-source', '{"canonical_key":"canvas:2:image:8"}',
        NOW(), NOW()
    ),
    (
        91003, 3, 3, 'freeform', NULL, NULL, NULL, NULL,
        'galgame_background', 'galgame_scene', 9, 'completed',
        'galgame-source', '{"canonical_key":"galgame:3:background:scene:9"}',
        NOW(), NOW()
    );

INSERT INTO prompt_artifact (
    id, generation_spec_id, prompt_kind, prompt_text, renderer_version,
    edited_by_user, created_at, updated_at
) VALUES
    (91101, 91001, 'video_prompt', 'legacy', 'legacy_asset_lineage_sync_v1', FALSE, NOW(), NOW()),
    (91102, 91002, 'image_prompt', 'canvas', 'canvas_image_v1', FALSE, NOW(), NOW()),
    (91103, 91003, 'image_prompt', 'galgame', 'galgame_image_v1', FALSE, NOW(), NOW());

INSERT INTO media_asset (
    id, project_id, root_project_id, source_snapshot_id, source_chapter_id,
    source_snapshot_chapter_id, media_kind, origin, file_path, thumbnail_path,
    width, height, duration_secs, fps, file_size, checksum, status,
    error_message, created_at, updated_at
) VALUES
    (92001, 1, 1, NULL, NULL, NULL, 'video', 'generated', 'legacy.mp4', NULL, 1280, 720, 4, 24, 10, NULL, 'completed', NULL, NOW(), NOW()),
    (92002, 2, 2, NULL, NULL, NULL, 'image', 'generated', 'canvas.png', NULL, 1024, 1024, NULL, NULL, 11, NULL, 'completed', NULL, NOW(), NOW()),
    (92003, 3, 3, NULL, NULL, NULL, 'image', 'generated', 'galgame.png', NULL, 1024, 1024, NULL, NULL, 12, NULL, 'completed', NULL, NOW(), NOW()),
    (92004, 4, 4, NULL, NULL, NULL, 'image', 'generated', 'consumer.png', NULL, 1024, 1024, NULL, NULL, 13, NULL, 'completed', NULL, NOW(), NOW());

INSERT INTO media_provenance (
    media_asset_id, generation_spec_id, prompt_artifact_id, provider_job_id,
    source_media_asset_id, source_external_path, imported_from_user,
    request_snapshot_json, created_at
) VALUES
    (92001, 91001, 91101, NULL, NULL, NULL, FALSE, '{"canonical_key":"legacy:generated_video:11:media"}', NOW()),
    (92002, 91002, 91102, NULL, NULL, NULL, FALSE, '{"canonical_key":"canvas:2:image:8"}', NOW()),
    (92003, 91003, 91103, NULL, NULL, NULL, FALSE, '{"canonical_key":"galgame:3:background:scene:9"}', NOW()),
    (92004, NULL, NULL, NULL, 92001, NULL, FALSE, '{"canonical_key":"consumer:4:image"}', NOW());

INSERT INTO generation_spec_media_ref (generation_spec_id, media_asset_id, role)
VALUES (91001, 92002, 'reference');

INSERT INTO media_binding (id, media_asset_id, target_kind, target_id, role, created_at)
VALUES (92101, 92001, 'chapter_video_prompt', 7, 'generated_result', NOW());

ALTER TABLE generation_spec ENABLE TRIGGER USER;
ALTER TABLE generation_spec_media_ref ENABLE TRIGGER USER;
ALTER TABLE media_asset ENABLE TRIGGER USER;
ALTER TABLE media_provenance ENABLE TRIGGER USER;
ALTER TABLE media_binding ENABLE TRIGGER USER;
"#,
        )
        .await
        .unwrap();

        let error = Migration
            .up(&SchemaManager::new(&db))
            .await
            .expect_err("真实媒体引用 legacy 媒体时必须拒绝清理");
        assert!(
            error
                .to_string()
                .contains("real media provenance references legacy lineage")
        );

        db.execute_unprepared(
            "DELETE FROM media_provenance WHERE media_asset_id = 92004; \
             DELETE FROM media_asset WHERE id = 92004;",
        )
        .await
        .unwrap();

        Migration.up(&SchemaManager::new(&db)).await.unwrap();
        Migration.up(&SchemaManager::new(&db)).await.unwrap();

        let row = db
            .query_one(Statement::from_string(
                DbBackend::Postgres,
                r#"
SELECT
    (SELECT COUNT(*) FROM generation_spec WHERE id = 91001)
  + (SELECT COUNT(*) FROM prompt_artifact WHERE id = 91101)
  + (SELECT COUNT(*) FROM media_asset WHERE id = 92001)
  + (SELECT COUNT(*) FROM media_provenance WHERE media_asset_id = 92001)
  + (SELECT COUNT(*) FROM media_binding WHERE media_asset_id = 92001)
  + (SELECT COUNT(*) FROM generation_spec_media_ref WHERE generation_spec_id = 91001)
    AS legacy_rows,
    (SELECT COUNT(*) FROM generation_spec WHERE id IN (91002, 91003))
  + (SELECT COUNT(*) FROM prompt_artifact WHERE id IN (91102, 91103))
  + (SELECT COUNT(*) FROM media_asset WHERE id IN (92002, 92003))
  + (SELECT COUNT(*) FROM media_provenance WHERE media_asset_id IN (92002, 92003))
    AS real_rows
"#,
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<i64>("", "legacy_rows").unwrap(), 0);
        assert_eq!(row.try_get::<i64>("", "real_rows").unwrap(), 8);
        assert!(UP_SQL.contains("legacy_asset_lineage_sync_v1"));
    }
}
