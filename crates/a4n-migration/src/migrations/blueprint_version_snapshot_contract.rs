//! Recanonicalize blueprint snapshots written after the original split-column cleanup.

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "blueprint_version_snapshot_contract";

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
            "blueprint_version_snapshot_contract 不可逆：退役快照字段不会恢复".to_string(),
        ))
    }
}

const UP_SQL: &str = r#"
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM blueprint_version
        WHERE jsonb_typeof(blueprint_data::jsonb -> 'characters')
                IS DISTINCT FROM 'array'
           OR jsonb_typeof(blueprint_data::jsonb -> 'relationships')
                IS DISTINCT FROM 'array'
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_contract: canonical snapshot arrays are missing';
    END IF;

    UPDATE blueprint_version AS version
    SET blueprint_data = jsonb_set(
        jsonb_set(
            jsonb_build_object(
                'style', NULL,
                'tone', NULL,
                'target_audience', NULL,
                'one_sentence_summary', NULL,
                'full_synopsis', NULL,
                'world_setting', NULL,
                'chapters_per_part', NULL,
                'avatar_svg', NULL,
                'avatar_animal', NULL
            ) || (
                version.blueprint_data::jsonb
                    - 'current_version'
                    - 'optimization_patch'
                    - 'chapter_outline'
                    - 'origin'
                    - 'import_analysis_proposal'
            ),
            '{characters}',
            (
                SELECT COALESCE(
                    jsonb_agg(
                        jsonb_build_object(
                            'gender', NULL,
                            'age', NULL,
                            'appearance', NULL,
                            'personality', NULL,
                            'background', NULL,
                            'motivation', NULL,
                            'arc_summary', NULL,
                            'relationship_to_protagonist', NULL,
                            'arc_stages', NULL
                        ) || (
                            character.value
                                - 'id'
                                - 'project_id'
                                - 'created_at'
                                - 'updated_at'
                        )
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
                    jsonb_build_object(
                        'description', NULL,
                        'strength', NULL,
                        'evolution', NULL
                    ) || (
                        relationship.value
                            - 'id'
                            - 'project_id'
                            - 'created_at'
                    )
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
            'blueprint_version_snapshot_contract: unknown top-level snapshot fields remain';
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
            'blueprint_version_snapshot_contract: character snapshots must be objects';
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
            'blueprint_version_snapshot_contract: unknown character snapshot fields remain';
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
            'blueprint_version_snapshot_contract: relationship snapshots must be objects';
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
            'blueprint_version_snapshot_contract: unknown relationship snapshot fields remain';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM novel_blueprint AS blueprint
        WHERE NOT EXISTS (
            SELECT 1
            FROM blueprint_version AS version
            WHERE version.project_id = blueprint.project_id
              AND version.version = blueprint.current_version
        )
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_contract: current blueprint version is missing';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM novel_project AS project
        WHERE project.raw_blueprint IS NOT NULL
          AND NOT EXISTS (
              SELECT 1
              FROM novel_blueprint AS blueprint
              WHERE blueprint.project_id = project.id
          )
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_contract: raw blueprint has no formal blueprint';
    END IF;

    UPDATE blueprint_version AS version
    SET blueprint_data = jsonb_build_object(
        'title', blueprint.title,
        'genre', blueprint.genre,
        'style', blueprint.style,
        'tone', blueprint.tone,
        'target_audience', blueprint.target_audience,
        'one_sentence_summary', blueprint.one_sentence_summary,
        'full_synopsis', blueprint.full_synopsis,
        'world_setting', blueprint.world_setting,
        'needs_part_outlines', blueprint.needs_part_outlines,
        'total_chapters', blueprint.total_chapters,
        'chapters_per_part', blueprint.chapters_per_part,
        'avatar_svg', blueprint.avatar_svg,
        'avatar_animal', blueprint.avatar_animal,
        'characters', COALESCE((
            SELECT jsonb_agg(
                jsonb_build_object(
                    'name', character.name,
                    'character_type', character.character_type,
                    'gender', character.gender,
                    'age', character.age,
                    'appearance', character.appearance,
                    'personality', character.personality,
                    'background', character.background,
                    'motivation', character.motivation,
                    'arc_summary', character.arc_summary,
                    'relationship_to_protagonist',
                        character.relationship_to_protagonist,
                    'arc_stages', character.arc_stages
                )
                ORDER BY character.id
            )
            FROM blueprint_character AS character
            WHERE character.project_id = blueprint.project_id
        ), '[]'::jsonb),
        'relationships', COALESCE((
            SELECT jsonb_agg(
                jsonb_build_object(
                    'relationship_type', relationship.relationship_type,
                    'character_a', relationship.character_a,
                    'character_b', relationship.character_b,
                    'description', relationship.description,
                    'strength', relationship.strength,
                    'evolution', relationship.evolution
                )
                ORDER BY relationship.id
            )
            FROM blueprint_relationship AS relationship
            WHERE relationship.project_id = blueprint.project_id
        ), '[]'::jsonb)
    )::json
    FROM novel_blueprint AS blueprint
    WHERE version.project_id = blueprint.project_id
      AND version.version = blueprint.current_version;

    IF EXISTS (
        SELECT 1
        FROM blueprint_version
        WHERE jsonb_typeof(blueprint_data::jsonb -> 'needs_part_outlines')
                IS DISTINCT FROM 'boolean'
           OR NOT CASE
                WHEN jsonb_typeof(blueprint_data::jsonb -> 'total_chapters') = 'number'
                 AND (blueprint_data::jsonb ->> 'total_chapters') ~ '^[0-9]+$'
                THEN (blueprint_data::jsonb ->> 'total_chapters')::numeric
                    BETWEEN 1 AND 10000
                ELSE FALSE
           END
           OR (
                blueprint_data::jsonb -> 'chapters_per_part' <> 'null'::jsonb
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
            'blueprint_version_snapshot_contract: invalid canonical chapter fields remain';
    END IF;

    UPDATE novel_blueprint
    SET needs_part_outlines = total_chapters > 50,
        chapters_per_part = CASE
            WHEN total_chapters > 50 THEN 25
            ELSE NULL
        END;

    UPDATE blueprint_version
    SET blueprint_data = jsonb_set(
        jsonb_set(
            blueprint_data::jsonb,
            '{needs_part_outlines}',
            to_jsonb((blueprint_data::jsonb ->> 'total_chapters')::integer > 50),
            false
        ),
        '{chapters_per_part}',
        CASE
            WHEN (blueprint_data::jsonb ->> 'total_chapters')::integer > 50
                THEN '25'::jsonb
            ELSE 'null'::jsonb
        END,
        false
    )::json;

    IF EXISTS (
        SELECT 1
        FROM blueprint_version
        WHERE (blueprint_data::jsonb ->> 'needs_part_outlines')::boolean
                IS DISTINCT FROM (
                    (blueprint_data::jsonb ->> 'total_chapters')::integer > 50
                )
           OR CASE
                WHEN (blueprint_data::jsonb ->> 'needs_part_outlines')::boolean
                THEN blueprint_data::jsonb -> 'chapters_per_part' = 'null'::jsonb
                    OR (blueprint_data::jsonb ->> 'chapters_per_part')::integer <> 25
                ELSE blueprint_data::jsonb -> 'chapters_per_part' <> 'null'::jsonb
           END
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_contract: derived chapter fields are inconsistent';
    END IF;

    UPDATE novel_project AS project
    SET raw_blueprint = jsonb_pretty(version.blueprint_data::jsonb)
    FROM novel_blueprint AS blueprint
    JOIN blueprint_version AS version
      ON version.project_id = blueprint.project_id
     AND version.version = blueprint.current_version
    WHERE project.id = blueprint.project_id;

    IF EXISTS (
        SELECT 1
        FROM novel_project AS project
        JOIN novel_blueprint AS blueprint
          ON blueprint.project_id = project.id
        JOIN blueprint_version AS version
          ON version.project_id = blueprint.project_id
         AND version.version = blueprint.current_version
        WHERE project.raw_blueprint IS NULL
           OR project.raw_blueprint::jsonb IS DISTINCT FROM version.blueprint_data::jsonb
    ) THEN
        RAISE EXCEPTION
            'blueprint_version_snapshot_contract: project raw_blueprint differs from current version';
    END IF;
END $$;
"#;

#[cfg(test)]
mod tests {
    use super::UP_SQL;
    use sea_orm::{ConnectionTrait, Statement};

    #[test]
    fn migration_recanonicalizes_post_cleanup_snapshot_writes() {
        assert!(UP_SQL.contains("canonical snapshot arrays are missing"));
        assert!(UP_SQL.contains("- 'optimization_patch'"));
        assert!(UP_SQL.contains("- 'chapter_outline'"));
        assert!(UP_SQL.contains("- 'project_id'"));
        assert!(UP_SQL.contains("unknown top-level snapshot fields remain"));
        assert!(UP_SQL.contains("unknown character snapshot fields remain"));
        assert!(UP_SQL.contains("unknown relationship snapshot fields remain"));
        assert!(UP_SQL.contains("current blueprint version is missing"));
        assert!(UP_SQL.contains("raw blueprint has no formal blueprint"));
        assert!(UP_SQL.contains("FROM blueprint_character AS character"));
        assert!(UP_SQL.contains("FROM blueprint_relationship AS relationship"));
        assert!(UP_SQL.contains("invalid canonical chapter fields remain"));
        assert!(UP_SQL.contains("UPDATE novel_blueprint"));
        assert!(UP_SQL.contains("UPDATE blueprint_version"));
        assert!(UP_SQL.contains("derived chapter fields are inconsistent"));
        assert!(UP_SQL.contains("SET raw_blueprint = jsonb_pretty"));
    }

    #[tokio::test]
    async fn migration_rebuilds_the_current_snapshot_from_formal_rows() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            r#"
INSERT INTO "user" (
    id, username, email, password_hash, role, is_active, lifecycle_gate,
    created_at, updated_at
) VALUES (
    91001, 'blueprint-contract', 'blueprint-contract@example.com',
    'test', 'user', TRUE, 'active', NOW(), NOW()
);

INSERT INTO novel_project (
    id, user_id, title, description, project_type, project_kind, source_mode,
    root_project_id, source_snapshot_id, source_sync_status, status, lifecycle_gate, target_chapters,
    current_chapter, total_chapters, chapters_per_part, avatar_svg,
    avatar_animal, raw_blueprint, import_analysis_progress, export_config,
    created_at, updated_at
) VALUES (
    92001, 91001, '雾城', '', 'novel_long', 'text', 'self',
    NULL, NULL, 'current', 'blueprint_ready', 'active', 1,
    0, 1, NULL, NULL, NULL, '{"legacy":true}', NULL, NULL, NOW(), NOW()
);

INSERT INTO novel_blueprint (
    project_id, title, genre, style, tone, target_audience,
    one_sentence_summary, full_synopsis, world_setting, current_version,
    needs_part_outlines, total_chapters, chapters_per_part, avatar_svg,
    avatar_animal, created_at, updated_at
) VALUES (
    92001, '雾城', '悬疑', '冷峻', NULL, NULL,
    '追查倒流迷雾', '正式梗概', '{"premise":"迷雾倒流"}', '2.0',
    FALSE, 1, NULL, NULL, 'fox', NOW(), NOW()
);

INSERT INTO blueprint_character (
    project_id, name, character_type, gender, age, appearance, personality,
    background, motivation, arc_summary, relationship_to_protagonist,
    arc_stages, created_at, updated_at
) VALUES (
    92001, '林澈', 'protagonist', NULL, NULL, NULL, '正式资料',
    NULL, NULL, NULL, NULL, NULL, NOW(), NOW()
);

INSERT INTO blueprint_relationship (
    project_id, relationship_type, character_a, character_b, description,
    strength, evolution, created_at
) VALUES (
    92001, 'rival', '林澈', '许灯', '互相试探', 7, NULL, NOW()
);

INSERT INTO blueprint_version (
    project_id, version, change_description, blueprint_data, created_at,
    created_by
) VALUES (
    92001, '1.0', '历史短篇版本', '{
        "title": "雾城初稿",
        "genre": "悬疑",
        "needs_part_outlines": false,
        "total_chapters": 1,
        "chapters_per_part": 25,
        "characters": [],
        "relationships": []
    }', NOW(), 'system'
), (
    92001, '2.0', '旧生成路径', '{
        "title": "旧快照",
        "genre": "悬疑",
        "needs_part_outlines": false,
        "total_chapters": 1,
        "current_version": "2.0",
        "chapter_outline": [],
        "optimization_patch": {"style": "legacy"},
        "characters": [{
            "id": 31,
            "project_id": 92001,
            "name": "林澈",
            "character_type": "protagonist",
            "personality": "旧快照",
            "created_at": "2026-01-01T00:00:00"
        }],
        "relationships": [{
            "id": 41,
            "project_id": 92001,
            "relationship_type": "rival",
            "character_a": "林澈",
            "character_b": "许灯",
            "created_at": "2026-01-01T00:00:00"
        }]
    }', NOW(), 'system'
);
"#,
        )
        .await
        .expect("legacy blueprint fixture must be created");

        db.execute_unprepared(UP_SQL)
            .await
            .expect("blueprint snapshot contract migration must succeed");

        let row = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT version.blueprint_data, project.raw_blueprint \
                 FROM blueprint_version AS version \
                 JOIN novel_project AS project ON project.id = version.project_id \
                 WHERE version.project_id = 92001 AND version.version = '2.0'"
                    .to_string(),
            ))
            .await
            .expect("canonical blueprint snapshot query must succeed")
            .expect("canonical blueprint snapshot must exist");
        let snapshot: serde_json::Value = row.try_get("", "blueprint_data").unwrap();
        let raw_blueprint: String = row.try_get("", "raw_blueprint").unwrap();
        let raw_blueprint: serde_json::Value = serde_json::from_str(&raw_blueprint).unwrap();

        assert_eq!(snapshot["title"], "雾城");
        assert_eq!(snapshot["total_chapters"], 1);
        assert_eq!(snapshot["avatar_animal"], "fox");
        assert_eq!(snapshot["characters"][0]["personality"], "正式资料");
        assert_eq!(snapshot["relationships"][0]["strength"], 7);
        assert!(snapshot.get("current_version").is_none());
        assert!(snapshot.get("chapter_outline").is_none());
        assert!(snapshot.get("optimization_patch").is_none());
        assert!(snapshot["characters"][0].get("id").is_none());
        assert!(snapshot["relationships"][0].get("project_id").is_none());
        assert_eq!(raw_blueprint, snapshot);

        let historical = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT blueprint_data FROM blueprint_version \
                 WHERE project_id = 92001 AND version = '1.0'"
                    .to_string(),
            ))
            .await
            .unwrap()
            .unwrap();
        let historical: serde_json::Value = historical.try_get("", "blueprint_data").unwrap();
        assert_eq!(historical["needs_part_outlines"], false);
        assert_eq!(historical["chapters_per_part"], serde_json::Value::Null);
    }
}
