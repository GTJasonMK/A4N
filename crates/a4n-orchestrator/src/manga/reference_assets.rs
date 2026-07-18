mod bindings;
mod contracts;
mod formatting;
mod metadata;

pub(in crate::manga) use bindings::page_reference_bindings;
pub use contracts::{MangaCharacterPortraitRef, MangaReferenceContext};
pub(in crate::manga) use formatting::format_character_portrait_context;
pub(in crate::manga) use metadata::{character_portraits_json, manga_reference_assets_metadata};
