use super::{optional_json_text, optional_rag_text, required_rag_text};
use crate::rag_ingest::RagRecord;
use a4n_core::models::scene::{scene_occurrence, scene_profile, scene_transition};
use a4n_core::rag::RagDataType;

pub(in crate::rag_ingest) fn scene_profile_rag_text(
    profile: &scene_profile::Model,
) -> Result<String, String> {
    let name = required_rag_text(
        Some(&profile.name),
        format!("场景档案 {} 缺少名称，跳过 RAG 入库", profile.id),
    )?;
    let mut lines = vec![
        "[场景档案]".to_string(),
        format!("场景: {name}"),
        format!("类型: {}", profile.scene_type),
        format!("重要性: {}", profile.importance),
        format!(
            "章节范围: 第{}章-第{}章",
            profile.first_seen_chapter, profile.last_seen_chapter
        ),
        format!("基础定义: {}", profile.base_description),
        format!("视觉特征: {}", profile.visual_description),
        format!("空间布局: {}", profile.spatial_layout),
        format!("氛围: {}", profile.atmosphere),
    ];
    if let Some(aliases) = optional_json_text(Some(&profile.aliases_json)) {
        lines.push(format!("别名: {aliases}"));
    }
    if let Some(entry_points) = optional_json_text(Some(&profile.entry_points_json)) {
        lines.push(format!("出入口: {entry_points}"));
    }
    if let Some(rules) = optional_json_text(Some(&profile.rules_json)) {
        lines.push(format!("规则: {rules}"));
    }
    if let Some(current_state) = optional_json_text(Some(&profile.current_state_json)) {
        lines.push(format!("当前状态: {current_state}"));
    }
    if let Some(evidence) = optional_json_text(Some(&profile.evidence_json)) {
        lines.push(format!("证据: {evidence}"));
    }
    Ok(lines.join("\n"))
}

pub(in crate::rag_ingest) fn scene_profile_rag_record(
    project_id: i64,
    profile: &scene_profile::Model,
) -> Result<RagRecord, String> {
    Ok(RagRecord {
        chunk_id: format!("scene_profile:{project_id}:{}", profile.id),
        text: scene_profile_rag_text(profile)?,
        metadata: serde_json::json!({
            "data_type": RagDataType::SceneProfile.as_str(),
            "scene_profile_id": profile.id,
            "name": profile.name,
            "scene_type": profile.scene_type,
            "importance": profile.importance,
            "first_seen_chapter": profile.first_seen_chapter,
            "last_seen_chapter": profile.last_seen_chapter,
        }),
    })
}

pub(in crate::rag_ingest) fn scene_occurrence_rag_text(
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

pub(in crate::rag_ingest) fn scene_occurrence_rag_record(
    project_id: i64,
    occurrence: &scene_occurrence::Model,
    scene_name: &str,
) -> Result<RagRecord, String> {
    Ok(RagRecord {
        chunk_id: format!("scene_occurrence:{project_id}:{}", occurrence.id),
        text: scene_occurrence_rag_text(occurrence, scene_name)?,
        metadata: serde_json::json!({
            "data_type": RagDataType::SceneOccurrence.as_str(),
            "scene_occurrence_id": occurrence.id,
            "scene_profile_id": occurrence.scene_profile_id,
            "scene_name": scene_name,
            "chapter_number": occurrence.chapter_number,
            "occurrence_order": occurrence.occurrence_order,
        }),
    })
}

pub(in crate::rag_ingest) fn scene_transition_rag_text(
    transition: &scene_transition::Model,
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

pub(in crate::rag_ingest) fn scene_transition_rag_record(
    project_id: i64,
    transition: &scene_transition::Model,
    from_scene_name: Option<&str>,
    to_scene_name: &str,
) -> Result<RagRecord, String> {
    Ok(RagRecord {
        chunk_id: format!("scene_transition:{project_id}:{}", transition.id),
        text: scene_transition_rag_text(transition, from_scene_name, to_scene_name)?,
        metadata: serde_json::json!({
            "data_type": RagDataType::SceneTransition.as_str(),
            "scene_transition_id": transition.id,
            "from_scene_profile_id": transition.from_scene_profile_id,
            "to_scene_profile_id": transition.to_scene_profile_id,
            "chapter_number": transition.chapter_number,
            "transition_order": transition.transition_order,
            "movement_type": transition.movement_type,
        }),
    })
}
