use a4n_core::{models::project::blueprint_character, rag::RagDataType};
use a4n_rag_core::record_text::{
    DimensionRagSection, dimension_fields_section, optional_json_text, required_rag_text,
};

use super::super::RagRecord;

fn blueprint_character_rag_sections(
    character: &blueprint_character::Model,
) -> Result<Vec<DimensionRagSection>, String> {
    let name = required_rag_text(
        Some(&character.name),
        format!("蓝图角色 {} 缺少姓名，跳过 RAG 入库", character.id),
    )?;
    let character_type = required_rag_text(
        Some(&character.character_type),
        format!("蓝图角色 {} 缺少类型，跳过 RAG 入库", character.id),
    )?;
    let arc_stages_text = optional_json_text(character.arc_stages.as_ref());
    let mut sections = Vec::new();
    if let Some(section) = dimension_fields_section(
        "角色设定",
        "identity",
        "身份",
        name,
        &[
            ("类型", Some(character_type)),
            ("性别", character.gender.as_deref()),
            ("年龄", character.age.as_deref()),
        ],
        true,
    ) {
        sections.push(section);
    }
    for (dimension, label, field_label, value) in [
        (
            "appearance",
            "外貌",
            "外貌",
            character.appearance.as_deref(),
        ),
        (
            "personality",
            "性格",
            "性格",
            character.personality.as_deref(),
        ),
        (
            "background",
            "背景",
            "背景",
            character.background.as_deref(),
        ),
        (
            "motivation",
            "动机",
            "动机",
            character.motivation.as_deref(),
        ),
    ] {
        if let Some(section) = dimension_fields_section(
            "角色设定",
            dimension,
            label,
            name,
            &[(field_label, value)],
            false,
        ) {
            sections.push(section);
        }
    }
    if let Some(section) = dimension_fields_section(
        "角色设定",
        "arc",
        "角色弧光",
        name,
        &[
            ("角色弧光", character.arc_summary.as_deref()),
            ("内在弧线", arc_stages_text.as_deref()),
        ],
        false,
    ) {
        sections.push(section);
    }
    if let Some(section) = dimension_fields_section(
        "角色设定",
        "relationship_to_protagonist",
        "与主角关系",
        name,
        &[(
            "与主角关系",
            character.relationship_to_protagonist.as_deref(),
        )],
        false,
    ) {
        sections.push(section);
    }
    if sections.is_empty() {
        return Err(format!(
            "蓝图角色 {} 没有可入库内容，跳过 RAG 入库",
            character.id
        ));
    }
    Ok(sections)
}

pub fn blueprint_character_rag_records(
    project_id: i64,
    character: &blueprint_character::Model,
) -> Result<Vec<RagRecord>, String> {
    let name = required_rag_text(
        Some(&character.name),
        format!("蓝图角色 {} 缺少姓名，跳过 RAG 入库", character.id),
    )?;
    let sections = blueprint_character_rag_sections(character)?;
    Ok(sections
        .into_iter()
        .enumerate()
        .map(|(section_index, section)| {
            let title = format!("{name} 角色设定: {}", section.label);
            RagRecord {
                chunk_id: format!(
                    "character:{project_id}:blueprint:{}:{}",
                    character.id, section.dimension
                ),
                text: section.text,
                metadata: serde_json::json!({
                    "data_type": RagDataType::Character.as_str(),
                    "source": "blueprint_character",
                    "character_id": character.id,
                    "character_name": name,
                    "dimension": section.dimension,
                    "section_index": section_index,
                    "title": title,
                }),
            }
        })
        .collect())
}
