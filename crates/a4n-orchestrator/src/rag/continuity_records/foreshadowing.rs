use a4n_core::{models::character::foreshadowing_index, rag::RagDataType};
use a4n_rag_core::record_text::{optional_json_text, required_rag_text, short_rag_title};

use crate::rag::{RagRecord, foreshadowing_rag_chunk_id};

pub(super) fn foreshadowing_rag_text(item: &foreshadowing_index::Model) -> Result<String, String> {
    let description = required_rag_text(
        Some(&item.description),
        format!("伏笔 {} 缺少描述，跳过 RAG 入库", item.id),
    )?;
    let status = required_rag_text(
        Some(&item.status),
        format!("伏笔 {} 缺少状态，跳过 RAG 入库", item.id),
    )?;

    let mut lines = vec![
        format!("伏笔: {description}"),
        format!("埋设章节: 第{}章", item.setup_chapter),
        format!("状态: {status}"),
        format!("重要度: {}", item.importance),
    ];
    if let Some(payoff_chapter) = item.payoff_chapter {
        lines.push(format!("揭示章节: 第{payoff_chapter}章"));
    }
    if let Some(related_characters) = optional_json_text(item.related_characters.as_ref()) {
        lines.push(format!("关联角色/实体: {related_characters}"));
    }
    Ok(lines.join("\n"))
}

pub fn foreshadowing_rag_record(
    project_id: i64,
    item: &foreshadowing_index::Model,
) -> Result<RagRecord, String> {
    let description = required_rag_text(
        Some(&item.description),
        format!("伏笔 {} 缺少描述，跳过 RAG 入库", item.id),
    )?;
    let text = foreshadowing_rag_text(item)?;
    let title = format!(
        "伏笔 第{}章: {}",
        item.setup_chapter,
        short_rag_title(description, 48)
    );
    Ok(RagRecord {
        chunk_id: foreshadowing_rag_chunk_id(project_id, item.id),
        text,
        metadata: serde_json::json!({
            "data_type": RagDataType::Foreshadowing.as_str(),
            "chapter_number": item.setup_chapter,
            "title": title,
        }),
    })
}
