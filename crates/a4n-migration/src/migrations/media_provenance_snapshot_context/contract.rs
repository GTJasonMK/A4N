pub(crate) const MEDIA_PROVENANCE_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_media_provenance_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    request_source TEXT;
BEGIN
    IF NEW.generation_spec_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM media_asset asset
        JOIN generation_spec spec ON spec.id = NEW.generation_spec_id
        WHERE asset.id = NEW.media_asset_id
          AND spec.project_id = asset.project_id
          AND spec.root_project_id = asset.root_project_id
          AND (
              (asset.source_snapshot_id IS NULL
               AND spec.source_snapshot_id IS NULL
               AND spec.source_snapshot_chapter_id IS NULL)
              OR spec.source_snapshot_id = asset.source_snapshot_id
          )
          AND (
              asset.source_snapshot_chapter_id IS NULL
              OR spec.source_snapshot_chapter_id = asset.source_snapshot_chapter_id
          )
          AND (
              asset.source_chapter_id IS NULL
              OR spec.source_chapter_id = asset.source_chapter_id
          )
    ) THEN
        RAISE EXCEPTION '媒体来源 generation_spec_id 必须匹配媒体资产项目族和源快照上下文';
    END IF;
    IF NEW.prompt_artifact_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM prompt_artifact prompt
        JOIN generation_spec spec ON spec.id = prompt.generation_spec_id
        JOIN media_asset asset ON asset.id = NEW.media_asset_id
        WHERE prompt.id = NEW.prompt_artifact_id
          AND spec.project_id = asset.project_id
          AND spec.root_project_id = asset.root_project_id
          AND (NEW.generation_spec_id IS NULL OR spec.id = NEW.generation_spec_id)
          AND (
              (asset.source_snapshot_id IS NULL
               AND spec.source_snapshot_id IS NULL
               AND spec.source_snapshot_chapter_id IS NULL)
              OR spec.source_snapshot_id = asset.source_snapshot_id
          )
          AND (
              asset.source_snapshot_chapter_id IS NULL
              OR spec.source_snapshot_chapter_id = asset.source_snapshot_chapter_id
          )
          AND (
              asset.source_chapter_id IS NULL
              OR spec.source_chapter_id = asset.source_chapter_id
          )
    ) THEN
        RAISE EXCEPTION '媒体来源 prompt_artifact_id 必须匹配媒体资产项目族和源快照上下文';
    END IF;
    IF NEW.source_media_asset_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM media_asset asset
        JOIN media_asset source ON source.id = NEW.source_media_asset_id
        WHERE asset.id = NEW.media_asset_id
          AND source.project_id = asset.project_id
          AND source.root_project_id = asset.root_project_id
          AND source.status = 'completed'
          AND source.file_path IS NOT NULL
          AND (
              (asset.source_snapshot_id IS NULL
               AND source.source_snapshot_id IS NULL
               AND source.source_snapshot_chapter_id IS NULL)
              OR source.source_snapshot_id = asset.source_snapshot_id
          )
          AND (
              asset.source_snapshot_chapter_id IS NULL
              OR source.source_snapshot_chapter_id = asset.source_snapshot_chapter_id
          )
          AND (
              asset.source_chapter_id IS NULL
              OR source.source_chapter_id = asset.source_chapter_id
          )
    ) THEN
        RAISE EXCEPTION '媒体来源 source_media_asset_id 必须匹配媒体资产项目族和源快照上下文';
    END IF;
    IF NEW.request_snapshot_json IS NOT NULL THEN
        IF jsonb_typeof(NEW.request_snapshot_json::jsonb) IS DISTINCT FROM 'object' THEN
            RAISE EXCEPTION '媒体来源 request_snapshot_json 必须是 JSON 对象';
        END IF;
        request_source := BTRIM(NEW.request_snapshot_json::jsonb ->> 'source');
    END IF;

    IF NEW.generation_spec_id IS NOT NULL AND EXISTS (
        SELECT 1
        FROM generation_spec spec
        WHERE spec.id = NEW.generation_spec_id
          AND (
              (spec.target_kind = 'galgame_background' AND spec.target_entity_kind = 'galgame_scene')
              OR (spec.target_kind = 'galgame_cg' AND spec.target_entity_kind = 'galgame_line')
          )
    ) THEN
        IF request_source IS DISTINCT FROM 'galgame_asset_generation' THEN
            RAISE EXCEPTION 'Galgame 资产媒体来源 request_snapshot_json.source 必须是 galgame_asset_generation';
        END IF;
    END IF;

    IF request_source = 'galgame_asset_generation' THEN
        IF NEW.generation_spec_id IS NULL THEN
            RAISE EXCEPTION 'Galgame 资产媒体来源缺少 generation_spec_id';
        END IF;
        IF jsonb_typeof(NEW.request_snapshot_json::jsonb -> 'target_kind') IS DISTINCT FROM 'string'
           OR BTRIM(NEW.request_snapshot_json::jsonb ->> 'target_kind') = '' THEN
            RAISE EXCEPTION 'Galgame 资产媒体来源 request_snapshot_json.target_kind 缺失';
        END IF;
        IF jsonb_typeof(NEW.request_snapshot_json::jsonb -> 'target_entity_kind') IS DISTINCT FROM 'string'
           OR BTRIM(NEW.request_snapshot_json::jsonb ->> 'target_entity_kind') = '' THEN
            RAISE EXCEPTION 'Galgame 资产媒体来源 request_snapshot_json.target_entity_kind 缺失';
        END IF;
        IF jsonb_typeof(NEW.request_snapshot_json::jsonb -> 'target_entity_id') IS DISTINCT FROM 'number'
           OR NOT ((NEW.request_snapshot_json::jsonb ->> 'target_entity_id') ~ '^[0-9]+$') THEN
            RAISE EXCEPTION 'Galgame 资产媒体来源 request_snapshot_json.target_entity_id 缺失';
        END IF;
        IF NOT EXISTS (
            SELECT 1
            FROM generation_spec spec
            WHERE spec.id = NEW.generation_spec_id
              AND spec.target_kind = BTRIM(NEW.request_snapshot_json::jsonb ->> 'target_kind')
              AND spec.target_entity_kind = BTRIM(NEW.request_snapshot_json::jsonb ->> 'target_entity_kind')
              AND spec.target_entity_id = (NEW.request_snapshot_json::jsonb ->> 'target_entity_id')::BIGINT
        ) THEN
            RAISE EXCEPTION 'Galgame 资产媒体来源 request_snapshot_json 与 generation_spec 目标不一致';
        END IF;
    END IF;

    IF request_source = 'galgame_audio_import'
       AND NOT (NEW.request_snapshot_json::jsonb ? 'target_line_id') THEN
        RAISE EXCEPTION 'Galgame 音频导入媒体来源 request_snapshot_json.target_line_id 缺失';
    END IF;
    IF NEW.request_snapshot_json IS NOT NULL
       AND NEW.request_snapshot_json::jsonb ? 'target_line_id' THEN
        IF jsonb_typeof(NEW.request_snapshot_json::jsonb -> 'target_line_id') IS DISTINCT FROM 'number'
           OR NOT ((NEW.request_snapshot_json::jsonb ->> 'target_line_id') ~ '^[0-9]+$') THEN
            RAISE EXCEPTION '媒体来源 request_snapshot_json.target_line_id 必须是整数';
        END IF;
        IF NOT EXISTS (
            SELECT 1
            FROM media_asset asset
            JOIN galgame_line line ON line.id = (NEW.request_snapshot_json::jsonb ->> 'target_line_id')::BIGINT
            JOIN galgame_scene scene ON scene.id = line.scene_id
            JOIN galgame_script script ON script.id = scene.script_id
            WHERE asset.id = NEW.media_asset_id
              AND asset.project_id = script.project_id
              AND asset.root_project_id = script.root_project_id
              AND asset.source_snapshot_id = script.source_snapshot_id
        ) THEN
            RAISE EXCEPTION '媒体来源 request_snapshot_json.target_line_id 必须指向同项目 Galgame 行';
        END IF;
    END IF;
    RETURN NEW;
END;
$$
"#;
