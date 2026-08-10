mod embedding;
mod guide;
mod image;
mod llm;
mod shared;
mod video;

pub(in crate::routes::commands) use embedding::*;
pub(in crate::routes::commands) use guide::*;
pub(in crate::routes::commands) use image::*;
pub(in crate::routes::commands) use llm::*;
pub(in crate::routes::commands) use video::*;

#[cfg(test)]
mod tests;
