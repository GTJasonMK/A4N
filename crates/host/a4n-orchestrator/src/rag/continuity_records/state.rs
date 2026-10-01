use a4n_db::rag::RagCharacterStateRow;
use a4n_db_models::models::character::character_state_index;
use a4n_domain::rag::RagDataType;
use a4n_rag_core::RagRecord;
use a4n_rag_core::record_text::{optional_json_text, optional_rag_text, required_rag_text};

use crate::rag::character_state_rag_chunk_id;

pub(super) fn character_state_rag_text(
    state_index: &character_state_index::Model,
    name: &str,
) -> Result<String, String> {
    character_state_rag_text_fields(
        state_index.id,
        state_index.chapter_number,
        &state_index.state_summary,
        state_index.emotional_state.as_deref(),
        state_index.current_location.as_deref(),
        state_index.current_goal.as_deref(),
        &state_index.key_attributes,
        name,
    )
}

fn character_state_rag_text_fields(
    id: i64,
    chapter_number: i32,
    state_summary: &str,
    emotional_state: Option<&str>,
    current_location: Option<&str>,
    current_goal: Option<&str>,
    key_attributes: &serde_json::Value,
    name: &str,
) -> Result<String, String> {
    let name = required_rag_text(
        Some(name),
        format!("角色状态 {id} 缺少角色名，跳过 RAG 入库"),
    )?;
    let state_summary = required_rag_text(
        Some(state_summary),
        format!("角色状态 {id} 缺少状态摘要，跳过 RAG 入库"),
    )?;
    let mut lines = vec![
        format!("角色: {name}"),
        format!("章节: 第{chapter_number}章"),
        format!("状态: {state_summary}"),
    ];
    if let Some(emotional_state) = optional_rag_text(emotional_state) {
        lines.push(format!("情绪: {emotional_state}"));
    }
    if let Some(location) = optional_rag_text(current_location) {
        lines.push(format!("位置: {location}"));
    }
    if let Some(goal) = optional_rag_text(current_goal) {
        lines.push(format!("目标: {goal}"));
    }
    if let Some(attributes) = optional_json_text(Some(key_attributes)) {
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
    character_state_rag_record_fields(
        project_id,
        state_index.id,
        state_index.chapter_number,
        name,
        text,
    )
}

pub fn character_state_rag_record_from_row(
    project_id: i64,
    state_index: &RagCharacterStateRow,
    name: &str,
) -> Result<RagRecord, String> {
    let name = required_rag_text(
        Some(name),
        format!("角色状态 {} 缺少角色名，跳过 RAG 入库", state_index.id),
    )?;
    let text = character_state_rag_text_fields(
        state_index.id,
        state_index.chapter_number,
        &state_index.state_summary,
        state_index.emotional_state.as_deref(),
        state_index.current_location.as_deref(),
        state_index.current_goal.as_deref(),
        &state_index.key_attributes,
        name,
    )?;
    character_state_rag_record_fields(
        project_id,
        state_index.id,
        state_index.chapter_number,
        name,
        text,
    )
}

fn character_state_rag_record_fields(
    project_id: i64,
    id: i64,
    chapter_number: i32,
    name: &str,
    text: String,
) -> Result<RagRecord, String> {
    let title = format!("{name} 第{chapter_number}章后状态");
    Ok(RagRecord {
        chunk_id: character_state_rag_chunk_id(project_id, id),
        text,
        metadata: serde_json::json!({
            "data_type": RagDataType::CharacterState.as_str(),
            "chapter_number": chapter_number,
            "title": title,
        }),
    })
}
