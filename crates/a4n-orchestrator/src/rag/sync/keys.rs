use a4n_core::rag::RagDataType;
use a4n_rag_core::vector_store::DeleteFilter;

use super::contracts::RagSyncError;

pub fn character_state_rag_chunk_id(project_id: i64, state_id: i64) -> String {
    format!("character_state:{project_id}:{state_id}")
}

pub fn chapter_rag_chunk_prefix(project_id: i64, chapter_id: i64) -> String {
    format!("chapter:{project_id}:{chapter_id}:")
}

pub fn character_attribute_event_rag_chunk_id(project_id: i64, event_id: i64) -> String {
    format!("character_attribute_event:{project_id}:{event_id}")
}

pub fn foreshadowing_rag_chunk_id(project_id: i64, foreshadowing_id: i64) -> String {
    format!("foreshadowing:{project_id}:{foreshadowing_id}")
}

pub fn protagonist_rag_chunk_prefix(project_id: i64, profile_id: i64) -> String {
    format!("protagonist:{project_id}:{profile_id}:")
}

pub fn protagonist_rag_chunk_id(project_id: i64, profile_id: i64, dimension: &str) -> String {
    format!(
        "{}{}",
        protagonist_rag_chunk_prefix(project_id, profile_id),
        dimension
    )
}

pub(super) fn chapter_rag_delete_filter(
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
) -> Result<DeleteFilter, RagSyncError> {
    if chapter_number <= 0 {
        return Err(RagSyncError::InvalidDeleteInput {
            message: format!("RAG 章节清理 chapter_number 必须大于 0，实际为 {chapter_number}"),
        });
    }
    let chapter_number =
        u32::try_from(chapter_number).map_err(|_| RagSyncError::InvalidDeleteInput {
            message: "RAG 章节清理 chapter_number 超出 u32 范围".to_string(),
        })?;
    Ok(DeleteFilter {
        project_id,
        data_type: None,
        chapter_number: Some(chapter_number),
        ids: Vec::new(),
        id_prefixes: vec![chapter_rag_chunk_prefix(project_id, chapter_id)],
    })
}

pub(super) fn rag_id_prefix(
    project_id: i64,
    data_type: RagDataType,
) -> Result<String, RagSyncError> {
    match data_type {
        RagDataType::Chapter
        | RagDataType::ChapterSummary
        | RagDataType::ChapterMetadata
        | RagDataType::KeyEvent => Err(RagSyncError::InvalidRecord {
            message: format!(
                "{} 需要按具体章节清理，不能按共享章节前缀清理",
                data_type.as_str()
            ),
        }),
        RagDataType::Foreshadowing => Ok(format!("foreshadowing:{project_id}:")),
        RagDataType::Outline => Ok(format!("outline:{project_id}:")),
        RagDataType::PartOutline => Ok(format!("part_outline:{project_id}:")),
        RagDataType::Blueprint => Ok(format!("blueprint:{project_id}:")),
        RagDataType::BlueprintMetadata => Ok(format!("blueprint_metadata:{project_id}:")),
        RagDataType::WorldSetting => Ok(format!("world_setting:{project_id}:")),
        RagDataType::Character => Ok(format!("character:{project_id}:")),
        RagDataType::Protagonist => Ok(format!("protagonist:{project_id}:")),
        RagDataType::CharacterState => Ok(format!("character_state:{project_id}:")),
        RagDataType::CharacterAttributeEvent => {
            Ok(format!("character_attribute_event:{project_id}:"))
        }
        RagDataType::Relationship => Ok(format!("relationship:{project_id}:")),
        RagDataType::SceneProfile => Ok(format!("scene_profile:{project_id}:")),
        RagDataType::SceneOccurrence => Ok(format!("scene_occurrence:{project_id}:")),
        RagDataType::SceneTransition => Ok(format!("scene_transition:{project_id}:")),
    }
}
