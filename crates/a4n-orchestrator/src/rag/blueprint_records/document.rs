use a4n_core::{models::project::novel_blueprint, rag::RagDataType};
use a4n_rag_core::record_text::{optional_json_text, optional_rag_text, required_rag_text};

use super::super::{RagRecord, rag_chunk_id};

fn blueprint_rag_text(blueprint: &novel_blueprint::Model) -> Result<String, String> {
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

pub fn blueprint_rag_record(
    project_id: i64,
    blueprint: &novel_blueprint::Model,
) -> Result<RagRecord, String> {
    Ok(RagRecord {
        chunk_id: rag_chunk_id(project_id, RagDataType::Blueprint, blueprint.id),
        text: blueprint_rag_text(blueprint)?,
        metadata: serde_json::json!({
            "data_type": RagDataType::Blueprint.as_str(),
        }),
    })
}

fn blueprint_metadata_rag_text(blueprint: &novel_blueprint::Model) -> Result<String, String> {
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

pub fn blueprint_metadata_rag_record(
    project_id: i64,
    blueprint: &novel_blueprint::Model,
) -> Result<RagRecord, String> {
    Ok(RagRecord {
        chunk_id: rag_chunk_id(project_id, RagDataType::BlueprintMetadata, blueprint.id),
        text: blueprint_metadata_rag_text(blueprint)?,
        metadata: serde_json::json!({
            "data_type": RagDataType::BlueprintMetadata.as_str(),
        }),
    })
}
