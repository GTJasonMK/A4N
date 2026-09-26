use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(LOCAL_REFERENCES_SQL)
            .await?;
        manager
            .get_connection()
            .execute_unprepared(RETIREMENT_SQL)
            .await?;
        for table in [
            "generation_spec",
            "galgame_scene",
            "galgame_line",
            "prompt_artifact",
            "media_asset",
            "video_segment",
        ] {
            manager.get_connection().execute_unprepared(&format!(
                "CREATE TRIGGER trg_source_sync_{table}_deleted AFTER DELETE ON {table} REFERENCING OLD TABLE AS deleted_local_targets FOR EACH STATEMENT EXECUTE FUNCTION a4n_retire_source_sync_deleted_targets()"
            )).await?;
        }
        manager
            .get_connection()
            .execute_unprepared(RETIRE_EXISTING_SQL)
            .await?;
        let previous = super::galgame_scene_source_identity::BACKFILL_FUNCTION_SQL;
        if previous.matches("conflict.status = 'resolved'").count() != 1 {
            return Err(DbErr::Custom(
                "source_sync_local_target_retirement: unexpected historical backfill contract"
                    .into(),
            ));
        }
        let backfill = previous
            .replacen("CREATE FUNCTION", "CREATE OR REPLACE FUNCTION", 1)
            .replace(
                "conflict.status = 'resolved'",
                "conflict.status IN ('resolved','ignored','obsolete')",
            );
        manager
            .get_connection()
            .execute_unprepared(&backfill)
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "失效的来源比较已转为历史记录，不能恢复为待处理目标".into(),
        ))
    }
}

// Obsolete records contain historical IDs, so never interpret their JSON as live references.
// The view also gives deletion and one-time repair the same reference contract.
const LOCAL_REFERENCES_SQL: &str = r#"
CREATE VIEW a4n_source_sync_local_references AS
WITH current_comparisons AS MATERIALIZED (
    SELECT id, conflict_scope, target_kind, target_id, resolution_strategy,
           local_summary_json::jsonb AS local, affected_refs_json::jsonb AS affected,
           decision_json::jsonb AS decision
    FROM source_sync_conflict WHERE status <> 'obsolete'
)
SELECT c.id AS conflict_id, ref.target_table, ref.target_id
FROM current_comparisons c
CROSS JOIN LATERAL (VALUES
    (CASE
        WHEN c.conflict_scope IN ('generation_spec','text_anchor') AND c.target_kind = 'segment' THEN 'generation_spec'
        WHEN c.conflict_scope = 'text_anchor' AND c.target_kind IN ('galgame_scene','galgame_line') THEN c.target_kind
    END, c.target_id),
    ('generation_spec', (c.local ->> 'generation_spec_id')::bigint),
    ('generation_spec', (c.affected ->> 'generation_spec_id')::bigint),
    ('galgame_scene', (c.local ->> 'galgame_scene_id')::bigint),
    ('galgame_scene', (c.affected ->> 'galgame_scene_id')::bigint),
    ('galgame_line', (c.local ->> 'galgame_line_id')::bigint),
    ('galgame_line', (c.affected ->> 'galgame_line_id')::bigint),
    ('generation_spec', CASE WHEN c.resolution_strategy IN ('rebind_anchor','adopt_source','rebuild_prompt','create_replacement') THEN (c.decision ->> 'generation_spec_id')::bigint END),
    ('prompt_artifact', CASE WHEN c.resolution_strategy = 'manual' THEN (c.decision ->> 'prompt_artifact_id')::bigint END),
    ('galgame_scene', CASE WHEN c.resolution_strategy = 'regenerate_scene' THEN (c.decision ->> 'galgame_scene_id')::bigint END),
    ('video_segment', CASE WHEN c.resolution_strategy = 'create_replacement' THEN (c.decision #>> '{replacement_segment,previous_segment_id}')::bigint END),
    ('video_segment', CASE WHEN c.resolution_strategy = 'create_replacement' THEN (c.decision #>> '{replacement_segment,next_segment_id}')::bigint END)
) ref(target_table, target_id)
WHERE ref.target_table IS NOT NULL AND ref.target_id IS NOT NULL
UNION ALL
SELECT c.id, 'media_asset', (ref.value ->> 'media_asset_id')::bigint
FROM current_comparisons c
CROSS JOIN LATERAL jsonb_array_elements(CASE
    WHEN c.resolution_strategy IN ('adopt_source','rebuild_prompt','create_replacement') THEN c.decision -> 'media_refs'
    ELSE '[]'::jsonb END) ref(value)
WHERE ref.value ->> 'media_asset_id' IS NOT NULL;
"#;

const RETIREMENT_SQL: &str = r#"
CREATE FUNCTION a4n_retire_source_sync_local_targets(target_table TEXT, removed_ids BIGINT[])
RETURNS VOID LANGUAGE SQL STRICT AS $$
    UPDATE source_sync_conflict c
    SET status = 'obsolete',
        resolution_strategy = CASE WHEN c.status = 'pending' THEN 'delete_local' ELSE c.resolution_strategy END,
        decision_json = CASE WHEN c.status = 'pending' THEN json_build_object('system_action','local_target_removed') ELSE c.decision_json END,
        resolved_at = CASE WHEN c.status = 'pending' THEN CURRENT_TIMESTAMP AT TIME ZONE 'UTC' ELSE c.resolved_at END
    WHERE c.status <> 'obsolete' AND c.id IN (
        SELECT ref.conflict_id FROM a4n_source_sync_local_references ref
        WHERE ref.target_table = $1 AND ref.target_id = ANY($2)
    )
$$;

CREATE FUNCTION a4n_retire_source_sync_deleted_targets()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    PERFORM a4n_retire_source_sync_local_targets(TG_TABLE_NAME, (SELECT array_agg(id) FROM deleted_local_targets));
    RETURN NULL;
END;
$$;
"#;

const RETIRE_EXISTING_SQL: &str = r#"
DO $$
DECLARE removed RECORD;
BEGIN
    FOR removed IN
        SELECT ref.target_table, array_agg(DISTINCT ref.target_id) AS ids
        FROM a4n_source_sync_local_references ref
        WHERE CASE ref.target_table
            WHEN 'generation_spec' THEN NOT EXISTS (SELECT 1 FROM generation_spec target WHERE target.id = ref.target_id)
            WHEN 'galgame_scene' THEN NOT EXISTS (SELECT 1 FROM galgame_scene target WHERE target.id = ref.target_id)
            WHEN 'galgame_line' THEN NOT EXISTS (SELECT 1 FROM galgame_line target WHERE target.id = ref.target_id)
            WHEN 'prompt_artifact' THEN NOT EXISTS (SELECT 1 FROM prompt_artifact target WHERE target.id = ref.target_id)
            WHEN 'media_asset' THEN NOT EXISTS (SELECT 1 FROM media_asset target WHERE target.id = ref.target_id)
            WHEN 'video_segment' THEN NOT EXISTS (SELECT 1 FROM video_segment target WHERE target.id = ref.target_id)
        END
        GROUP BY ref.target_table
    LOOP
        PERFORM a4n_retire_source_sync_local_targets(removed.target_table, removed.ids);
    END LOOP;
END;
$$;
"#;

#[cfg(test)]
mod tests;
