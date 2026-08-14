use a4n_core::{models::chapter::part_outline, rag::RagDataType};
use a4n_rag_core::record_text::{optional_json_text, optional_rag_text, required_rag_text};

use super::super::RagRecord;

fn part_outline_rag_text(part: &part_outline::Model) -> Result<String, String> {
    let title = required_rag_text(
        Some(&part.title),
        format!("分部大纲 {} 缺少标题，跳过 RAG 入库", part.id),
    )?;
    let summary = required_rag_text(
        part.summary.as_deref(),
        format!("分部大纲 {} 缺少摘要，跳过 RAG 入库", part.id),
    )?;
    let mut lines = vec![
        format!("分部: {title}"),
        format!(
            "章节范围: 第{}章-第{}章",
            part.start_chapter, part.end_chapter
        ),
        format!("摘要: {summary}"),
    ];
    if let Some(theme) = optional_rag_text(part.theme.as_deref()) {
        lines.push(format!("主题: {theme}"));
    }
    if let Some(key_events) = optional_json_text(part.key_events.as_ref()) {
        lines.push(format!("关键事件: {key_events}"));
    }
    if let Some(character_focus) = optional_json_text(part.character_focus.as_ref()) {
        lines.push(format!("角色焦点: {character_focus}"));
    }
    Ok(lines.join("\n"))
}

pub fn part_outline_rag_record(
    project_id: i64,
    part: &part_outline::Model,
) -> Result<RagRecord, String> {
    Ok(RagRecord {
        chunk_id: format!("part_outline:{project_id}:{}", part.id),
        text: part_outline_rag_text(part)?,
        metadata: serde_json::json!({
            "data_type": RagDataType::PartOutline.as_str(),
            "part_id": part.id,
            "part_number": part.part_number,
            "start_chapter": part.start_chapter,
            "end_chapter": part.end_chapter,
        }),
    })
}
