pub(crate) const MEDIA_ASSET_FAMILY_STRICT_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_media_asset_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    project_kind TEXT;
    project_root_project_id BIGINT;
    project_source_snapshot_id BIGINT;
BEGIN
    SELECT project_row.project_kind,
           project_row.root_project_id,
           project_row.source_snapshot_id
      INTO project_kind,
           project_root_project_id,
           project_source_snapshot_id
    FROM novel_project project_row
    WHERE project_row.id = NEW.project_id;

    IF project_kind IS NULL THEN
        RAISE EXCEPTION '媒体资产 project_id 不存在';
    END IF;

    IF project_kind = 'video'
       AND project_root_project_id IS NULL
       AND project_source_snapshot_id IS NULL THEN
        IF NEW.root_project_id <> NEW.project_id THEN
            RAISE EXCEPTION 'freeform 媒体资产 root_project_id 必须等于 project_id';
        END IF;
        IF NEW.source_snapshot_id IS NOT NULL
           OR NEW.source_chapter_id IS NOT NULL
           OR NEW.source_snapshot_chapter_id IS NOT NULL THEN
            RAISE EXCEPTION 'freeform 媒体资产不能绑定源快照或源章节';
        END IF;
    ELSE
        IF NOT EXISTS (
            SELECT 1 FROM novel_project root
            WHERE root.id = NEW.root_project_id
              AND root.project_kind = 'text'
              AND root.root_project_id IS NULL
              AND root.source_snapshot_id IS NULL
        ) THEN
            RAISE EXCEPTION '媒体资产 root_project_id 必须指向文本项目';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM novel_project project_row
            WHERE project_row.id = NEW.project_id
              AND (
                  (project_row.project_kind = 'text' AND project_row.id = NEW.root_project_id)
                  OR (
                      project_row.project_kind IN ('comic','video','galgame')
                      AND project_row.root_project_id = NEW.root_project_id
                      AND project_row.source_snapshot_id IS NOT NULL
                  )
              )
        ) THEN
            RAISE EXCEPTION '媒体资产 project_id 必须属于同一项目族';
        END IF;
        IF NEW.source_snapshot_id IS NULL THEN
            RAISE EXCEPTION 'snapshot-backed 媒体资产必须包含 source_snapshot_id';
        END IF;
        IF project_kind <> 'text'
           AND project_source_snapshot_id IS DISTINCT FROM NEW.source_snapshot_id THEN
            RAISE EXCEPTION '媒体资产 source_snapshot_id 必须等于派生项目源快照';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM project_source_snapshot snapshot
            WHERE snapshot.id = NEW.source_snapshot_id
              AND snapshot.root_project_id = NEW.root_project_id
        ) THEN
            RAISE EXCEPTION '媒体资产 source_snapshot_id 必须属于根文本项目';
        END IF;
        IF (NEW.source_chapter_id IS NULL) <> (NEW.source_snapshot_chapter_id IS NULL) THEN
            RAISE EXCEPTION '媒体资产 source_chapter_id 和 source_snapshot_chapter_id 必须同时提供或同时为空';
        END IF;
        IF NEW.source_snapshot_chapter_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM source_snapshot_chapter chapter
            WHERE chapter.id = NEW.source_snapshot_chapter_id
              AND chapter.root_project_id = NEW.root_project_id
              AND chapter.snapshot_id = NEW.source_snapshot_id
              AND chapter.source_chapter_id = NEW.source_chapter_id
        ) THEN
            RAISE EXCEPTION '媒体资产 source_snapshot_chapter_id 必须属于同一源快照章节';
        END IF;
    END IF;

    IF NEW.status = 'completed' AND NEW.file_path IS NULL THEN
        RAISE EXCEPTION 'completed 媒体资产必须有 file_path';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM generation_spec_media_ref media_ref
        LEFT JOIN generation_spec spec ON spec.id = media_ref.generation_spec_id
        WHERE media_ref.media_asset_id = NEW.id
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
              OR spec.id IS NULL
              OR spec.project_id IS DISTINCT FROM NEW.project_id
              OR spec.root_project_id IS DISTINCT FROM NEW.root_project_id
              OR NEW.status <> 'completed'
              OR NEW.file_path IS NULL
          )
    ) THEN
        RAISE EXCEPTION '媒体资产更新会破坏已有生成意图媒体引用';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM media_binding binding
        LEFT JOIN galgame_scene scene ON scene.id = binding.target_id
        LEFT JOIN galgame_script script ON script.id = scene.script_id
        LEFT JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id
        WHERE binding.media_asset_id = NEW.id
          AND binding.target_kind = 'galgame_scene'
          AND (
              binding.role <> 'background'
              OR scene.id IS NULL
              OR script.id IS NULL
              OR NEW.project_id IS DISTINCT FROM script.project_id
              OR NEW.root_project_id IS DISTINCT FROM script.root_project_id
              OR NEW.source_snapshot_id IS DISTINCT FROM script.source_snapshot_id
              OR NEW.media_kind <> 'image'
              OR NEW.status <> 'completed'
              OR NEW.file_path IS NULL
              OR (
                  NEW.source_snapshot_chapter_id IS NOT NULL
                  AND NEW.source_snapshot_chapter_id IS DISTINCT FROM scene.source_snapshot_chapter_id
              )
              OR (
                  NEW.source_chapter_id IS NOT NULL
                  AND chapter.source_chapter_id IS DISTINCT FROM NEW.source_chapter_id
              )
          )
    ) THEN
        RAISE EXCEPTION '媒体资产更新会破坏已有 Galgame 场媒体绑定';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM media_binding binding
        LEFT JOIN galgame_line line_row ON line_row.id = binding.target_id
        LEFT JOIN galgame_scene scene ON scene.id = line_row.scene_id
        LEFT JOIN galgame_script script ON script.id = scene.script_id
        LEFT JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id
        WHERE binding.media_asset_id = NEW.id
          AND binding.target_kind = 'galgame_line'
          AND (
              binding.role NOT IN ('cg', 'bgm', 'sfx')
              OR line_row.id IS NULL
              OR scene.id IS NULL
              OR script.id IS NULL
              OR NEW.project_id IS DISTINCT FROM script.project_id
              OR NEW.root_project_id IS DISTINCT FROM script.root_project_id
              OR NEW.source_snapshot_id IS DISTINCT FROM script.source_snapshot_id
              OR (
                  binding.role = 'cg'
                  AND NEW.media_kind <> 'image'
              )
              OR (
                  binding.role IN ('bgm', 'sfx')
                  AND NEW.media_kind <> 'audio'
              )
              OR NEW.status <> 'completed'
              OR NEW.file_path IS NULL
              OR (
                  NEW.source_snapshot_chapter_id IS NOT NULL
                  AND NEW.source_snapshot_chapter_id IS DISTINCT FROM scene.source_snapshot_chapter_id
              )
              OR (
                  NEW.source_chapter_id IS NOT NULL
                  AND chapter.source_chapter_id IS DISTINCT FROM NEW.source_chapter_id
              )
          )
    ) THEN
        RAISE EXCEPTION '媒体资产更新会破坏已有 Galgame 行媒体绑定';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM media_provenance provenance
        LEFT JOIN generation_spec spec ON spec.id = provenance.generation_spec_id
        WHERE provenance.media_asset_id = NEW.id
          AND provenance.generation_spec_id IS NOT NULL
          AND (
              spec.id IS NULL
              OR spec.project_id IS DISTINCT FROM NEW.project_id
              OR spec.root_project_id IS DISTINCT FROM NEW.root_project_id
              OR NOT (
                  (NEW.source_snapshot_id IS NULL
                   AND spec.source_snapshot_id IS NULL
                   AND spec.source_snapshot_chapter_id IS NULL)
                  OR spec.source_snapshot_id = NEW.source_snapshot_id
              )
              OR (
                  NEW.source_snapshot_chapter_id IS NOT NULL
                  AND spec.source_snapshot_chapter_id IS DISTINCT FROM NEW.source_snapshot_chapter_id
              )
              OR (
                  NEW.source_chapter_id IS NOT NULL
                  AND spec.source_chapter_id IS DISTINCT FROM NEW.source_chapter_id
              )
          )
    ) THEN
        RAISE EXCEPTION '媒体资产更新会破坏已有 generation_spec 媒体来源';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM media_provenance provenance
        LEFT JOIN prompt_artifact prompt ON prompt.id = provenance.prompt_artifact_id
        LEFT JOIN generation_spec spec ON spec.id = prompt.generation_spec_id
        WHERE provenance.media_asset_id = NEW.id
          AND provenance.prompt_artifact_id IS NOT NULL
          AND (
              prompt.id IS NULL
              OR spec.id IS NULL
              OR spec.project_id IS DISTINCT FROM NEW.project_id
              OR spec.root_project_id IS DISTINCT FROM NEW.root_project_id
              OR (
                  provenance.generation_spec_id IS NOT NULL
                  AND spec.id IS DISTINCT FROM provenance.generation_spec_id
              )
              OR NOT (
                  (NEW.source_snapshot_id IS NULL
                   AND spec.source_snapshot_id IS NULL
                   AND spec.source_snapshot_chapter_id IS NULL)
                  OR spec.source_snapshot_id = NEW.source_snapshot_id
              )
              OR (
                  NEW.source_snapshot_chapter_id IS NOT NULL
                  AND spec.source_snapshot_chapter_id IS DISTINCT FROM NEW.source_snapshot_chapter_id
              )
              OR (
                  NEW.source_chapter_id IS NOT NULL
                  AND spec.source_chapter_id IS DISTINCT FROM NEW.source_chapter_id
              )
          )
    ) THEN
        RAISE EXCEPTION '媒体资产更新会破坏已有 prompt_artifact 媒体来源';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM media_provenance provenance
        LEFT JOIN media_asset source_asset ON source_asset.id = provenance.source_media_asset_id
        WHERE provenance.media_asset_id = NEW.id
          AND provenance.source_media_asset_id IS NOT NULL
          AND (
              source_asset.id IS NULL
              OR source_asset.project_id IS DISTINCT FROM NEW.project_id
              OR source_asset.root_project_id IS DISTINCT FROM NEW.root_project_id
              OR source_asset.status <> 'completed'
              OR source_asset.file_path IS NULL
              OR NOT (
                  (NEW.source_snapshot_id IS NULL
                   AND source_asset.source_snapshot_id IS NULL
                   AND source_asset.source_snapshot_chapter_id IS NULL)
                  OR source_asset.source_snapshot_id = NEW.source_snapshot_id
              )
              OR (
                  NEW.source_snapshot_chapter_id IS NOT NULL
                  AND source_asset.source_snapshot_chapter_id IS DISTINCT FROM NEW.source_snapshot_chapter_id
              )
              OR (
                  NEW.source_chapter_id IS NOT NULL
                  AND source_asset.source_chapter_id IS DISTINCT FROM NEW.source_chapter_id
              )
          )
    ) THEN
        RAISE EXCEPTION '媒体资产更新会破坏已有 source_media_asset 媒体来源';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM media_provenance provenance
        LEFT JOIN media_asset derived_asset ON derived_asset.id = provenance.media_asset_id
        WHERE provenance.source_media_asset_id = NEW.id
          AND (
              derived_asset.id IS NULL
              OR NEW.project_id IS DISTINCT FROM derived_asset.project_id
              OR NEW.root_project_id IS DISTINCT FROM derived_asset.root_project_id
              OR NEW.status <> 'completed'
              OR NEW.file_path IS NULL
              OR NOT (
                  (derived_asset.source_snapshot_id IS NULL
                   AND NEW.source_snapshot_id IS NULL
                   AND NEW.source_snapshot_chapter_id IS NULL)
                  OR NEW.source_snapshot_id = derived_asset.source_snapshot_id
              )
              OR (
                  derived_asset.source_snapshot_chapter_id IS NOT NULL
                  AND NEW.source_snapshot_chapter_id IS DISTINCT FROM derived_asset.source_snapshot_chapter_id
              )
              OR (
                  derived_asset.source_chapter_id IS NOT NULL
                  AND NEW.source_chapter_id IS DISTINCT FROM derived_asset.source_chapter_id
              )
          )
    ) THEN
        RAISE EXCEPTION '媒体资产更新会破坏引用它的媒体来源';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM media_provenance provenance
        LEFT JOIN galgame_line line_row
          ON line_row.id = CASE
              WHEN jsonb_typeof(provenance.request_snapshot_json::jsonb -> 'target_line_id') = 'number'
                   AND (provenance.request_snapshot_json::jsonb ->> 'target_line_id') ~ '^[0-9]+$'
              THEN (provenance.request_snapshot_json::jsonb ->> 'target_line_id')::BIGINT
              ELSE NULL
          END
        LEFT JOIN galgame_scene scene ON scene.id = line_row.scene_id
        LEFT JOIN galgame_script script ON script.id = scene.script_id
        WHERE provenance.media_asset_id = NEW.id
          AND provenance.request_snapshot_json IS NOT NULL
          AND provenance.request_snapshot_json::jsonb ? 'target_line_id'
          AND jsonb_typeof(provenance.request_snapshot_json::jsonb -> 'target_line_id') = 'number'
          AND (provenance.request_snapshot_json::jsonb ->> 'target_line_id') ~ '^[0-9]+$'
          AND (
              line_row.id IS NULL
              OR scene.id IS NULL
              OR script.id IS NULL
              OR NEW.project_id IS DISTINCT FROM script.project_id
              OR NEW.root_project_id IS DISTINCT FROM script.root_project_id
              OR NEW.source_snapshot_id IS DISTINCT FROM script.source_snapshot_id
          )
    ) THEN
        RAISE EXCEPTION '媒体资产更新会破坏已有 Galgame 音频导入媒体来源';
    END IF;

    RETURN NEW;
END;
$$
"#;

pub(crate) const GENERATION_SPEC_MEDIA_REF_STRICT_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_media_ref_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.role NOT IN (
        'first_frame',
        'end_frame',
        'character_reference',
        'object_reference',
        'scene_reference',
        'style_reference',
        'reference'
    ) THEN
        RAISE EXCEPTION '生成意图媒体引用 role 无效';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM generation_spec spec
        JOIN media_asset asset ON asset.id = NEW.media_asset_id
        WHERE spec.id = NEW.generation_spec_id
          AND asset.project_id = spec.project_id
          AND asset.root_project_id = spec.root_project_id
          AND asset.status = 'completed'
          AND asset.file_path IS NOT NULL
    ) THEN
        RAISE EXCEPTION '生成意图媒体引用必须使用同项目族已完成媒体资产';
    END IF;
    RETURN NEW;
END;
$$
"#;
