mod embedding;
mod image;
mod llm;
mod settings_all;
mod shared;
mod video;

pub(in crate::routes::commands) use embedding::*;
pub(in crate::routes::commands) use image::*;
pub(in crate::routes::commands) use llm::*;
pub(in crate::routes::commands) use settings_all::*;
pub(in crate::routes::commands) use video::*;

#[cfg(test)]
mod tests;
