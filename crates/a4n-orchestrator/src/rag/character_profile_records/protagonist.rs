use a4n_core::{models::character::protagonist_profile, rag::RagDataType};
use a4n_rag_core::record_text::{
    DimensionRagSection, dimension_fields_section, dimension_text_section, optional_json_text,
    required_rag_text,
};

use super::super::{RagRecord, protagonist_rag_chunk_id};

fn protagonist_rag_sections(
    profile: &protagonist_profile::Model,
) -> Result<Vec<DimensionRagSection>, String> {
    let name = required_rag_text(
        Some(&profile.name),
        format!("主角档案 {} 缺少姓名，跳过 RAG 入库", profile.id),
    )?;
    let mut sections = Vec::new();
    if let Some(section) = dimension_fields_section(
        "主角档案",
        "identity",
        "身份",
        name,
        &[
            ("性别", profile.gender.as_deref()),
            ("年龄", profile.age.as_deref()),
        ],
        true,
    ) {
        sections.push(section);
    }
    for (dimension, label, field_label, value) in [
        ("appearance", "外貌", "外貌", profile.appearance.as_deref()),
        ("background", "背景", "背景", profile.background.as_deref()),
        (
            "personality",
            "人格",
            "人格",
            profile.personality.as_deref(),
        ),
    ] {
        if let Some(section) = dimension_fields_section(
            "主角档案",
            dimension,
            label,
            name,
            &[(field_label, value)],
            false,
        ) {
            sections.push(section);
        }
    }
    for (dimension, label, field_label, value) in [
        (
            "base_attributes",
            "基础属性",
            "基础属性",
            optional_json_text(Some(&profile.base_attributes)),
        ),
        (
            "current_state",
            "当前状态",
            "当前状态",
            optional_json_text(Some(&profile.current_state)),
        ),
        (
            "character_arc",
            "角色弧光",
            "角色弧光",
            optional_json_text(profile.character_arc.as_ref()),
        ),
        (
            "voice_fingerprint",
            "声音指纹",
            "声音指纹",
            optional_json_text(profile.voice_fingerprint.as_ref()),
        ),
    ] {
        if let Some(section) =
            dimension_text_section("主角档案", dimension, label, name, field_label, value)
        {
            sections.push(section);
        }
    }
    if sections.is_empty() {
        return Err(format!(
            "主角档案 {} 没有可入库内容，跳过 RAG 入库",
            profile.id
        ));
    }
    Ok(sections)
}

pub fn protagonist_rag_records(
    project_id: i64,
    profile: &protagonist_profile::Model,
) -> Result<Vec<RagRecord>, String> {
    let name = required_rag_text(
        Some(&profile.name),
        format!("主角档案 {} 缺少姓名，跳过 RAG 入库", profile.id),
    )?;
    let sections = protagonist_rag_sections(profile)?;
    Ok(sections
        .into_iter()
        .map(|section| {
            let title = format!("{name} 主角档案: {}", section.label);
            RagRecord {
                chunk_id: protagonist_rag_chunk_id(project_id, profile.id, section.dimension),
                text: section.text,
                metadata: serde_json::json!({
                    "data_type": RagDataType::Protagonist.as_str(),
                    "title": title,
                }),
            }
        })
        .collect())
}
