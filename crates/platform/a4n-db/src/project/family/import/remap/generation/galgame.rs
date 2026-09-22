mod background;
mod cg;
mod common;
mod portrait;

pub(super) use background::remap_galgame_background_params;
pub(super) use cg::remap_galgame_cg_params;
pub(super) use portrait::remap_galgame_portrait_params;

#[cfg(test)]
mod tests;
