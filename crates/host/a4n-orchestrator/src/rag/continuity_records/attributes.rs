use a4n_db::rag::RagAttributeEventRow;
use a4n_db_models::models::character::attribute_event;
use a4n_domain::rag::RagDataType;
use a4n_rag_core::RagRecord;
use a4n_rag_core::record_text::{optional_json_text, optional_rag_text, required_rag_text};

use crate::rag::character_attribute_event_rag_chunk_id;

pub(super) fn character_attribute_event_rag_text(
    event: &attribute_event::Model,
    name: &str,
) -> Result<String, String> {
    character_attribute_event_rag_text_fields(
        event.id,
        event.chapter_number,
        &event.category,
        &event.key,
        &event.operation,
        &event.old_value,
        &event.new_value,
        &event.change_description,
        &event.cause,
        &event.evidence,
        name,
    )
}

fn character_attribute_event_rag_text_fields(
    id: i64,
    chapter_number: i32,
    category: &str,
    key: &str,
    operation: &str,
    old_value: &serde_json::Value,
    new_value: &serde_json::Value,
    change_description: &str,
    cause: &str,
    evidence: &str,
    name: &str,
) -> Result<String, String> {
    let name = required_rag_text(
        Some(name),
        format!("角色属性事件 {id} 缺少角色名，跳过 RAG 入库"),
    )?;
    let category = required_rag_text(
        Some(category),
        format!("角色属性事件 {id} 缺少属性类别，跳过 RAG 入库"),
    )?;
    let key = required_rag_text(
        Some(key),
        format!("角色属性事件 {id} 缺少属性键，跳过 RAG 入库"),
    )?;
    let operation = required_rag_text(
        Some(operation),
        format!("角色属性事件 {id} 缺少操作类型，跳过 RAG 入库"),
    )?;

    let mut lines = vec![
        format!("角色: {name}"),
        format!("章节: 第{chapter_number}章"),
        format!("类别: {}", attribute_category_label(category)),
        format!("属性: {key}"),
        format!("操作: {}", attribute_operation_label(operation)),
    ];
    if let Some(old_value) = optional_json_text(Some(old_value)) {
        lines.push(format!("原值: {old_value}"));
    }
    if let Some(new_value) = optional_json_text(Some(new_value)) {
        lines.push(format!("新值: {new_value}"));
    }
    if let Some(description) = optional_rag_text(Some(change_description)) {
        lines.push(format!("摘要: {description}"));
    }
    if let Some(cause) = optional_rag_text(Some(cause)) {
        lines.push(format!("原因: {cause}"));
    }
    if let Some(evidence) = optional_rag_text(Some(evidence)) {
        lines.push(format!("证据: {evidence}"));
    }
    Ok(lines.join("\n"))
}

pub fn character_attribute_event_rag_record(
    project_id: i64,
    event: &attribute_event::Model,
    name: &str,
) -> Result<RagRecord, String> {
    let name = required_rag_text(
        Some(name),
        format!("角色属性事件 {} 缺少角色名，跳过 RAG 入库", event.id),
    )?;
    let text = character_attribute_event_rag_text(event, name)?;
    character_attribute_event_rag_record_fields(
        project_id,
        event.id,
        event.chapter_number,
        name,
        text,
    )
}

pub fn character_attribute_event_rag_record_from_row(
    project_id: i64,
    event: &RagAttributeEventRow,
    name: &str,
) -> Result<RagRecord, String> {
    let name = required_rag_text(
        Some(name),
        format!("角色属性事件 {} 缺少角色名，跳过 RAG 入库", event.id),
    )?;
    let text = character_attribute_event_rag_text_fields(
        event.id,
        event.chapter_number,
        &event.category,
        &event.key,
        &event.operation,
        &event.old_value,
        &event.new_value,
        &event.change_description,
        &event.cause,
        &event.evidence,
        name,
    )?;
    character_attribute_event_rag_record_fields(
        project_id,
        event.id,
        event.chapter_number,
        name,
        text,
    )
}

fn character_attribute_event_rag_record_fields(
    project_id: i64,
    id: i64,
    chapter_number: i32,
    name: &str,
    text: String,
) -> Result<RagRecord, String> {
    let title = format!("{name} 第{chapter_number}章属性变更");
    Ok(RagRecord {
        chunk_id: character_attribute_event_rag_chunk_id(project_id, id),
        text,
        metadata: serde_json::json!({
            "data_type": RagDataType::CharacterAttributeEvent.as_str(),
            "chapter_number": chapter_number,
            "title": title,
        }),
    })
}

fn attribute_category_label(category: &str) -> &'static str {
    match category {
        "explicit" => "显性属性",
        "implicit" => "隐性属性",
        "social" => "社会属性",
        _ => "属性",
    }
}

fn attribute_operation_label(operation: &str) -> &'static str {
    match operation {
        "add" => "新增",
        "modify" => "修改",
        "delete" => "删除",
        _ => "变更",
    }
}
