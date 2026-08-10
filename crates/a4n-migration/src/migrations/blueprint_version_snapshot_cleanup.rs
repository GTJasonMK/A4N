//! Remove split blueprint-version snapshots superseded by `blueprint_data`.

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "blueprint_version_snapshot_cleanup";

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
        Err(DbErr::Custom(
            "blueprint_version_snapshot_cleanup 不可逆：回滚会恢复重复快照来源".to_string(),
        ))
    }
}

const UP_SQL: &str = r#"
DO $$
DECLARE
    redundant_column_count INTEGER;
BEGIN
    SELECT COUNT(*)
    INTO redundant_column_count
    FROM information_schema.columns
    WHERE table_schema = current_schema()
      AND table_name = 'blueprint_version'
      AND column_name IN (
          'world_setting_snapshot',
          'characters_snapshot',
          'relationships_snapshot',
          'synopsis_snapshot'
      );

    IF redundant_column_count = 0 THEN
        RETURN;
    END IF;
    IF redundant_column_count <> 4 THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_cleanup: partial redundant snapshot schema';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM blueprint_version
        WHERE jsonb_typeof(blueprint_data::jsonb -> 'characters') IS DISTINCT FROM 'array'
           OR jsonb_typeof(blueprint_data::jsonb -> 'relationships') IS DISTINCT FROM 'array'
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_cleanup: canonical snapshot arrays are missing';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM blueprint_version
        WHERE jsonb_typeof(blueprint_data::jsonb -> 'needs_part_outlines')
                IS DISTINCT FROM 'boolean'
           OR NOT CASE
                WHEN jsonb_typeof(blueprint_data::jsonb -> 'total_chapters') = 'number'
                 AND (blueprint_data::jsonb ->> 'total_chapters') ~ '^[0-9]+$'
                THEN (blueprint_data::jsonb ->> 'total_chapters')::numeric
                    BETWEEN 5 AND 10000
                ELSE FALSE
           END
           OR (
                blueprint_data::jsonb ? 'chapters_per_part'
                AND blueprint_data::jsonb -> 'chapters_per_part' <> 'null'::jsonb
                AND NOT CASE
                    WHEN jsonb_typeof(blueprint_data::jsonb -> 'chapters_per_part') = 'number'
                     AND (blueprint_data::jsonb ->> 'chapters_per_part') ~ '^[0-9]+$'
                    THEN (blueprint_data::jsonb ->> 'chapters_per_part')::numeric
                        BETWEEN 1 AND 2147483647
                    ELSE FALSE
                END
           )
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_cleanup: invalid canonical chapter fields remain';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM blueprint_version
        WHERE (blueprint_data::jsonb ->> 'needs_part_outlines')::boolean
                IS DISTINCT FROM (
                    (blueprint_data::jsonb ->> 'total_chapters')::integer > 50
                )
           OR CASE
                WHEN (blueprint_data::jsonb ->> 'needs_part_outlines')::boolean
                THEN NOT (blueprint_data::jsonb ? 'chapters_per_part')
                    OR blueprint_data::jsonb -> 'chapters_per_part' = 'null'::jsonb
                    OR (blueprint_data::jsonb ->> 'chapters_per_part')::integer <> 25
                ELSE blueprint_data::jsonb ? 'chapters_per_part'
                    AND blueprint_data::jsonb -> 'chapters_per_part' <> 'null'::jsonb
           END
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_cleanup: derived chapter fields are inconsistent';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM blueprint_version
        WHERE (
            NULLIF(world_setting_snapshot::jsonb, 'null'::jsonb) IS NOT NULL
            AND world_setting_snapshot::jsonb IS DISTINCT FROM
                blueprint_data::jsonb -> 'world_setting'
        ) OR (
            NULLIF(characters_snapshot::jsonb, 'null'::jsonb) IS NOT NULL
            AND characters_snapshot::jsonb IS DISTINCT FROM
                blueprint_data::jsonb -> 'characters'
        ) OR (
            NULLIF(relationships_snapshot::jsonb, 'null'::jsonb) IS NOT NULL
            AND relationships_snapshot::jsonb IS DISTINCT FROM
                blueprint_data::jsonb -> 'relationships'
        ) OR (
            synopsis_snapshot IS NOT NULL
            AND synopsis_snapshot IS DISTINCT FROM
                blueprint_data::jsonb ->> 'full_synopsis'
        )
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_cleanup: split snapshot differs from blueprint_data';
    END IF;

    UPDATE blueprint_version AS version
    SET blueprint_data = jsonb_set(
        jsonb_set(
            version.blueprint_data::jsonb
                - 'current_version'
                - 'optimization_patch'
                - 'chapter_outline'
                - 'origin'
                - 'import_analysis_proposal',
            '{characters}',
            (
                SELECT COALESCE(
                    jsonb_agg(
                        character.value
                            - 'id'
                            - 'project_id'
                            - 'created_at'
                            - 'updated_at'
                        ORDER BY character.ordinality
                    ),
                    '[]'::jsonb
                )
                FROM jsonb_array_elements(
                    version.blueprint_data::jsonb -> 'characters'
                ) WITH ORDINALITY AS character(value, ordinality)
            ),
            false
        ),
        '{relationships}',
        (
            SELECT COALESCE(
                jsonb_agg(
                    relationship.value
                        - 'id'
                        - 'project_id'
                        - 'created_at'
                    ORDER BY relationship.ordinality
                ),
                '[]'::jsonb
            )
            FROM jsonb_array_elements(
                version.blueprint_data::jsonb -> 'relationships'
            ) WITH ORDINALITY AS relationship(value, ordinality)
        ),
        false
    )::json;

    IF EXISTS (
        SELECT 1
        FROM blueprint_version AS version
        CROSS JOIN LATERAL jsonb_object_keys(version.blueprint_data::jsonb) AS field(name)
        WHERE NOT (field.name = ANY (ARRAY[
            'title',
            'genre',
            'style',
            'tone',
            'target_audience',
            'one_sentence_summary',
            'full_synopsis',
            'world_setting',
            'needs_part_outlines',
            'total_chapters',
            'chapters_per_part',
            'avatar_svg',
            'avatar_animal',
            'characters',
            'relationships'
        ]::text[]))
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_cleanup: unknown top-level snapshot fields remain';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM blueprint_version AS version
        CROSS JOIN LATERAL jsonb_array_elements(
            version.blueprint_data::jsonb -> 'characters'
        ) AS character(value)
        WHERE jsonb_typeof(character.value) IS DISTINCT FROM 'object'
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_cleanup: character snapshots must be objects';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM blueprint_version AS version
        CROSS JOIN LATERAL jsonb_array_elements(
            version.blueprint_data::jsonb -> 'characters'
        ) AS character(value)
        CROSS JOIN LATERAL jsonb_object_keys(character.value) AS field(name)
        WHERE NOT (field.name = ANY (ARRAY[
            'name',
            'character_type',
            'gender',
            'age',
            'appearance',
            'personality',
            'background',
            'motivation',
            'arc_summary',
            'relationship_to_protagonist',
            'arc_stages'
        ]::text[]))
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_cleanup: unknown character snapshot fields remain';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM blueprint_version AS version
        CROSS JOIN LATERAL jsonb_array_elements(
            version.blueprint_data::jsonb -> 'relationships'
        ) AS relationship(value)
        WHERE jsonb_typeof(relationship.value) IS DISTINCT FROM 'object'
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_cleanup: relationship snapshots must be objects';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM blueprint_version AS version
        CROSS JOIN LATERAL jsonb_array_elements(
            version.blueprint_data::jsonb -> 'relationships'
        ) AS relationship(value)
        CROSS JOIN LATERAL jsonb_object_keys(relationship.value) AS field(name)
        WHERE NOT (field.name = ANY (ARRAY[
            'relationship_type',
            'character_a',
            'character_b',
            'description',
            'strength',
            'evolution'
        ]::text[]))
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_cleanup: unknown relationship snapshot fields remain';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM blueprint_version AS version
        CROSS JOIN LATERAL (VALUES
            ('title', TRUE),
            ('genre', TRUE),
            ('style', FALSE),
            ('tone', FALSE),
            ('target_audience', FALSE),
            ('one_sentence_summary', FALSE),
            ('full_synopsis', FALSE),
            ('avatar_svg', FALSE),
            ('avatar_animal', FALSE)
        ) AS field(name, required)
        WHERE (
            field.required
            AND (
                NOT (version.blueprint_data::jsonb ? field.name)
                OR version.blueprint_data::jsonb -> field.name = 'null'::jsonb
            )
        ) OR (
            version.blueprint_data::jsonb ? field.name
            AND version.blueprint_data::jsonb -> field.name <> 'null'::jsonb
            AND (
                jsonb_typeof(version.blueprint_data::jsonb -> field.name)
                    IS DISTINCT FROM 'string'
                OR btrim(version.blueprint_data::jsonb ->> field.name) = ''
            )
        )
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_cleanup: invalid top-level snapshot strings remain';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM blueprint_version AS version
        CROSS JOIN LATERAL jsonb_array_elements(
            version.blueprint_data::jsonb -> 'characters'
        ) AS character(value)
        CROSS JOIN LATERAL (VALUES
            ('name', TRUE),
            ('character_type', TRUE),
            ('gender', FALSE),
            ('age', FALSE),
            ('appearance', FALSE),
            ('personality', FALSE),
            ('background', FALSE),
            ('motivation', FALSE),
            ('arc_summary', FALSE),
            ('relationship_to_protagonist', FALSE)
        ) AS field(name, required)
        WHERE (
            field.required
            AND (
                NOT (character.value ? field.name)
                OR character.value -> field.name = 'null'::jsonb
            )
        ) OR (
            character.value ? field.name
            AND character.value -> field.name <> 'null'::jsonb
            AND (
                jsonb_typeof(character.value -> field.name) IS DISTINCT FROM 'string'
                OR btrim(character.value ->> field.name) = ''
            )
        )
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_cleanup: invalid character snapshot strings remain';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM blueprint_version AS version
        CROSS JOIN LATERAL jsonb_array_elements(
            version.blueprint_data::jsonb -> 'relationships'
        ) AS relationship(value)
        CROSS JOIN LATERAL (VALUES
            ('relationship_type', TRUE),
            ('character_a', TRUE),
            ('character_b', TRUE),
            ('description', FALSE)
        ) AS field(name, required)
        WHERE (
            field.required
            AND (
                NOT (relationship.value ? field.name)
                OR relationship.value -> field.name = 'null'::jsonb
            )
        ) OR (
            relationship.value ? field.name
            AND relationship.value -> field.name <> 'null'::jsonb
            AND (
                jsonb_typeof(relationship.value -> field.name) IS DISTINCT FROM 'string'
                OR btrim(relationship.value ->> field.name) = ''
            )
        )
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_cleanup: invalid relationship snapshot strings remain';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM blueprint_version AS version
        CROSS JOIN LATERAL jsonb_array_elements(
            version.blueprint_data::jsonb -> 'relationships'
        ) AS relationship(value)
        WHERE relationship.value ? 'strength'
          AND relationship.value -> 'strength' <> 'null'::jsonb
          AND NOT CASE
                WHEN jsonb_typeof(relationship.value -> 'strength') = 'number'
                 AND (relationship.value ->> 'strength') ~ '^[0-9]+$'
                THEN (relationship.value ->> 'strength')::numeric BETWEEN 1 AND 10
                ELSE FALSE
          END
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_cleanup: invalid relationship strengths remain';
    END IF;

    ALTER TABLE blueprint_version
        DROP COLUMN world_setting_snapshot,
        DROP COLUMN characters_snapshot,
        DROP COLUMN relationships_snapshot,
        DROP COLUMN synopsis_snapshot;
END $$;
"#;

#[cfg(test)]
mod tests {
    use super::UP_SQL;

    #[test]
    fn migration_guards_then_drops_all_split_snapshot_columns() {
        assert!(UP_SQL.contains("split snapshot differs from blueprint_data"));
        assert!(UP_SQL.contains("invalid canonical chapter fields remain"));
        assert!(UP_SQL.contains("derived chapter fields are inconsistent"));
        assert!(UP_SQL.contains("- 'optimization_patch'"));
        assert!(UP_SQL.contains("- 'import_analysis_proposal'"));
        assert!(UP_SQL.contains("unknown top-level snapshot fields remain"));
        assert!(UP_SQL.contains("unknown character snapshot fields remain"));
        assert!(UP_SQL.contains("unknown relationship snapshot fields remain"));
        assert!(UP_SQL.contains("invalid top-level snapshot strings remain"));
        assert!(UP_SQL.contains("invalid character snapshot strings remain"));
        assert!(UP_SQL.contains("invalid relationship snapshot strings remain"));
        assert!(UP_SQL.contains("invalid relationship strengths remain"));
        assert_eq!(UP_SQL.matches("DROP COLUMN").count(), 4);
    }
}
