use a4n_core::rag::RagDataType;

pub(crate) fn rag_project_prefix(project_id: i64, data_type: RagDataType) -> String {
    rag_chunk_id(project_id, data_type, "")
}

pub(crate) fn rag_chunk_id(
    project_id: i64,
    data_type: RagDataType,
    suffix: impl std::fmt::Display,
) -> String {
    format!("{}:{project_id}:{suffix}", data_type.as_str())
}

pub(crate) fn character_state_rag_chunk_id(project_id: i64, state_id: i64) -> String {
    rag_chunk_id(project_id, RagDataType::CharacterState, state_id)
}

pub(crate) fn chapter_rag_chunk_prefix(project_id: i64, chapter_id: i64) -> String {
    format!(
        "{}{chapter_id}:",
        rag_project_prefix(project_id, RagDataType::Chapter)
    )
}

pub(crate) fn character_attribute_event_rag_chunk_id(project_id: i64, event_id: i64) -> String {
    rag_chunk_id(project_id, RagDataType::CharacterAttributeEvent, event_id)
}

pub(crate) fn foreshadowing_rag_chunk_id(project_id: i64, foreshadowing_id: i64) -> String {
    rag_chunk_id(project_id, RagDataType::Foreshadowing, foreshadowing_id)
}

pub(crate) fn protagonist_rag_chunk_prefix(project_id: i64, profile_id: i64) -> String {
    format!(
        "{}{profile_id}:",
        rag_project_prefix(project_id, RagDataType::Protagonist)
    )
}

pub(crate) fn protagonist_rag_chunk_id(
    project_id: i64,
    profile_id: i64,
    dimension: &str,
) -> String {
    format!(
        "{}{dimension}",
        protagonist_rag_chunk_prefix(project_id, profile_id)
    )
}
