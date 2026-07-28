use super::super::super::super::ProjectFamilyPackage;
use super::super::super::common::validate_unique_key_strings;

pub(super) fn validate_generation_ref_keys(package: &ProjectFamilyPackage) -> Result<(), String> {
    validate_unique_key_strings(
        package.generation_spec_text_anchors.iter().map(|record| {
            format!(
                "{}/{}/{}",
                record.generation_spec_id, record.text_anchor_id, record.role
            )
        }),
        "generation_spec_text_anchors(generation_spec_id,text_anchor_id,role)",
    )?;
    validate_unique_key_strings(
        package.generation_spec_entity_refs.iter().map(|record| {
            format!(
                "{}/{}/{}/{}",
                record.generation_spec_id, record.entity_kind, record.entity_id, record.role
            )
        }),
        "generation_spec_entity_refs(generation_spec_id,entity_kind,entity_id,role)",
    )?;
    validate_unique_key_strings(
        package.generation_spec_media_refs.iter().map(|record| {
            format!(
                "{}/{}/{}",
                record.generation_spec_id, record.media_asset_id, record.role
            )
        }),
        "generation_spec_media_refs(generation_spec_id,media_asset_id,role)",
    )
}
