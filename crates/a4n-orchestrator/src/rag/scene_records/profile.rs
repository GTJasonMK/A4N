use a4n_core::{models::scene::scene_profile, rag::RagDataType};
use a4n_rag_core::record_text::{optional_json_text, required_rag_text};

use super::super::RagRecord;

fn scene_profile_rag_text(profile: &scene_profile::Model) -> Result<String, String> {
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

pub fn scene_profile_rag_record(
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

pub fn scene_profile_rag_record_label(profile: &scene_profile::Model) -> String {
    format!("场景档案：{}", profile.name)
}
