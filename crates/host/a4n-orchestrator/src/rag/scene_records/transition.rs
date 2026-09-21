use a4n_db::rag::RagSceneTransitionRow;
use a4n_domain::rag::RagDataType;
use a4n_rag_core::RagRecord;
use a4n_rag_core::record_text::{optional_json_text, optional_rag_text, required_rag_text};

use super::super::rag_chunk_id;

fn scene_transition_rag_text(
    transition: &RagSceneTransitionRow,
    from_scene_name: Option<&str>,
    to_scene_name: &str,
) -> Result<String, String> {
    let to_scene_name = required_rag_text(
        Some(to_scene_name),
        format!("场景位移 {} 缺少目标场景名称，跳过 RAG 入库", transition.id),
    )?;
    let mut lines = vec![
        "[场景位移]".to_string(),
        format!("章节: 第{}章", transition.chapter_number),
        format!("顺序: {}", transition.transition_order),
        format!("类型: {}", transition.movement_type),
        format!("来源场景: {}", from_scene_name.unwrap_or("未记录/章节开头")),
        format!("目标场景: {to_scene_name}"),
        format!("路径/转场: {}", transition.route_description),
    ];
    if let Some(time_cost) = optional_rag_text(transition.time_cost.as_deref()) {
        lines.push(format!("耗时: {time_cost}"));
    }
    if let Some(distance) = optional_rag_text(transition.distance_hint.as_deref()) {
        lines.push(format!("距离/空间关系: {distance}"));
    }
    if let Some(participants) = optional_json_text(Some(&transition.participants_json)) {
        lines.push(format!("参与角色: {participants}"));
    }
    if let Some(objects) = optional_json_text(Some(&transition.object_refs_json)) {
        lines.push(format!("相关物体: {objects}"));
    }
    Ok(lines.join("\n"))
}

pub fn scene_transition_rag_record(
    project_id: i64,
    transition: &RagSceneTransitionRow,
    from_scene_name: Option<&str>,
    to_scene_name: &str,
) -> Result<RagRecord, String> {
    Ok(RagRecord {
        chunk_id: rag_chunk_id(project_id, RagDataType::SceneTransition, transition.id),
        text: scene_transition_rag_text(transition, from_scene_name, to_scene_name)?,
        metadata: serde_json::json!({
            "data_type": RagDataType::SceneTransition.as_str(),
            "chapter_number": transition.chapter_number,
        }),
    })
}
