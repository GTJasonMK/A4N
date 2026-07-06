use super::json_format::format_json_for_rag;
use super::{optional_json_text, optional_rag_text, required_rag_text};
use crate::rag_ingest::RagRecord;
use a4n_core::models::project::{blueprint_relationship, novel_blueprint};
use a4n_core::rag::RagDataType;
use serde_json::Value;

pub(in crate::rag_ingest) fn blueprint_rag_text(
    blueprint: &novel_blueprint::Model,
) -> Result<String, String> {
    let title = required_rag_text(
        Some(&blueprint.title),
        format!("蓝图 {} 缺少标题，跳过 RAG 入库", blueprint.id),
    )?;
    let genre = required_rag_text(
        Some(&blueprint.genre),
        format!("蓝图 {} 缺少类型，跳过 RAG 入库", blueprint.id),
    )?;
    let full_synopsis = required_rag_text(
        blueprint.full_synopsis.as_deref(),
        format!("蓝图 {} 缺少完整梗概，跳过 RAG 入库", blueprint.id),
    )?;
    let mut lines = vec![
        format!("标题: {title}"),
        format!("类型: {genre}"),
        format!("完整梗概: {full_synopsis}"),
    ];
    if let Some(summary) = optional_rag_text(blueprint.one_sentence_summary.as_deref()) {
        lines.insert(2, format!("简述: {summary}"));
    }
    if let Some(world_setting) = optional_json_text(blueprint.world_setting.as_ref()) {
        lines.push(format!("世界观: {world_setting}"));
    }
    Ok(lines.join("\n"))
}

pub(in crate::rag_ingest) fn blueprint_rag_record(
    project_id: i64,
    blueprint: &novel_blueprint::Model,
) -> Result<RagRecord, String> {
    Ok(RagRecord {
        chunk_id: format!("blueprint:{project_id}:{}", blueprint.id),
        text: blueprint_rag_text(blueprint)?,
        metadata: serde_json::json!({
            "data_type": RagDataType::Blueprint.as_str(),
            "blueprint_id": blueprint.id,
        }),
    })
}

pub(in crate::rag_ingest) fn blueprint_world_setting_sections(
    blueprint: &novel_blueprint::Model,
) -> Result<Vec<(String, String)>, String> {
    let Some(world_setting) = blueprint.world_setting.as_ref() else {
        return Err(format!("蓝图 {} 缺少世界观，跳过 RAG 入库", blueprint.id));
    };
    if world_setting.is_null() {
        return Err(format!("蓝图 {} 世界观为空，跳过 RAG 入库", blueprint.id));
    }
    let sections = world_setting_sections(world_setting);
    if sections.is_empty() {
        return Err(format!(
            "蓝图 {} 世界观没有可入库内容，跳过 RAG 入库",
            blueprint.id
        ));
    }
    Ok(sections)
}

pub(in crate::rag_ingest) fn blueprint_world_setting_rag_records(
    project_id: i64,
    blueprint: &novel_blueprint::Model,
) -> Result<Vec<RagRecord>, String> {
    let sections = blueprint_world_setting_sections(blueprint)?;
    Ok(sections
        .into_iter()
        .enumerate()
        .map(|(section_index, (field, text))| RagRecord {
            chunk_id: format!("world_setting:{project_id}:{}:{field}", blueprint.id),
            text,
            metadata: serde_json::json!({
                "data_type": RagDataType::WorldSetting.as_str(),
                "blueprint_id": blueprint.id,
                "field": field,
                "section_index": section_index,
            }),
        })
        .collect())
}

pub(in crate::rag_ingest) fn blueprint_metadata_rag_text(
    blueprint: &novel_blueprint::Model,
) -> Result<String, String> {
    let title = required_rag_text(
        Some(&blueprint.title),
        format!("蓝图元数据 {} 缺少标题，跳过 RAG 入库", blueprint.id),
    )?;
    let genre = required_rag_text(
        Some(&blueprint.genre),
        format!("蓝图元数据 {} 缺少类型，跳过 RAG 入库", blueprint.id),
    )?;
    let mut lines = vec![format!("小说标题: {title}"), format!("体裁类型: {genre}")];
    if let Some(style) = optional_rag_text(blueprint.style.as_deref()) {
        lines.push(format!("写作风格: {style}"));
    }
    if let Some(tone) = optional_rag_text(blueprint.tone.as_deref()) {
        lines.push(format!("情感基调: {tone}"));
    }
    if let Some(target_audience) = optional_rag_text(blueprint.target_audience.as_deref()) {
        lines.push(format!("目标读者: {target_audience}"));
    }
    if let Some(summary) = optional_rag_text(blueprint.one_sentence_summary.as_deref()) {
        lines.push(format!("一句话简介: {summary}"));
    }
    Ok(lines.join("\n"))
}

pub(in crate::rag_ingest) fn blueprint_metadata_rag_record(
    project_id: i64,
    blueprint: &novel_blueprint::Model,
) -> Result<RagRecord, String> {
    Ok(RagRecord {
        chunk_id: format!("blueprint_metadata:{project_id}:{}", blueprint.id),
        text: blueprint_metadata_rag_text(blueprint)?,
        metadata: serde_json::json!({
            "data_type": RagDataType::BlueprintMetadata.as_str(),
            "blueprint_id": blueprint.id,
        }),
    })
}

pub(in crate::rag_ingest) fn relationship_rag_text(
    relationship: &blueprint_relationship::Model,
) -> Result<String, String> {
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

pub(in crate::rag_ingest) fn relationship_rag_record(
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

#[cfg(test)]
pub(in crate::rag_ingest) fn expected_blueprint_world_setting_record_count(
    blueprint: &novel_blueprint::Model,
) -> usize {
    blueprint_world_setting_sections(blueprint)
        .map(|sections| sections.len())
        .unwrap_or(0)
}

fn world_setting_sections(value: &Value) -> Vec<(String, String)> {
    let Some(object) = value.as_object() else {
        let text = format_json_for_rag(value);
        return if text.trim().is_empty() {
            Vec::new()
        } else {
            vec![("world_setting".to_string(), format!("世界观设定:\n{text}"))]
        };
    };

    object
        .iter()
        .filter_map(|(key, value)| {
            let text = format_json_for_rag(value);
            if text.trim().is_empty() {
                return None;
            }
            Some((
                key.clone(),
                format!("[世界观-{}]\n{}", world_setting_label(key), text),
            ))
        })
        .collect()
}

fn world_setting_label(key: &str) -> &'static str {
    match key {
        "premise" => "世界前提",
        "core_rules" => "核心规则",
        "technology_or_magic" => "能力/机制体系",
        "key_locations" => "关键地点",
        "factions" => "主要阵营",
        "history_timeline" => "历史脉络",
        "culture_and_society" => "文化与社会",
        "conflict_sources" => "冲突来源",
        "mysteries_and_foreshadowing" => "悬念与伏笔",
        "genre_promises" => "类型承诺",
        _ => "其他设定",
    }
}
