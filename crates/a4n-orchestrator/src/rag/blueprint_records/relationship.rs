use a4n_core::{models::project::blueprint_relationship, rag::RagDataType};
use a4n_rag_core::record_text::{optional_json_text, required_rag_text};

use super::super::RagRecord;

fn relationship_rag_text(relationship: &blueprint_relationship::Model) -> Result<String, String> {
    let character_a = required_rag_text(
        Some(&relationship.character_a),
        format!("角色关系 {} 缺少角色 A，跳过 RAG 入库", relationship.id),
    )?;
    let character_b = required_rag_text(
        Some(&relationship.character_b),
        format!("角色关系 {} 缺少角色 B，跳过 RAG 入库", relationship.id),
    )?;
    let description = required_rag_text(
        relationship.description.as_deref(),
        format!("角色关系 {} 缺少描述，跳过 RAG 入库", relationship.id),
    )?;
    let relation_type = required_rag_text(
        Some(&relationship.relationship_type),
        format!("角色关系 {} 缺少类型，跳过 RAG 入库", relationship.id),
    )?;
    let mut lines = vec![
        format!("{character_a} -> {character_b}: {description}"),
        format!("类型: {relation_type}"),
    ];
    if let Some(strength) = relationship.strength {
        lines.push(format!("强度: {strength}"));
    }
    if let Some(evolution) = optional_json_text(relationship.evolution.as_ref()) {
        lines.push(format!("关系动态: {evolution}"));
    }
    Ok(lines.join("\n"))
}

pub fn relationship_rag_record(
    project_id: i64,
    relationship: &blueprint_relationship::Model,
) -> Result<RagRecord, String> {
    Ok(RagRecord {
        chunk_id: format!("relationship:{project_id}:{}", relationship.id),
        text: relationship_rag_text(relationship)?,
        metadata: serde_json::json!({
            "data_type": RagDataType::Relationship.as_str(),
            "relationship_id": relationship.id,
            "character_a": relationship.character_a,
            "character_b": relationship.character_b,
        }),
    })
}
