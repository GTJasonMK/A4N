pub(super) const MEDIA_PROVENANCE_FAMILY_LEGACY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_media_provenance_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.generation_spec_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM media_asset asset
        JOIN generation_spec spec ON spec.id = NEW.generation_spec_id
        WHERE asset.id = NEW.media_asset_id
          AND spec.project_id = asset.project_id
          AND spec.root_project_id = asset.root_project_id
    ) THEN
        RAISE EXCEPTION '媒体来源 generation_spec_id 必须属于媒体资产项目族';
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
    ) THEN
        RAISE EXCEPTION '媒体来源 prompt_artifact_id 必须属于媒体资产项目族';
    END IF;
    IF NEW.source_media_asset_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM media_asset asset
        JOIN media_asset source ON source.id = NEW.source_media_asset_id
        WHERE asset.id = NEW.media_asset_id
          AND source.project_id = asset.project_id
          AND source.root_project_id = asset.root_project_id
    ) THEN
        RAISE EXCEPTION '媒体来源 source_media_asset_id 必须属于同一项目族';
    END IF;
    RETURN NEW;
END;
$$
"#;

pub(super) const MEDIA_PROVENANCE_FAMILY_LEGACY_UPDATE_TRIGGER_SQL: &str = r#"
CREATE TRIGGER trg_media_provenance_family_update
BEFORE UPDATE OF media_asset_id, generation_spec_id, prompt_artifact_id, source_media_asset_id ON media_provenance
FOR EACH ROW EXECUTE FUNCTION a4n_validate_media_provenance_family()
"#;
