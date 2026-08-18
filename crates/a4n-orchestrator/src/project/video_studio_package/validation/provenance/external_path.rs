use std::collections::HashSet;

use a4n_core::models::media::media_provenance;
use a4n_domain::{GeneratedMediaReference, parse_generated_media_reference};

pub(in crate::project::video_studio_package::validation) fn validate_media_provenance_source_external_path(
    provenance: &media_provenance::Model,
    image_ids: &HashSet<i64>,
    video_ids: &HashSet<i64>,
) -> Result<(), String> {
    let Some(raw) = provenance.source_external_path.as_deref() else {
        return Ok(());
    };
    let Some(reference) = parse_generated_media_reference(raw).map_err(|error| {
        format!(
            "媒体来源 source_external_path 的 {} ID 无效: media_asset_id={}, source_external_path={raw}",
            error.kind(),
            provenance.media_asset_id
        )
    })? else {
        return Ok(());
    };
    let ids = match reference {
        GeneratedMediaReference::Video(_) => video_ids,
        GeneratedMediaReference::Image(_) => image_ids,
    };
    if !ids.contains(&reference.id()) {
        return Err(format!(
            "媒体来源 source_external_path 引用了不存在的{}: media_asset_id={}, {}_id={}",
            reference.label(),
            provenance.media_asset_id,
            reference.kind(),
            reference.id()
        ));
    }
    Ok(())
}
