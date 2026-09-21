mod entity;
mod galgame;
mod source;

pub(super) use entity::validate_generation_spec_target_entity;
pub(super) use galgame::validate_generation_spec_galgame_scene_context;
pub(super) use source::{
    required_generation_spec_source_snapshot_id, validate_generation_spec_source_chapter,
};
