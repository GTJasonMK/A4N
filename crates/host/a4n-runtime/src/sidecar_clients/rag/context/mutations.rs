use super::scope::rag_operation_timestamp;

pub(crate) fn chapter_rag_cleanup_operation_trace_id(project_id: i64) -> Result<String, String> {
    if project_id <= 0 {
        return Err(format!(
            "Chapter RAG cleanup project_id 必须是正整数，实际为 {project_id}"
        ));
    }
    Ok(format!(
        "chapter_rag_cleanup:{project_id}:{}",
        rag_operation_timestamp()?
    ))
}

pub(crate) fn character_profile_delete_rag_operation_trace_id(
    project_id: i64,
    profile_id: i64,
) -> Result<String, String> {
    validate_character_rag_mutation_scope(project_id, profile_id)?;
    Ok(format!(
        "character_profile_delete:{project_id}:{profile_id}:{}",
        rag_operation_timestamp()?
    ))
}

pub(crate) fn character_profile_rollback_rag_operation_trace_id(
    project_id: i64,
    profile_id: i64,
    chapter_number: i32,
) -> Result<String, String> {
    validate_character_rag_mutation_scope(project_id, profile_id)?;
    if chapter_number < 0 {
        return Err(format!(
            "Character profile rollback chapter_number 不能小于 0，实际为 {chapter_number}"
        ));
    }
    Ok(format!(
        "character_profile_rollback:{project_id}:{profile_id}:{chapter_number}:{}",
        rag_operation_timestamp()?
    ))
}

fn validate_character_rag_mutation_scope(project_id: i64, profile_id: i64) -> Result<(), String> {
    if project_id <= 0 {
        return Err(format!(
            "Character RAG mutation project_id 必须是正整数，实际为 {project_id}"
        ));
    }
    if profile_id <= 0 {
        return Err(format!(
            "Character RAG mutation profile_id 必须是正整数，实际为 {profile_id}"
        ));
    }
    Ok(())
}
