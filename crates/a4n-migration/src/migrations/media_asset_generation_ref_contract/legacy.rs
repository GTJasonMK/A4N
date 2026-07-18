pub(super) const MEDIA_ASSET_FAMILY_LEGACY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_media_asset_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    project_kind TEXT;
    project_source_mode TEXT;
    project_source_snapshot_id BIGINT;
BEGIN
    SELECT project_row.project_kind,
           project_row.source_mode,
           project_row.source_snapshot_id
      INTO project_kind,
           project_source_mode,
           project_source_snapshot_id
    FROM novel_project project_row
    WHERE project_row.id = NEW.project_id;

    IF project_kind IS NULL THEN
        RAISE EXCEPTION '媒体资产 project_id 不存在';
    END IF;

    IF project_source_mode = 'freeform' THEN
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
              AND root.source_mode = 'self'
        ) THEN
            RAISE EXCEPTION '媒体资产 root_project_id 必须指向文本项目';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM novel_project project_row
            WHERE project_row.id = NEW.project_id
              AND (
                  (project_row.project_kind = 'text' AND project_row.id = NEW.root_project_id)
                  OR (
                      project_row.project_kind <> 'text'
                      AND project_row.source_mode = 'derived'
                      AND project_row.root_project_id = NEW.root_project_id
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
        IF NEW.source_chapter_id IS NOT NULL AND NEW.source_snapshot_chapter_id IS NULL THEN
            RAISE EXCEPTION '设置 source_chapter_id 时必须同时设置 source_snapshot_chapter_id';
        END IF;
        IF NEW.source_snapshot_chapter_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM source_snapshot_chapter chapter
            WHERE chapter.id = NEW.source_snapshot_chapter_id
              AND chapter.root_project_id = NEW.root_project_id
              AND chapter.snapshot_id = NEW.source_snapshot_id
              AND (NEW.source_chapter_id IS NULL OR chapter.source_chapter_id = NEW.source_chapter_id)
        ) THEN
            RAISE EXCEPTION '媒体资产 source_snapshot_chapter_id 必须属于同一源快照章节';
        END IF;
    END IF;

    IF NEW.status = 'completed' AND NEW.file_path IS NULL THEN
        RAISE EXCEPTION 'completed 媒体资产必须有 file_path';
    END IF;
    RETURN NEW;
END;
$$
"#;

pub(super) const GENERATION_SPEC_MEDIA_REF_LEGACY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_media_ref_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM generation_spec spec
        JOIN media_asset asset ON asset.id = NEW.media_asset_id
        WHERE spec.id = NEW.generation_spec_id
          AND asset.project_id = spec.project_id
          AND asset.root_project_id = spec.root_project_id
    ) THEN
        RAISE EXCEPTION '生成意图媒体引用必须使用同项目族媒体资产';
    END IF;
    RETURN NEW;
END;
$$
"#;
