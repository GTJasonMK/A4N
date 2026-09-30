use a4n_db::rag::RagChapterOutlineRow;
use a4n_domain::chapter::ChapterOutlineRecord;
use a4n_domain::rag::RagDataType;
use a4n_rag_core::RagRecord;
use a4n_rag_core::record_text::{optional_json_text, required_rag_text};
use serde_json::Value;

use super::super::rag_chunk_id;

fn outline_rag_text(
    id: i64,
    title: Option<&str>,
    summary: Option<&str>,
    beats: Option<&Value>,
    key_scenes: Option<&Value>,
) -> Result<String, String> {
    let title = required_rag_text(title, format!("大纲 {id} 缺少标题，跳过 RAG 入库"))?;
    let summary = required_rag_text(summary, format!("大纲 {id} 缺少摘要，跳过 RAG 入库"))?;
    let mut lines = vec![format!("标题: {title}"), format!("摘要: {summary}")];
    if let Some(beats) = optional_json_text(beats) {
        lines.push(format!("节拍: {beats}"));
    }
    if let Some(key_scenes) = optional_json_text(key_scenes) {
        lines.push(format!("关键场景: {key_scenes}"));
    }
    Ok(lines.join("\n"))
}

fn outline_rag_record_fields(
    project_id: i64,
    id: i64,
    chapter_number: i32,
    title: Option<&str>,
    summary: Option<&str>,
    beats: Option<&Value>,
    key_scenes: Option<&Value>,
) -> Result<RagRecord, String> {
    Ok(RagRecord {
        chunk_id: rag_chunk_id(project_id, RagDataType::Outline, id),
        text: outline_rag_text(id, title, summary, beats, key_scenes)?,
        metadata: serde_json::json!({
            "data_type": RagDataType::Outline.as_str(),
            "chapter_number": chapter_number,
        }),
    })
}

pub fn outline_rag_record(
    project_id: i64,
    outline: &ChapterOutlineRecord,
) -> Result<RagRecord, String> {
    outline_rag_record_fields(
        project_id,
        outline.id,
        outline.chapter_number,
        outline.title.as_deref(),
        outline.summary.as_deref(),
        outline.beats.as_ref(),
        outline.key_scenes.as_ref(),
    )
}

pub fn outline_rag_record_from_row(
    project_id: i64,
    outline: &RagChapterOutlineRow,
) -> Result<RagRecord, String> {
    outline_rag_record_fields(
        project_id,
        outline.id,
        outline.chapter_number,
        outline.title.as_deref(),
        outline.summary.as_deref(),
        outline.beats.as_ref(),
        outline.key_scenes.as_ref(),
    )
}
