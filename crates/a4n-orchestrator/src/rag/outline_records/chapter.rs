use a4n_core::{models::chapter::chapter_outline, rag::RagDataType};
use a4n_rag_core::record_text::{optional_json_text, required_rag_text};

use super::super::{RagRecord, rag_chunk_id};

fn outline_rag_text(outline: &chapter_outline::Model) -> Result<String, String> {
    let title = required_rag_text(
        outline.title.as_deref(),
        format!("大纲 {} 缺少标题，跳过 RAG 入库", outline.id),
    )?;
    let summary = required_rag_text(
        outline.summary.as_deref(),
        format!("大纲 {} 缺少摘要，跳过 RAG 入库", outline.id),
    )?;
    let mut lines = vec![format!("标题: {title}"), format!("摘要: {summary}")];
    if let Some(beats) = optional_json_text(outline.beats.as_ref()) {
        lines.push(format!("节拍: {beats}"));
    }
    if let Some(key_scenes) = optional_json_text(outline.key_scenes.as_ref()) {
        lines.push(format!("关键场景: {key_scenes}"));
    }
    Ok(lines.join("\n"))
}

pub fn outline_rag_record(
    project_id: i64,
    outline: &chapter_outline::Model,
) -> Result<RagRecord, String> {
    Ok(RagRecord {
        chunk_id: rag_chunk_id(project_id, RagDataType::Outline, outline.id),
        text: outline_rag_text(outline)?,
        metadata: serde_json::json!({
            "data_type": RagDataType::Outline.as_str(),
            "chapter_number": outline.chapter_number,
        }),
    })
}
