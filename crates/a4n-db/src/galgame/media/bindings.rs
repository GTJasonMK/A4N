mod completed;
mod mutation;
mod read_model;
mod rules;

pub use completed::completed_galgame_visual_binding_records;
pub use mutation::{bind_galgame_media_records, unbind_galgame_media_records};
pub use read_model::galgame_media_binding_read_model;
pub(in crate::galgame) use rules::{
    validate_galgame_media_asset_id, validate_galgame_media_target_ids,
};
