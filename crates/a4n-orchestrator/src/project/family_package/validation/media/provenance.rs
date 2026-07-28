mod context;
mod generation;
mod prompt;
mod request;
mod source_asset;

use a4n_core::models::media::{media_asset, media_provenance};

use super::contracts::MediaProvenanceRefContext;
use generation::validate_generation_spec_ref;
use prompt::validate_prompt_artifact_ref;
use request::validate_request_snapshot;
use source_asset::validate_source_media_asset_ref;

pub(super) fn validate_media_provenance_refs(
    provenance: &media_provenance::Model,
    asset: &media_asset::Model,
    refs: &MediaProvenanceRefContext<'_>,
) -> Result<(), String> {
    let linked_spec = validate_generation_spec_ref(provenance, asset, refs)?;
    validate_prompt_artifact_ref(provenance, asset, linked_spec, refs)?;
    validate_source_media_asset_ref(provenance, asset, refs)?;
    validate_request_snapshot(provenance, asset, linked_spec, refs)
}
