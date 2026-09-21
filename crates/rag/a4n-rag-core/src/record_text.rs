mod json;
mod rules;
mod sections;

pub use json::format_json_for_rag;
pub use rules::{optional_json_text, optional_rag_text, required_rag_text, short_rag_title};
pub use sections::{DimensionRagSection, dimension_fields_section, dimension_text_section};

#[cfg(test)]
mod tests;
