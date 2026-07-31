mod embedding;
mod image;
mod llm;
mod shared;
mod video;

pub(in crate::routes::commands) use embedding::*;
pub(in crate::routes::commands) use image::*;
pub(in crate::routes::commands) use llm::*;
pub(super) use shared::require_global_config_admin;
pub(in crate::routes::commands) use video::*;

#[cfg(test)]
mod tests;
