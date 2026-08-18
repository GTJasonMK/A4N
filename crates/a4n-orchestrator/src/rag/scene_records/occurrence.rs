use a4n_core::{models::scene::scene_occurrence, rag::RagDataType};
use a4n_rag_core::record_text::{optional_json_text, optional_rag_text, required_rag_text};

use super::super::{RagRecord, rag_chunk_id};

fn scene_occurrence_rag_text(
    occurrence: &scene_occurrence::Model,
    scene_name: &str,
) -> Result<String, String> {
    let scene_name = required_rag_text(
        Some(scene_name),
        format!("场景出场 {} 缺少场景名称，跳过 RAG 入库", occurrence.id),
    )?;
    let mut lines = vec![
        "[场景出场]".to_string(),
        format!("场景: {scene_name}"),
        format!("章节: 第{}章", occurrence.chapter_number),
        format!("出场顺序: {}", occurrence.occurrence_order),
        format!("摘要: {}", occurrence.summary),
    ];
    if let Some(timeline) = optional_rag_text(occurrence.timeline_position.as_deref()) {
        lines.push(format!("时间位置: {timeline}"));
    }
    if let Some(mood) = optional_rag_text(occurrence.mood.as_deref()) {
        lines.push(format!("情绪张力: {mood}"));
    }
    if let Some(weather) = optional_rag_text(occurrence.weather_or_lighting.as_deref()) {
        lines.push(format!("天气/光线: {weather}"));
    }
    if let Some(characters) = optional_json_text(Some(&occurrence.characters_json)) {
        lines.push(format!("角色: {characters}"));
    }
    if let Some(objects) = optional_json_text(Some(&occurrence.objects_json)) {
        lines.push(format!("物体: {objects}"));
    }
    if let Some(events) = optional_json_text(Some(&occurrence.key_events_json)) {
        lines.push(format!("关键事件: {events}"));
    }
    if let Some(entry_state) = optional_json_text(Some(&occurrence.entry_state_json)) {
        lines.push(format!("进入状态: {entry_state}"));
    }
    if let Some(exit_state) = optional_json_text(Some(&occurrence.exit_state_json)) {
        lines.push(format!("离开状态: {exit_state}"));
    }
    Ok(lines.join("\n"))
}

pub fn scene_occurrence_rag_record(
    project_id: i64,
    occurrence: &scene_occurrence::Model,
    scene_name: &str,
) -> Result<RagRecord, String> {
    Ok(RagRecord {
        chunk_id: rag_chunk_id(project_id, RagDataType::SceneOccurrence, occurrence.id),
        text: scene_occurrence_rag_text(occurrence, scene_name)?,
        metadata: serde_json::json!({
            "data_type": RagDataType::SceneOccurrence.as_str(),
            "chapter_number": occurrence.chapter_number,
        }),
    })
}
