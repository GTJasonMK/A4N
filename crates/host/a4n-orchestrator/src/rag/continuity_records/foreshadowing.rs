use a4n_db::rag::RagForeshadowingRow;
use a4n_domain::rag::RagDataType;
use a4n_rag_core::RagRecord;
use a4n_rag_core::record_text::{optional_json_text, required_rag_text, short_rag_title};

use crate::rag::foreshadowing_rag_chunk_id;

pub(super) fn foreshadowing_rag_text(item: &RagForeshadowingRow) -> Result<String, String> {
    foreshadowing_rag_text_fields(
        item.id,
        &item.description,
        item.setup_chapter,
        item.payoff_chapter,
        &item.status,
        item.related_characters.as_ref(),
        item.importance,
    )
}

fn foreshadowing_rag_text_fields(
    id: i64,
    description: &str,
    setup_chapter: i32,
    payoff_chapter: Option<i32>,
    status: &str,
    related_characters: Option<&serde_json::Value>,
    importance: i32,
) -> Result<String, String> {
    let description = required_rag_text(
        Some(description),
        format!("伏笔 {id} 缺少描述，跳过 RAG 入库"),
    )?;
    let status = required_rag_text(Some(status), format!("伏笔 {id} 缺少状态，跳过 RAG 入库"))?;

    let mut lines = vec![
        format!("伏笔: {description}"),
        format!("埋设章节: 第{setup_chapter}章"),
        format!("状态: {status}"),
        format!("重要度: {importance}"),
    ];
    if let Some(payoff_chapter) = payoff_chapter {
        lines.push(format!("揭示章节: 第{payoff_chapter}章"));
    }
    if let Some(related_characters) = optional_json_text(related_characters) {
        lines.push(format!("关联角色/实体: {related_characters}"));
    }
    Ok(lines.join("\n"))
}

pub fn foreshadowing_rag_record(
    project_id: i64,
    item: &RagForeshadowingRow,
) -> Result<RagRecord, String> {
    let description = required_rag_text(
        Some(&item.description),
        format!("伏笔 {} 缺少描述，跳过 RAG 入库", item.id),
    )?;
    let text = foreshadowing_rag_text(item)?;
    foreshadowing_rag_record_fields(project_id, item.id, item.setup_chapter, description, text)
}

pub fn foreshadowing_rag_record_from_row(
    project_id: i64,
    item: &RagForeshadowingRow,
) -> Result<RagRecord, String> {
    let description = required_rag_text(
        Some(&item.description),
        format!("伏笔 {} 缺少描述，跳过 RAG 入库", item.id),
    )?;
    let text = foreshadowing_rag_text_fields(
        item.id,
        &item.description,
        item.setup_chapter,
        item.payoff_chapter,
        &item.status,
        item.related_characters.as_ref(),
        item.importance,
    )?;
    foreshadowing_rag_record_fields(project_id, item.id, item.setup_chapter, description, text)
}

fn foreshadowing_rag_record_fields(
    project_id: i64,
    id: i64,
    setup_chapter: i32,
    description: &str,
    text: String,
) -> Result<RagRecord, String> {
    let title = format!(
        "伏笔 第{}章: {}",
        setup_chapter,
        short_rag_title(description, 48),
    );
    Ok(RagRecord {
        chunk_id: foreshadowing_rag_chunk_id(project_id, id),
        text,
        metadata: serde_json::json!({
            "data_type": RagDataType::Foreshadowing.as_str(),
            "chapter_number": setup_chapter,
            "title": title,
        }),
    })
}
