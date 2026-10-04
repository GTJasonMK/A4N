use a4n_db::rag::RagAttributeEventRow;
use a4n_domain::character::CharacterAttributeEventRecord;
use a4n_domain::rag::RagDataType;
use a4n_rag_core::RagRecord;
use a4n_rag_core::record_text::{optional_json_text, optional_rag_text, required_rag_text};

use crate::rag::character_attribute_event_rag_chunk_id;

pub(super) fn character_attribute_event_rag_text(
    event: &RagAttributeEventRow,
    name: &str,
) -> Result<String, String> {
    let name = required_rag_text(
        Some(name),
        format!("角色属性事件 {} 缺少角色名，跳过 RAG 入库", event.id),
    )?;
    let category = required_rag_text(
        Some(&event.category),
        format!("角色属性事件 {} 缺少属性类别，跳过 RAG 入库", event.id),
    )?;
    let key = required_rag_text(
        Some(&event.key),
        format!("角色属性事件 {} 缺少属性键，跳过 RAG 入库", event.id),
    )?;
    let operation = required_rag_text(
        Some(&event.operation),
        format!("角色属性事件 {} 缺少操作类型，跳过 RAG 入库", event.id),
    )?;

    let mut lines = vec![
        format!("角色: {name}"),
        format!("章节: 第{}章", event.chapter_number),
        format!("类别: {}", attribute_category_label(category)),
        format!("属性: {key}"),
        format!("操作: {}", attribute_operation_label(operation)),
    ];
    if let Some(old_value) = optional_json_text(Some(&event.old_value)) {
        lines.push(format!("原值: {old_value}"));
    }
    if let Some(new_value) = optional_json_text(Some(&event.new_value)) {
        lines.push(format!("新值: {new_value}"));
    }
    if let Some(description) = optional_rag_text(Some(&event.change_description)) {
        lines.push(format!("摘要: {description}"));
    }
    if let Some(cause) = optional_rag_text(Some(&event.cause)) {
        lines.push(format!("原因: {cause}"));
    }
    if let Some(evidence) = optional_rag_text(Some(&event.evidence)) {
        lines.push(format!("证据: {evidence}"));
    }
    Ok(lines.join("\n"))
}

pub fn character_attribute_event_rag_record(
    project_id: i64,
    event: &CharacterAttributeEventRecord,
    name: &str,
) -> Result<RagRecord, String> {
    character_attribute_event_rag_record_from_row(
        project_id,
        &RagAttributeEventRow::from(event),
        name,
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
    let text = character_attribute_event_rag_text(event, name)?;
    let title = format!("{name} 第{}章属性变更", event.chapter_number);
    Ok(RagRecord {
        chunk_id: character_attribute_event_rag_chunk_id(project_id, event.id),
        text,
        metadata: serde_json::json!({
            "data_type": RagDataType::CharacterAttributeEvent.as_str(),
            "chapter_number": event.chapter_number,
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
