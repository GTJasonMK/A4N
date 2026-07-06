//! Galgame generation spec target/params integrity contract.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N Galgame generation spec contract migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        for sql in GALGAME_GENERATION_SPEC_CONTRACT_UP_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N Galgame generation spec contract migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        for sql in GALGAME_GENERATION_SPEC_CONTRACT_DOWN_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }
}

const GALGAME_GENERATION_SPEC_CONTRACT_UP_SQL: &[&str] = &[
    GALGAME_GENERATION_SPEC_FAMILY_FUNCTION_SQL,
    GENERATION_SPEC_TEXT_ANCHOR_CHAPTER_FAMILY_FUNCTION_SQL,
    GENERATION_SPEC_ENTITY_REF_FAMILY_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_generation_spec_family_update ON generation_spec",
    GALGAME_GENERATION_SPEC_FAMILY_UPDATE_TRIGGER_SQL,
    "DROP TRIGGER IF EXISTS trg_generation_spec_entity_ref_family_insert ON generation_spec_entity_ref",
    "DROP TRIGGER IF EXISTS trg_generation_spec_entity_ref_family_update ON generation_spec_entity_ref",
    GENERATION_SPEC_ENTITY_REF_FAMILY_INSERT_TRIGGER_SQL,
    GENERATION_SPEC_ENTITY_REF_FAMILY_UPDATE_TRIGGER_SQL,
];

const GALGAME_GENERATION_SPEC_CONTRACT_DOWN_SQL: &[&str] = &[
    "DROP TRIGGER IF EXISTS trg_generation_spec_entity_ref_family_update ON generation_spec_entity_ref",
    "DROP TRIGGER IF EXISTS trg_generation_spec_entity_ref_family_insert ON generation_spec_entity_ref",
    "DROP FUNCTION IF EXISTS a4n_validate_generation_spec_entity_ref_family()",
    GENERIC_GENERATION_SPEC_FAMILY_FUNCTION_SQL,
    GENERIC_GENERATION_SPEC_TEXT_ANCHOR_FAMILY_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_generation_spec_family_update ON generation_spec",
    GENERIC_GENERATION_SPEC_FAMILY_UPDATE_TRIGGER_SQL,
];

pub(crate) const GALGAME_GENERATION_SPEC_FAMILY_UPDATE_TRIGGER_SQL: &str = r#"
CREATE TRIGGER trg_generation_spec_family_update
BEFORE UPDATE OF project_id, root_project_id, source_context_kind, source_snapshot_id, source_chapter_id, source_snapshot_chapter_id, canvas_text_asset_id, target_kind, target_entity_kind, target_entity_id, params_json ON generation_spec
FOR EACH ROW EXECUTE FUNCTION a4n_validate_generation_spec_family()
"#;

pub(crate) const GALGAME_GENERATION_SPEC_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    context_kind TEXT := COALESCE(NULLIF(NEW.source_context_kind, ''), 'snapshot_backed');
    params JSONB := NEW.params_json::jsonb;
    project_kind TEXT;
    project_source_mode TEXT;
    project_root_project_id BIGINT;
    project_source_snapshot_id BIGINT;
    target_id BIGINT;
    target_scene_id BIGINT;
    target_scene_snapshot_chapter_id BIGINT;
    target_scene_source_chapter_id BIGINT;
    target_line_id BIGINT;
    target_line_scene_id BIGINT;
    target_script_project_id BIGINT;
    target_script_root_project_id BIGINT;
    target_script_source_snapshot_id BIGINT;
    expected_key TEXT;
    param_text TEXT;
    params_scene_id BIGINT;
    params_line_id BIGINT;
    params_source_snapshot_chapter_id BIGINT;
    params_source_chapter_id BIGINT;
    params_profile_id BIGINT;
    params_speaker_profile_id BIGINT;
    params_expression TEXT;
    params_pose TEXT;
    params_cg_importance TEXT;
    source_line_count BIGINT;
    source_line_distinct_count BIGINT;
    sample_positions TEXT[];
    source_positions TEXT[];
BEGIN
    IF jsonb_typeof(params) IS DISTINCT FROM 'object' THEN
        RAISE EXCEPTION '生成意图 params_json 必须是 JSON 对象';
    END IF;

    IF context_kind NOT IN ('snapshot_backed', 'freeform') THEN
        RAISE EXCEPTION '生成意图 source_context_kind 无效';
    END IF;

    SELECT project_row.project_kind,
           project_row.source_mode,
           project_row.root_project_id,
           project_row.source_snapshot_id
      INTO project_kind,
           project_source_mode,
           project_root_project_id,
           project_source_snapshot_id
    FROM novel_project project_row
    WHERE project_row.id = NEW.project_id;

    IF context_kind = 'snapshot_backed' THEN
        IF NOT EXISTS (
            SELECT 1 FROM novel_project root
            WHERE root.id = NEW.root_project_id
              AND root.project_kind = 'text'
        ) THEN
            RAISE EXCEPTION '生成意图 root_project_id 必须指向文本项目';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM novel_project project_row
            WHERE project_row.id = NEW.project_id
              AND (
                  (project_row.project_kind = 'text' AND project_row.id = NEW.root_project_id)
                  OR (project_row.project_kind <> 'text' AND project_row.root_project_id = NEW.root_project_id)
              )
        ) THEN
            RAISE EXCEPTION '生成意图 project_id 必须属于同一项目族';
        END IF;
        IF NEW.source_snapshot_id IS NULL THEN
            RAISE EXCEPTION 'snapshot_backed 生成意图必须包含 source_snapshot_id';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM project_source_snapshot snapshot
            WHERE snapshot.id = NEW.source_snapshot_id
              AND snapshot.root_project_id = NEW.root_project_id
        ) THEN
            RAISE EXCEPTION '生成意图 source_snapshot_id 必须属于根文本项目';
        END IF;
        IF NEW.source_snapshot_chapter_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM source_snapshot_chapter chapter
            WHERE chapter.id = NEW.source_snapshot_chapter_id
              AND chapter.snapshot_id = NEW.source_snapshot_id
              AND chapter.root_project_id = NEW.root_project_id
              AND (NEW.source_chapter_id IS NULL OR chapter.source_chapter_id = NEW.source_chapter_id)
        ) THEN
            RAISE EXCEPTION '生成意图 source_snapshot_chapter_id 必须属于目标快照';
        END IF;
    ELSE
        IF NOT EXISTS (
            SELECT 1 FROM novel_project project_row
            WHERE project_row.id = NEW.project_id
              AND project_row.project_kind = 'video'
              AND project_row.source_mode = 'freeform'
        ) THEN
            RAISE EXCEPTION 'freeform 生成意图必须属于自由视频项目';
        END IF;
        IF NEW.root_project_id <> NEW.project_id THEN
            RAISE EXCEPTION 'freeform 生成意图 root_project_id 必须等于 project_id';
        END IF;
        IF NEW.source_snapshot_id IS NOT NULL
            OR NEW.source_chapter_id IS NOT NULL
            OR NEW.source_snapshot_chapter_id IS NOT NULL
        THEN
            RAISE EXCEPTION 'freeform 生成意图不能绑定源快照或源章节';
        END IF;
        IF NEW.canvas_text_asset_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM canvas_text_asset asset
            WHERE asset.id = NEW.canvas_text_asset_id
              AND asset.project_id = NEW.project_id
        ) THEN
            RAISE EXCEPTION 'freeform 生成意图 canvas_text_asset_id 必须属于当前项目';
        END IF;
    END IF;

    IF NEW.target_kind IN ('galgame_background', 'galgame_cg', 'portrait_expression')
       OR NEW.target_entity_kind IN ('galgame_scene', 'galgame_line')
    THEN
        IF (NEW.target_kind, NEW.target_entity_kind) NOT IN (
            ('galgame_background', 'galgame_scene'),
            ('galgame_cg', 'galgame_line'),
            ('portrait_expression', 'character_profile')
        ) THEN
            RAISE EXCEPTION 'Galgame 生成意图 target_kind/target_entity_kind 配对无效';
        END IF;

        IF context_kind <> 'snapshot_backed'
           OR project_kind <> 'galgame'
           OR project_source_mode <> 'derived'
           OR project_root_project_id IS DISTINCT FROM NEW.root_project_id
           OR project_source_snapshot_id IS DISTINCT FROM NEW.source_snapshot_id
        THEN
            RAISE EXCEPTION 'Galgame 生成意图必须属于同一源快照的 Galgame 派生项目';
        END IF;

        IF NEW.target_entity_id IS NULL OR NEW.target_entity_id <= 0 THEN
            RAISE EXCEPTION 'Galgame 生成意图 target_entity_id 必填';
        END IF;
        target_id := NEW.target_entity_id;
    END IF;

    IF NEW.target_kind = 'galgame_background'
       AND NEW.target_entity_kind = 'galgame_scene'
    THEN
        SELECT scene_row.id,
               scene_row.source_snapshot_chapter_id,
               chapter_row.source_chapter_id,
               script_row.project_id,
               script_row.root_project_id,
               script_row.source_snapshot_id
          INTO target_scene_id,
               target_scene_snapshot_chapter_id,
               target_scene_source_chapter_id,
               target_script_project_id,
               target_script_root_project_id,
               target_script_source_snapshot_id
        FROM galgame_scene scene_row
        JOIN galgame_script script_row ON script_row.id = scene_row.script_id
        JOIN source_snapshot_chapter chapter_row ON chapter_row.id = scene_row.source_snapshot_chapter_id
        WHERE scene_row.id = target_id;

        IF target_scene_id IS NULL THEN
            RAISE EXCEPTION 'Galgame 背景生成意图目标场不存在';
        END IF;
        IF target_script_project_id <> NEW.project_id
           OR target_script_root_project_id <> NEW.root_project_id
           OR target_script_source_snapshot_id <> NEW.source_snapshot_id
        THEN
            RAISE EXCEPTION 'Galgame 背景生成意图目标场上下文不一致';
        END IF;
        IF NEW.source_snapshot_chapter_id IS NULL
           OR NEW.source_chapter_id IS NULL
           OR NEW.source_snapshot_chapter_id <> target_scene_snapshot_chapter_id
           OR NEW.source_chapter_id <> target_scene_source_chapter_id
        THEN
            RAISE EXCEPTION 'Galgame 背景生成意图源章节必须等于目标场章节';
        END IF;

        IF jsonb_typeof(params -> 'canonical_key') IS DISTINCT FROM 'string' THEN
            RAISE EXCEPTION 'Galgame 背景生成意图 canonical_key 缺失';
        END IF;
        expected_key := format('galgame:%s:background:scene:%s', NEW.project_id, target_id);
        IF NULLIF(BTRIM(params ->> 'canonical_key'), '') IS DISTINCT FROM expected_key THEN
            RAISE EXCEPTION 'Galgame 背景生成意图 canonical_key 与目标不一致';
        END IF;

        IF jsonb_typeof(params -> 'scene') IS DISTINCT FROM 'object' THEN
            RAISE EXCEPTION 'Galgame 背景生成意图参数 scene 必须是对象';
        END IF;
        IF jsonb_typeof(params #> '{scene,id}') IS DISTINCT FROM 'number' THEN
            RAISE EXCEPTION 'Galgame 背景生成意图 scene.id 必须是正整数';
        END IF;
        param_text := params #>> '{scene,id}';
        IF param_text !~ '^[0-9]+$' THEN
            RAISE EXCEPTION 'Galgame 背景生成意图 scene.id 必须是正整数';
        END IF;
        params_scene_id := param_text::BIGINT;
        IF params_scene_id <> target_id THEN
            RAISE EXCEPTION 'Galgame 背景生成意图 scene.id 与目标场不一致';
        END IF;

        IF jsonb_typeof(params -> 'source') IS DISTINCT FROM 'object' THEN
            RAISE EXCEPTION 'Galgame 背景生成意图参数 source 必须是对象';
        END IF;
        IF jsonb_typeof(params #> '{source,source_snapshot_chapter_id}') IS DISTINCT FROM 'number' THEN
            RAISE EXCEPTION 'Galgame 背景生成意图 source.source_snapshot_chapter_id 必须是正整数';
        END IF;
        param_text := params #>> '{source,source_snapshot_chapter_id}';
        IF param_text !~ '^[0-9]+$' THEN
            RAISE EXCEPTION 'Galgame 背景生成意图 source.source_snapshot_chapter_id 必须是正整数';
        END IF;
        params_source_snapshot_chapter_id := param_text::BIGINT;
        IF params_source_snapshot_chapter_id <> NEW.source_snapshot_chapter_id THEN
            RAISE EXCEPTION 'Galgame 背景生成意图 source_snapshot_chapter_id 与生成意图不一致';
        END IF;

        IF jsonb_typeof(params #> '{source,source_chapter_id}') IS DISTINCT FROM 'number' THEN
            RAISE EXCEPTION 'Galgame 背景生成意图 source.source_chapter_id 必须是正整数';
        END IF;
        param_text := params #>> '{source,source_chapter_id}';
        IF param_text !~ '^[0-9]+$' THEN
            RAISE EXCEPTION 'Galgame 背景生成意图 source.source_chapter_id 必须是正整数';
        END IF;
        params_source_chapter_id := param_text::BIGINT;
        IF params_source_chapter_id <> NEW.source_chapter_id
           OR params_source_chapter_id <> target_scene_source_chapter_id
        THEN
            RAISE EXCEPTION 'Galgame 背景生成意图 source_chapter_id 与目标场章节不一致';
        END IF;
    ELSIF NEW.target_kind = 'galgame_cg'
       AND NEW.target_entity_kind = 'galgame_line'
    THEN
        SELECT line_row.id,
               line_row.scene_id,
               scene_row.id,
               scene_row.source_snapshot_chapter_id,
               chapter_row.source_chapter_id,
               script_row.project_id,
               script_row.root_project_id,
               script_row.source_snapshot_id
          INTO target_line_id,
               target_line_scene_id,
               target_scene_id,
               target_scene_snapshot_chapter_id,
               target_scene_source_chapter_id,
               target_script_project_id,
               target_script_root_project_id,
               target_script_source_snapshot_id
        FROM galgame_line line_row
        JOIN galgame_scene scene_row ON scene_row.id = line_row.scene_id
        JOIN galgame_script script_row ON script_row.id = scene_row.script_id
        JOIN source_snapshot_chapter chapter_row ON chapter_row.id = scene_row.source_snapshot_chapter_id
        WHERE line_row.id = target_id;

        IF target_line_id IS NULL THEN
            RAISE EXCEPTION 'Galgame CG 生成意图目标行不存在';
        END IF;
        IF target_script_project_id <> NEW.project_id
           OR target_script_root_project_id <> NEW.root_project_id
           OR target_script_source_snapshot_id <> NEW.source_snapshot_id
        THEN
            RAISE EXCEPTION 'Galgame CG 生成意图目标行上下文不一致';
        END IF;
        IF NEW.source_snapshot_chapter_id IS NULL
           OR NEW.source_chapter_id IS NULL
           OR NEW.source_snapshot_chapter_id <> target_scene_snapshot_chapter_id
           OR NEW.source_chapter_id <> target_scene_source_chapter_id
        THEN
            RAISE EXCEPTION 'Galgame CG 生成意图源章节必须等于目标行所属场章节';
        END IF;

        IF jsonb_typeof(params -> 'canonical_key') IS DISTINCT FROM 'string' THEN
            RAISE EXCEPTION 'Galgame CG 生成意图 canonical_key 缺失';
        END IF;
        expected_key := format('galgame:%s:cg:line:%s', NEW.project_id, target_id);
        IF NULLIF(BTRIM(params ->> 'canonical_key'), '') IS DISTINCT FROM expected_key THEN
            RAISE EXCEPTION 'Galgame CG 生成意图 canonical_key 与目标不一致';
        END IF;

        IF jsonb_typeof(params -> 'line') IS DISTINCT FROM 'object' THEN
            RAISE EXCEPTION 'Galgame CG 生成意图参数 line 必须是对象';
        END IF;
        IF jsonb_typeof(params #> '{line,id}') IS DISTINCT FROM 'number' THEN
            RAISE EXCEPTION 'Galgame CG 生成意图 line.id 必须是正整数';
        END IF;
        param_text := params #>> '{line,id}';
        IF param_text !~ '^[0-9]+$' THEN
            RAISE EXCEPTION 'Galgame CG 生成意图 line.id 必须是正整数';
        END IF;
        params_line_id := param_text::BIGINT;
        IF params_line_id <> target_id THEN
            RAISE EXCEPTION 'Galgame CG 生成意图 line.id 与目标行不一致';
        END IF;

        IF (params #> '{line,speaker_profile_id}') IS NOT NULL
           AND jsonb_typeof(params #> '{line,speaker_profile_id}') IS DISTINCT FROM 'null'
        THEN
            IF jsonb_typeof(params #> '{line,speaker_profile_id}') IS DISTINCT FROM 'number' THEN
                RAISE EXCEPTION 'Galgame CG 生成意图 line.speaker_profile_id 必须是正整数';
            END IF;
            param_text := params #>> '{line,speaker_profile_id}';
            IF param_text !~ '^[0-9]+$' THEN
                RAISE EXCEPTION 'Galgame CG 生成意图 line.speaker_profile_id 必须是正整数';
            END IF;
            params_speaker_profile_id := param_text::BIGINT;
            IF NOT EXISTS (
                SELECT 1 FROM source_snapshot_profile profile
                WHERE profile.snapshot_id = NEW.source_snapshot_id
                  AND profile.root_project_id = NEW.root_project_id
                  AND profile.profile_kind = 'character'
                  AND profile.source_profile_id = params_speaker_profile_id
            ) THEN
                RAISE EXCEPTION 'Galgame CG 生成意图 line.speaker_profile_id 必须属于源快照角色档案';
            END IF;
        END IF;

        IF jsonb_typeof(params -> 'scene') IS DISTINCT FROM 'object' THEN
            RAISE EXCEPTION 'Galgame CG 生成意图参数 scene 必须是对象';
        END IF;
        IF jsonb_typeof(params #> '{scene,id}') IS DISTINCT FROM 'number' THEN
            RAISE EXCEPTION 'Galgame CG 生成意图 scene.id 必须是正整数';
        END IF;
        param_text := params #>> '{scene,id}';
        IF param_text !~ '^[0-9]+$' THEN
            RAISE EXCEPTION 'Galgame CG 生成意图 scene.id 必须是正整数';
        END IF;
        params_scene_id := param_text::BIGINT;
        IF params_scene_id <> target_line_scene_id THEN
            RAISE EXCEPTION 'Galgame CG 生成意图 scene.id 与目标行所属场不一致';
        END IF;

        IF jsonb_typeof(params -> 'cg_note') IS DISTINCT FROM 'string'
           OR NULLIF(BTRIM(params ->> 'cg_note'), '') IS NULL
        THEN
            RAISE EXCEPTION 'Galgame CG 生成意图 cg_note 必须是非空字符串';
        END IF;
        IF params ? 'cg_importance'
           AND jsonb_typeof(params -> 'cg_importance') IS DISTINCT FROM 'null'
        THEN
            IF jsonb_typeof(params -> 'cg_importance') IS DISTINCT FROM 'string' THEN
                RAISE EXCEPTION 'Galgame CG 生成意图 cg_importance 必须是字符串';
            END IF;
            params_cg_importance := NULLIF(BTRIM(params ->> 'cg_importance'), '');
            IF params_cg_importance IS NOT NULL
               AND params_cg_importance NOT IN ('normal', 'high', 'critical')
            THEN
                RAISE EXCEPTION 'Galgame CG 生成意图 cg_importance 无效';
            END IF;
        END IF;
    ELSIF NEW.target_kind = 'portrait_expression'
       AND NEW.target_entity_kind = 'character_profile'
    THEN
        IF NEW.source_chapter_id IS NOT NULL
           OR NEW.source_snapshot_chapter_id IS NOT NULL
        THEN
            RAISE EXCEPTION 'Galgame 立绘差分生成意图必须使用源快照级上下文';
        END IF;

        IF NOT EXISTS (
            SELECT 1 FROM source_snapshot_profile profile
            WHERE profile.snapshot_id = NEW.source_snapshot_id
              AND profile.root_project_id = NEW.root_project_id
              AND profile.profile_kind = 'character'
              AND profile.source_profile_id = target_id
        ) THEN
            RAISE EXCEPTION 'Galgame 立绘差分目标角色必须属于源快照';
        END IF;

        IF jsonb_typeof(params #> '{character_profile_id}') IS DISTINCT FROM 'number' THEN
            RAISE EXCEPTION 'Galgame 立绘差分 character_profile_id 必须是正整数';
        END IF;
        param_text := params #>> '{character_profile_id}';
        IF param_text !~ '^[0-9]+$' THEN
            RAISE EXCEPTION 'Galgame 立绘差分 character_profile_id 必须是正整数';
        END IF;
        params_profile_id := param_text::BIGINT;
        IF params_profile_id <> target_id THEN
            RAISE EXCEPTION 'Galgame 立绘差分 character_profile_id 与 target_entity_id 不一致';
        END IF;

        IF jsonb_typeof(params -> 'expression') IS DISTINCT FROM 'string' THEN
            RAISE EXCEPTION 'Galgame 立绘差分 expression 必须是非空字符串';
        END IF;
        params_expression := NULLIF(BTRIM(params ->> 'expression'), '');
        IF params_expression IS NULL
           OR params_expression NOT IN ('neutral', 'smile', 'sad', 'angry', 'surprised', 'shy', 'serious')
        THEN
            RAISE EXCEPTION 'Galgame 立绘差分 expression 无效';
        END IF;

        params_pose := NULL;
        IF params ? 'pose'
           AND jsonb_typeof(params -> 'pose') IS DISTINCT FROM 'null'
        THEN
            IF jsonb_typeof(params -> 'pose') IS DISTINCT FROM 'string' THEN
                RAISE EXCEPTION 'Galgame 立绘差分 pose 必须是字符串';
            END IF;
            params_pose := NULLIF(BTRIM(params ->> 'pose'), '');
        END IF;

        IF jsonb_typeof(params -> 'canonical_key') IS DISTINCT FROM 'string' THEN
            RAISE EXCEPTION 'Galgame 立绘差分 canonical_key 缺失';
        END IF;
        IF params_pose IS NULL THEN
            expected_key := format(
                'galgame:%s:portrait:%s:expression:%s',
                NEW.project_id,
                target_id,
                params_expression
            );
        ELSE
            expected_key := format(
                'galgame:%s:portrait:%s:expression:%s:pose:%s',
                NEW.project_id,
                target_id,
                params_expression,
                params_pose
            );
        END IF;
        IF NULLIF(BTRIM(params ->> 'canonical_key'), '') IS DISTINCT FROM expected_key THEN
            RAISE EXCEPTION 'Galgame 立绘差分 canonical_key 与目标不一致';
        END IF;

        IF jsonb_typeof(params -> 'source_line_ids') IS DISTINCT FROM 'array'
           OR jsonb_array_length(params -> 'source_line_ids') = 0
        THEN
            RAISE EXCEPTION 'Galgame 立绘差分 source_line_ids 必须是非空数组';
        END IF;
        IF EXISTS (
            SELECT 1
            FROM jsonb_array_elements(params -> 'source_line_ids') AS elem(value)
            WHERE jsonb_typeof(elem.value) IS DISTINCT FROM 'number'
        ) THEN
            RAISE EXCEPTION 'Galgame 立绘差分 source_line_ids 必须是正整数数组';
        END IF;
        IF EXISTS (
            SELECT 1
            FROM jsonb_array_elements(params -> 'source_line_ids') AS elem(value)
            WHERE (elem.value #>> '{}') !~ '^[0-9]+$'
        ) THEN
            RAISE EXCEPTION 'Galgame 立绘差分 source_line_ids 必须是正整数数组';
        END IF;
        IF EXISTS (
            SELECT 1
            FROM jsonb_array_elements(params -> 'source_line_ids') AS elem(value)
            WHERE (elem.value #>> '{}')::BIGINT <= 0
        ) THEN
            RAISE EXCEPTION 'Galgame 立绘差分 source_line_ids 必须是正整数数组';
        END IF;
        SELECT COUNT(*), COUNT(DISTINCT (elem.value #>> '{}')::BIGINT)
          INTO source_line_count, source_line_distinct_count
        FROM jsonb_array_elements(params -> 'source_line_ids') AS elem(value);
        IF source_line_count <> source_line_distinct_count THEN
            RAISE EXCEPTION 'Galgame 立绘差分 source_line_ids 不能包含重复 ID';
        END IF;

        IF EXISTS (
            WITH source_ids AS (
                SELECT (elem.value #>> '{}')::BIGINT AS line_id
                FROM jsonb_array_elements(params -> 'source_line_ids') AS elem(value)
            )
            SELECT 1
            FROM source_ids source_id
            LEFT JOIN galgame_line line_row ON line_row.id = source_id.line_id
            LEFT JOIN galgame_scene scene_row ON scene_row.id = line_row.scene_id
            LEFT JOIN galgame_script script_row ON script_row.id = scene_row.script_id
            LEFT JOIN source_snapshot_chapter chapter_row ON chapter_row.id = scene_row.source_snapshot_chapter_id
            WHERE line_row.id IS NULL
               OR line_row.speaker_profile_id IS DISTINCT FROM target_id
               OR NULLIF(BTRIM(COALESCE(line_row.expression, '')), '') IS DISTINCT FROM params_expression
               OR COALESCE(NULLIF(BTRIM(line_row.pose), ''), '') IS DISTINCT FROM COALESCE(params_pose, '')
               OR script_row.project_id IS DISTINCT FROM NEW.project_id
               OR script_row.root_project_id IS DISTINCT FROM NEW.root_project_id
               OR script_row.source_snapshot_id IS DISTINCT FROM NEW.source_snapshot_id
               OR chapter_row.snapshot_id IS DISTINCT FROM NEW.source_snapshot_id
               OR chapter_row.root_project_id IS DISTINCT FROM NEW.root_project_id
               OR (
                   NULLIF(BTRIM(line_row.position), '') IS NOT NULL
                   AND NULLIF(BTRIM(line_row.position), '') NOT IN ('left', 'center', 'right')
               )
        ) THEN
            RAISE EXCEPTION 'Galgame 立绘差分 source_line_ids 行上下文不一致';
        END IF;

        IF jsonb_typeof(params -> 'position_samples') IS DISTINCT FROM 'array' THEN
            RAISE EXCEPTION 'Galgame 立绘差分 position_samples 必须是数组';
        END IF;
        IF EXISTS (
            SELECT 1
            FROM jsonb_array_elements(params -> 'position_samples') AS elem(value)
            WHERE jsonb_typeof(elem.value) IS DISTINCT FROM 'string'
               OR NULLIF(BTRIM(elem.value #>> '{}'), '') IS NULL
               OR BTRIM(elem.value #>> '{}') NOT IN ('left', 'center', 'right')
        ) THEN
            RAISE EXCEPTION 'Galgame 立绘差分 position_samples 包含无效站位';
        END IF;
        SELECT COALESCE(array_agg(DISTINCT sample_position ORDER BY sample_position), ARRAY[]::TEXT[])
          INTO sample_positions
        FROM (
            SELECT BTRIM(elem.value #>> '{}') AS sample_position
            FROM jsonb_array_elements(params -> 'position_samples') AS elem(value)
        ) samples;
        WITH source_ids AS (
            SELECT (elem.value #>> '{}')::BIGINT AS line_id
            FROM jsonb_array_elements(params -> 'source_line_ids') AS elem(value)
        )
        SELECT COALESCE(array_agg(DISTINCT source_position ORDER BY source_position), ARRAY[]::TEXT[])
          INTO source_positions
        FROM (
            SELECT NULLIF(BTRIM(line_row.position), '') AS source_position
            FROM source_ids source_id
            JOIN galgame_line line_row ON line_row.id = source_id.line_id
        ) positions
        WHERE source_position IS NOT NULL;
        IF sample_positions IS DISTINCT FROM source_positions THEN
            RAISE EXCEPTION 'Galgame 立绘差分 position_samples 与 source_line_ids 行站位不一致';
        END IF;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM generation_spec_text_anchor spec_anchor
        LEFT JOIN text_anchor anchor ON anchor.id = spec_anchor.text_anchor_id
        WHERE spec_anchor.generation_spec_id = NEW.id
          AND (
              anchor.id IS NULL
              OR anchor.project_id IS DISTINCT FROM NEW.root_project_id
              OR anchor.source_snapshot_id IS DISTINCT FROM NEW.source_snapshot_id
              OR (
                  NEW.source_snapshot_chapter_id IS NOT NULL
                  AND anchor.source_snapshot_chapter_id IS DISTINCT FROM NEW.source_snapshot_chapter_id
              )
          )
    ) THEN
        RAISE EXCEPTION '生成意图已有文本锚点必须属于更新后的同一根项目、源快照和源快照章节';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM generation_spec_entity_ref entity_ref
        LEFT JOIN source_snapshot_profile profile
          ON profile.snapshot_id = NEW.source_snapshot_id
         AND profile.root_project_id = NEW.root_project_id
         AND profile.profile_kind = CASE
             WHEN entity_ref.entity_kind = 'character_profile' THEN 'character'
             ELSE entity_ref.entity_kind
         END
         AND profile.source_profile_id = entity_ref.entity_id
        WHERE entity_ref.generation_spec_id = NEW.id
          AND (
              context_kind <> 'snapshot_backed'
              OR NEW.source_snapshot_id IS NULL
              OR entity_ref.entity_kind = 'event'
              OR profile.id IS NULL
          )
    ) THEN
        RAISE EXCEPTION '生成意图已有实体引用必须属于更新后的源快照档案';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM generation_spec_media_ref media_ref
        LEFT JOIN media_asset asset ON asset.id = media_ref.media_asset_id
        WHERE media_ref.generation_spec_id = NEW.id
          AND (
              media_ref.role NOT IN (
                  'first_frame',
                  'end_frame',
                  'character_reference',
                  'object_reference',
                  'scene_reference',
                  'style_reference',
                  'reference'
              )
              OR asset.id IS NULL
              OR asset.project_id IS DISTINCT FROM NEW.project_id
              OR asset.root_project_id IS DISTINCT FROM NEW.root_project_id
              OR asset.status <> 'completed'
              OR asset.file_path IS NULL
          )
    ) THEN
        RAISE EXCEPTION '生成意图已有媒体引用必须属于更新后的同一项目族且指向已完成媒体资产';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM media_provenance provenance
        LEFT JOIN media_asset asset ON asset.id = provenance.media_asset_id
        WHERE provenance.generation_spec_id = NEW.id
          AND (
              asset.id IS NULL
              OR asset.project_id IS DISTINCT FROM NEW.project_id
              OR asset.root_project_id IS DISTINCT FROM NEW.root_project_id
              OR NOT (
                  (asset.source_snapshot_id IS NULL
                   AND NEW.source_snapshot_id IS NULL
                   AND NEW.source_snapshot_chapter_id IS NULL)
                  OR NEW.source_snapshot_id = asset.source_snapshot_id
              )
              OR (
                  asset.source_snapshot_chapter_id IS NOT NULL
                  AND NEW.source_snapshot_chapter_id IS DISTINCT FROM asset.source_snapshot_chapter_id
              )
              OR (
                  asset.source_chapter_id IS NOT NULL
                  AND NEW.source_chapter_id IS DISTINCT FROM asset.source_chapter_id
              )
          )
    ) THEN
        RAISE EXCEPTION '生成意图已有媒体来源必须匹配更新后的项目族和源快照上下文';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM media_provenance provenance
        JOIN prompt_artifact prompt ON prompt.id = provenance.prompt_artifact_id
        LEFT JOIN media_asset asset ON asset.id = provenance.media_asset_id
        WHERE prompt.generation_spec_id = NEW.id
          AND (
              asset.id IS NULL
              OR (
                  provenance.generation_spec_id IS NOT NULL
                  AND provenance.generation_spec_id IS DISTINCT FROM NEW.id
              )
              OR asset.project_id IS DISTINCT FROM NEW.project_id
              OR asset.root_project_id IS DISTINCT FROM NEW.root_project_id
              OR NOT (
                  (asset.source_snapshot_id IS NULL
                   AND NEW.source_snapshot_id IS NULL
                   AND NEW.source_snapshot_chapter_id IS NULL)
                  OR NEW.source_snapshot_id = asset.source_snapshot_id
              )
              OR (
                  asset.source_snapshot_chapter_id IS NOT NULL
                  AND NEW.source_snapshot_chapter_id IS DISTINCT FROM asset.source_snapshot_chapter_id
              )
              OR (
                  asset.source_chapter_id IS NOT NULL
                  AND NEW.source_chapter_id IS DISTINCT FROM asset.source_chapter_id
              )
          )
    ) THEN
        RAISE EXCEPTION '生成意图已有提示词媒体来源必须匹配更新后的项目族和源快照上下文';
    END IF;

    IF NEW.target_kind IN ('galgame_background', 'galgame_cg')
       AND EXISTS (
           SELECT 1
           FROM media_provenance provenance
           WHERE provenance.generation_spec_id = NEW.id
             AND (
                 provenance.request_snapshot_json IS NULL
                 OR jsonb_typeof(provenance.request_snapshot_json::jsonb) IS DISTINCT FROM 'object'
                 OR BTRIM(provenance.request_snapshot_json::jsonb ->> 'source') IS DISTINCT FROM 'galgame_asset_generation'
             )
       )
    THEN
        RAISE EXCEPTION 'Galgame 生成意图已有媒体来源必须保留 galgame_asset_generation request snapshot';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM media_provenance provenance
        WHERE provenance.generation_spec_id = NEW.id
          AND provenance.request_snapshot_json IS NOT NULL
          AND jsonb_typeof(provenance.request_snapshot_json::jsonb) = 'object'
          AND BTRIM(provenance.request_snapshot_json::jsonb ->> 'source') = 'galgame_asset_generation'
          AND (
              BTRIM(provenance.request_snapshot_json::jsonb ->> 'target_kind') IS DISTINCT FROM NEW.target_kind
              OR BTRIM(provenance.request_snapshot_json::jsonb ->> 'target_entity_kind') IS DISTINCT FROM NEW.target_entity_kind
              OR BTRIM(provenance.request_snapshot_json::jsonb ->> 'target_entity_id') IS DISTINCT FROM NEW.target_entity_id::TEXT
          )
    ) THEN
        RAISE EXCEPTION 'Galgame 生成意图已有媒体来源 request_snapshot_json 必须匹配更新后的目标';
    END IF;

    NEW.source_context_kind = context_kind;
    RETURN NEW;
END;
$$
"#;

pub(crate) const GENERATION_SPEC_TEXT_ANCHOR_CHAPTER_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_text_anchor_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    spec_context_kind TEXT;
    spec_source_snapshot_chapter_id BIGINT;
    spec_source_chapter_id BIGINT;
    anchor_source_snapshot_chapter_id BIGINT;
    anchor_source_chapter_id BIGINT;
BEGIN
    SELECT COALESCE(NULLIF(spec.source_context_kind, ''), 'snapshot_backed'),
           spec.source_snapshot_chapter_id,
           spec.source_chapter_id,
           anchor.source_snapshot_chapter_id,
           anchor.source_chapter_id
      INTO spec_context_kind,
           spec_source_snapshot_chapter_id,
           spec_source_chapter_id,
           anchor_source_snapshot_chapter_id,
           anchor_source_chapter_id
    FROM generation_spec spec
    JOIN text_anchor anchor ON anchor.id = NEW.text_anchor_id
    WHERE spec.id = NEW.generation_spec_id
      AND anchor.project_id = spec.root_project_id
      AND anchor.source_snapshot_id = spec.source_snapshot_id;

    IF spec_context_kind IS NULL THEN
        RAISE EXCEPTION '生成意图文本锚点必须属于同一根项目和源快照';
    END IF;
    IF spec_context_kind = 'freeform' THEN
        RAISE EXCEPTION 'freeform 生成意图不能绑定 text_anchor';
    END IF;
    IF spec_source_snapshot_chapter_id IS NOT NULL THEN
        IF anchor_source_snapshot_chapter_id IS DISTINCT FROM spec_source_snapshot_chapter_id THEN
            RAISE EXCEPTION '生成意图文本锚点必须属于同一源快照章节';
        END IF;
        IF spec_source_chapter_id IS NOT NULL
           AND anchor_source_chapter_id IS DISTINCT FROM spec_source_chapter_id
        THEN
            RAISE EXCEPTION '生成意图文本锚点 source_chapter_id 必须与生成意图一致';
        END IF;
    END IF;

    RETURN NEW;
END;
$$
"#;

pub(crate) const GENERATION_SPEC_ENTITY_REF_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_entity_ref_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    spec_context_kind TEXT;
    spec_root_project_id BIGINT;
    spec_source_snapshot_id BIGINT;
    snapshot_profile_kind TEXT;
BEGIN
    IF NEW.entity_id <= 0 THEN
        RAISE EXCEPTION '生成意图实体引用 entity_id 必须是正整数';
    END IF;
    IF NEW.entity_kind NOT IN ('character', 'character_profile', 'object', 'scene', 'event') THEN
        RAISE EXCEPTION '生成意图实体引用 entity_kind 无效';
    END IF;
    IF NEW.role NOT IN ('subject', 'supporting', 'location', 'prop', 'continuity_reference') THEN
        RAISE EXCEPTION '生成意图实体引用 role 无效';
    END IF;
    IF NEW.entity_kind = 'event' THEN
        RAISE EXCEPTION '事件档案尚未建立持久模型，不能作为生成意图实体引用';
    END IF;

    SELECT COALESCE(NULLIF(spec.source_context_kind, ''), 'snapshot_backed'),
           spec.root_project_id,
           spec.source_snapshot_id
      INTO spec_context_kind,
           spec_root_project_id,
           spec_source_snapshot_id
    FROM generation_spec spec
    WHERE spec.id = NEW.generation_spec_id;

    IF spec_context_kind IS NULL THEN
        RAISE EXCEPTION '生成意图实体引用 generation_spec_id 不存在';
    END IF;
    IF spec_context_kind <> 'snapshot_backed' OR spec_source_snapshot_id IS NULL THEN
        RAISE EXCEPTION '生成意图实体引用必须属于 snapshot_backed 生成意图';
    END IF;

    snapshot_profile_kind := CASE
        WHEN NEW.entity_kind = 'character_profile' THEN 'character'
        ELSE NEW.entity_kind
    END;
    IF NOT EXISTS (
        SELECT 1
        FROM source_snapshot_profile profile
        WHERE profile.snapshot_id = spec_source_snapshot_id
          AND profile.root_project_id = spec_root_project_id
          AND profile.profile_kind = snapshot_profile_kind
          AND profile.source_profile_id = NEW.entity_id
    ) THEN
        RAISE EXCEPTION '生成意图实体引用必须属于生成意图源快照';
    END IF;

    RETURN NEW;
END;
$$
"#;

pub(crate) const GENERATION_SPEC_ENTITY_REF_FAMILY_INSERT_TRIGGER_SQL: &str = r#"
CREATE TRIGGER trg_generation_spec_entity_ref_family_insert
BEFORE INSERT ON generation_spec_entity_ref
FOR EACH ROW EXECUTE FUNCTION a4n_validate_generation_spec_entity_ref_family()
"#;

pub(crate) const GENERATION_SPEC_ENTITY_REF_FAMILY_UPDATE_TRIGGER_SQL: &str = r#"
CREATE TRIGGER trg_generation_spec_entity_ref_family_update
BEFORE UPDATE OF generation_spec_id, entity_kind, entity_id, role ON generation_spec_entity_ref
FOR EACH ROW EXECUTE FUNCTION a4n_validate_generation_spec_entity_ref_family()
"#;

const GENERIC_GENERATION_SPEC_FAMILY_UPDATE_TRIGGER_SQL: &str = r#"
CREATE TRIGGER trg_generation_spec_family_update
BEFORE UPDATE OF project_id, root_project_id, source_context_kind, source_snapshot_id, source_chapter_id, source_snapshot_chapter_id, canvas_text_asset_id ON generation_spec
FOR EACH ROW EXECUTE FUNCTION a4n_validate_generation_spec_family()
"#;

const GENERIC_GENERATION_SPEC_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    context_kind TEXT := COALESCE(NULLIF(NEW.source_context_kind, ''), 'snapshot_backed');
BEGIN
    IF context_kind NOT IN ('snapshot_backed', 'freeform') THEN
        RAISE EXCEPTION '生成意图 source_context_kind 无效';
    END IF;

    IF context_kind = 'snapshot_backed' THEN
        IF NOT EXISTS (
            SELECT 1 FROM novel_project root
            WHERE root.id = NEW.root_project_id
              AND root.project_kind = 'text'
        ) THEN
            RAISE EXCEPTION '生成意图 root_project_id 必须指向文本项目';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM novel_project project_row
            WHERE project_row.id = NEW.project_id
              AND (
                  (project_row.project_kind = 'text' AND project_row.id = NEW.root_project_id)
                  OR (project_row.project_kind <> 'text' AND project_row.root_project_id = NEW.root_project_id)
              )
        ) THEN
            RAISE EXCEPTION '生成意图 project_id 必须属于同一项目族';
        END IF;
        IF NEW.source_snapshot_id IS NULL THEN
            RAISE EXCEPTION 'snapshot_backed 生成意图必须包含 source_snapshot_id';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM project_source_snapshot snapshot
            WHERE snapshot.id = NEW.source_snapshot_id
              AND snapshot.root_project_id = NEW.root_project_id
        ) THEN
            RAISE EXCEPTION '生成意图 source_snapshot_id 必须属于根文本项目';
        END IF;
        IF NEW.source_snapshot_chapter_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM source_snapshot_chapter chapter
            WHERE chapter.id = NEW.source_snapshot_chapter_id
              AND chapter.snapshot_id = NEW.source_snapshot_id
              AND chapter.root_project_id = NEW.root_project_id
              AND (NEW.source_chapter_id IS NULL OR chapter.source_chapter_id = NEW.source_chapter_id)
        ) THEN
            RAISE EXCEPTION '生成意图 source_snapshot_chapter_id 必须属于目标快照';
        END IF;
    ELSE
        IF NOT EXISTS (
            SELECT 1 FROM novel_project project_row
            WHERE project_row.id = NEW.project_id
              AND project_row.project_kind = 'video'
              AND project_row.source_mode = 'freeform'
        ) THEN
            RAISE EXCEPTION 'freeform 生成意图必须属于自由视频项目';
        END IF;
        IF NEW.root_project_id <> NEW.project_id THEN
            RAISE EXCEPTION 'freeform 生成意图 root_project_id 必须等于 project_id';
        END IF;
        IF NEW.source_snapshot_id IS NOT NULL
            OR NEW.source_chapter_id IS NOT NULL
            OR NEW.source_snapshot_chapter_id IS NOT NULL
        THEN
            RAISE EXCEPTION 'freeform 生成意图不能绑定源快照或源章节';
        END IF;
        IF NEW.canvas_text_asset_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM canvas_text_asset asset
            WHERE asset.id = NEW.canvas_text_asset_id
              AND asset.project_id = NEW.project_id
        ) THEN
            RAISE EXCEPTION 'freeform 生成意图 canvas_text_asset_id 必须属于当前项目';
        END IF;
    END IF;

    NEW.source_context_kind = context_kind;
    RETURN NEW;
END;
$$
"#;

const GENERIC_GENERATION_SPEC_TEXT_ANCHOR_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_text_anchor_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM generation_spec spec
        WHERE spec.id = NEW.generation_spec_id
          AND COALESCE(NULLIF(spec.source_context_kind, ''), 'snapshot_backed') = 'freeform'
    ) THEN
        RAISE EXCEPTION 'freeform 生成意图不能绑定 text_anchor';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM generation_spec spec
        JOIN text_anchor anchor ON anchor.id = NEW.text_anchor_id
        WHERE spec.id = NEW.generation_spec_id
          AND anchor.project_id = spec.root_project_id
          AND anchor.source_snapshot_id = spec.source_snapshot_id
    ) THEN
        RAISE EXCEPTION '生成意图文本锚点必须属于同一根项目和源快照';
    END IF;
    RETURN NEW;
END;
$$
"#;

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_spec_family_trigger_enforces_galgame_asset_contract() {
        for expected in [
            "Galgame 生成意图 target_kind/target_entity_kind 配对无效",
            "Galgame 生成意图 target_entity_id 必填",
            "Galgame 背景生成意图 scene.id 与目标场不一致",
            "Galgame CG 生成意图 scene.id 与目标行所属场不一致",
            "Galgame 立绘差分目标角色必须属于源快照",
            "Galgame 立绘差分 source_line_ids 行上下文不一致",
            "Galgame 立绘差分 position_samples 与 source_line_ids 行站位不一致",
            "生成意图已有媒体引用必须属于更新后的同一项目族且指向已完成媒体资产",
            "生成意图已有媒体来源必须匹配更新后的项目族和源快照上下文",
            "生成意图已有提示词媒体来源必须匹配更新后的项目族和源快照上下文",
            "Galgame 生成意图已有媒体来源 request_snapshot_json 必须匹配更新后的目标",
        ] {
            assert!(
                GALGAME_GENERATION_SPEC_FAMILY_FUNCTION_SQL.contains(expected),
                "missing Galgame generation_spec trigger contract: {expected}"
            );
        }
    }

    #[test]
    fn generation_spec_update_trigger_covers_galgame_target_and_params() {
        for expected in [
            "target_kind",
            "target_entity_kind",
            "target_entity_id",
            "params_json ON generation_spec",
        ] {
            assert!(
                GALGAME_GENERATION_SPEC_FAMILY_UPDATE_TRIGGER_SQL.contains(expected),
                "generation_spec update trigger must cover: {expected}"
            );
        }
    }

    #[test]
    fn generation_spec_text_anchor_trigger_enforces_chapter_scope() {
        for expected in [
            "生成意图文本锚点必须属于同一源快照章节",
            "生成意图文本锚点 source_chapter_id 必须与生成意图一致",
            "生成意图已有文本锚点必须属于更新后的同一根项目、源快照和源快照章节",
        ] {
            assert!(
                GENERATION_SPEC_TEXT_ANCHOR_CHAPTER_FAMILY_FUNCTION_SQL.contains(expected)
                    || GALGAME_GENERATION_SPEC_FAMILY_FUNCTION_SQL.contains(expected),
                "missing generation_spec text_anchor chapter contract: {expected}"
            );
        }
    }

    #[test]
    fn generation_spec_entity_ref_trigger_enforces_snapshot_profile_scope() {
        for expected in [
            "生成意图实体引用 entity_kind 无效",
            "生成意图实体引用 role 无效",
            "事件档案尚未建立持久模型，不能作为生成意图实体引用",
            "生成意图实体引用必须属于生成意图源快照",
            "生成意图已有实体引用必须属于更新后的源快照档案",
            "CREATE TRIGGER trg_generation_spec_entity_ref_family_insert",
            "CREATE TRIGGER trg_generation_spec_entity_ref_family_update",
        ] {
            assert!(
                GENERATION_SPEC_ENTITY_REF_FAMILY_FUNCTION_SQL.contains(expected)
                    || GALGAME_GENERATION_SPEC_FAMILY_FUNCTION_SQL.contains(expected)
                    || GENERATION_SPEC_ENTITY_REF_FAMILY_INSERT_TRIGGER_SQL.contains(expected)
                    || GENERATION_SPEC_ENTITY_REF_FAMILY_UPDATE_TRIGGER_SQL.contains(expected),
                "missing generation_spec entity_ref trigger contract: {expected}"
            );
        }
    }
}
