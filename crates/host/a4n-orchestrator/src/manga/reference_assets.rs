mod bindings;
mod discovery;
mod metadata;

pub(in crate::manga) use bindings::page_reference_bindings;
pub(in crate::manga) use discovery::{
    manga_page_reference_discovery_candidates, parse_manga_page_reference_discovery,
    validate_manga_page_reference_discovery_candidates,
};
pub(in crate::manga) use metadata::manga_reference_assets_json;
