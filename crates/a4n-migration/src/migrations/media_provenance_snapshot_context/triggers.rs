pub(super) const MEDIA_PROVENANCE_FAMILY_DROP_UPDATE_TRIGGER_SQL: &str =
    "DROP TRIGGER IF EXISTS trg_media_provenance_family_update ON media_provenance";

pub(super) const MEDIA_PROVENANCE_FAMILY_UPDATE_TRIGGER_SQL: &str = r#"
CREATE TRIGGER trg_media_provenance_family_update
BEFORE UPDATE OF media_asset_id, generation_spec_id, prompt_artifact_id, source_media_asset_id, request_snapshot_json ON media_provenance
FOR EACH ROW EXECUTE FUNCTION a4n_validate_media_provenance_family()
"#;
