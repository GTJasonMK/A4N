mod entities;
mod metadata;
mod result;
mod text;

pub(super) use entities::decode_xml_entities;
pub(super) use metadata::extract_document_metadata;
pub(super) use result::parse_extracted_document;
pub(super) use text::extract_xml_text;
