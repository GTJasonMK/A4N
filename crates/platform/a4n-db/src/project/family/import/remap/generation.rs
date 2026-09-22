mod galgame;
mod manga;
mod params;
mod target;

pub(in crate::project::family::import) use manga::{
    remap_manga_page_layout, remap_manga_reference_bindings_value,
};
pub(in crate::project::family::import) use params::remap_generation_spec_params_json;
pub(in crate::project::family::import) use target::{
    remap_generation_spec_source_version_hash, remap_generation_target_id,
};
