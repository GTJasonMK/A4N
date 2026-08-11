pub mod contracts;
pub mod loading;
pub mod projections;

pub use contracts::{ChapterOutlineGenerationContext, PartOutlineGenerationContext};
pub(in crate::outline) use loading::lock_outline_generation_sources;
pub use loading::{load_chapter_outline_generation_context, load_part_generation_context};
pub use projections::part_context_json;
pub(crate) use projections::{chapter_outline_context_json, outline_formal_materials_context_json};
pub(in crate::outline) use projections::{
    focused_part_context, minimal_blueprint_context, minimal_part_context,
};

#[cfg(test)]
pub(in crate::outline) use projections::{
    has_outline_generation_source, outline_blueprint_context_json,
};
