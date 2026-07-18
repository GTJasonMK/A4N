use a4n_core::{models::character::character_state_index, rag::RagDataType};
use a4n_rag_core::record_text::{optional_json_text, optional_rag_text, required_rag_text};

use crate::rag::{RagRecord, character_state_rag_chunk_id};

pub(super) fn character_state_rag_text(
    state_index: &character_state_index::Model,
    name: &str,
) -> Result<String, String> {
    let name = required_rag_text(
        Some(name),
        format!("角色状态 {} 缺少角色名，跳过 RAG 入库", state_index.id),
    )?;
    let state_summary = required_rag_text(
        Some(&state_index.state_summary),
        format!("角色状态 {} 缺少状态摘要，跳过 RAG 入库", state_index.id),
    )?;
    let mut lines = vec![
        format!("角色: {name}"),
        format!("章节: 第{}章", state_index.chapter_number),
        format!("状态: {state_summary}"),
    ];
    if let Some(emotional_state) = optional_rag_text(state_index.emotional_state.as_deref()) {
        lines.push(format!("情绪: {emotional_state}"));
    }
    if let Some(location) = optional_rag_text(state_index.current_location.as_deref()) {
        lines.push(format!("位置: {location}"));
    }
    if let Some(goal) = optional_rag_text(state_index.current_goal.as_deref()) {
        lines.push(format!("目标: {goal}"));
    }
    if let Some(attributes) = optional_json_text(Some(&state_index.key_attributes)) {
        lines.push(format!("关键属性: {attributes}"));
    }
    Ok(lines.join("\n"))
}

pub fn character_state_rag_record(
    project_id: i64,
    state_index: &character_state_index::Model,
    name: &str,
) -> Result<RagRecord, String> {
    let name = required_rag_text(
        Some(name),
        format!("角色状态 {} 缺少角色名，跳过 RAG 入库", state_index.id),
    )?;
    let text = character_state_rag_text(state_index, name)?;
    let title = format!("{name} 第{}章后状态", state_index.chapter_number);
    Ok(RagRecord {
        chunk_id: character_state_rag_chunk_id(project_id, state_index.id),
        text,
        metadata: serde_json::json!({
            "data_type": RagDataType::CharacterState.as_str(),
            "state_id": state_index.id,
            "profile_id": state_index.profile_id,
            "character_name": name,
            "chapter_number": state_index.chapter_number,
            "title": title,
        }),
    })
}
