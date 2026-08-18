use a4n_core::{models::project::novel_blueprint, rag::RagDataType};
use a4n_rag_core::record_text::format_json_for_rag;
use serde_json::Value;

use super::super::{RagRecord, rag_chunk_id};

fn blueprint_world_setting_sections(
    blueprint: &novel_blueprint::Model,
) -> Result<Vec<(String, String)>, String> {
    let Some(world_setting) = blueprint.world_setting.as_ref() else {
        return Err(format!("蓝图 {} 缺少世界观，跳过 RAG 入库", blueprint.id));
    };
    if world_setting.is_null() {
        return Err(format!("蓝图 {} 世界观为空，跳过 RAG 入库", blueprint.id));
    }
    let sections = world_setting_sections(world_setting);
    if sections.is_empty() {
        return Err(format!(
            "蓝图 {} 世界观没有可入库内容，跳过 RAG 入库",
            blueprint.id
        ));
    }
    Ok(sections)
}

pub fn blueprint_world_setting_rag_records(
    project_id: i64,
    blueprint: &novel_blueprint::Model,
) -> Result<Vec<RagRecord>, String> {
    let sections = blueprint_world_setting_sections(blueprint)?;
    Ok(sections
        .into_iter()
        .map(|(field, text)| RagRecord {
            chunk_id: rag_chunk_id(
                project_id,
                RagDataType::WorldSetting,
                format_args!("{}:{field}", blueprint.id),
            ),
            text,
            metadata: serde_json::json!({
                "data_type": RagDataType::WorldSetting.as_str(),
            }),
        })
        .collect())
}

fn world_setting_sections(value: &Value) -> Vec<(String, String)> {
    let Some(object) = value.as_object() else {
        let text = format_json_for_rag(value);
        return if text.trim().is_empty() {
            Vec::new()
        } else {
            vec![("world_setting".to_string(), format!("世界观设定:\n{text}"))]
        };
    };

    object
        .iter()
        .filter_map(|(key, value)| {
            let text = format_json_for_rag(value);
            if text.trim().is_empty() {
                return None;
            }
            Some((
                key.clone(),
                format!("[世界观-{}]\n{}", world_setting_label(key), text),
            ))
        })
        .collect()
}

fn world_setting_label(key: &str) -> &'static str {
    match key {
        "premise" => "世界前提",
        "core_rules" => "核心规则",
        "technology_or_magic" => "能力/机制体系",
        "key_locations" => "关键地点",
        "factions" => "主要阵营",
        "history_timeline" => "历史脉络",
        "culture_and_society" => "文化与社会",
        "conflict_sources" => "冲突来源",
        "mysteries_and_foreshadowing" => "悬念与伏笔",
        "genre_promises" => "类型承诺",
        _ => "其他设定",
    }
}
