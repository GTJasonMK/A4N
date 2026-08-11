mod bindings;
mod contracts;
mod metadata;

pub(in crate::manga) use bindings::page_reference_bindings;
pub use contracts::{MangaCharacterPortraitRef, MangaReferenceContext};
pub(in crate::manga) use metadata::manga_reference_assets_json;
