use std::collections::BTreeMap;

use serde_json::Value;

use super::shooting_script::LegacyShootingBeatRef;
use crate::media_authoring::{
    AdaptationBeat, AdaptationBeatKind, AdaptationImportance, AdaptationOrigin, AdaptationUnit,
};

pub struct HistoricalGalgameScene {
    pub id: i64,
    pub title: String,
    pub location: Option<String>,
    pub participants: Vec<String>,
    pub lines: Vec<HistoricalGalgameLine>,
}

pub struct HistoricalGalgameLine {
    pub id: i64,
    pub index: i32,
    pub line_type: String,
    pub text: String,
    pub source_span_ids: Vec<String>,
}

pub fn historical_galgame_unit_id(id: i64) -> String {
    format!("legacy-scene-{id}")
}

pub fn historical_galgame_beat_id(id: i64) -> String {
    format!("legacy-line-{id}")
}

pub fn upgrade_galgame_presentation(
    value: &Value,
    reference: crate::galgame::GalgamePipelineBeatRefV4,
) -> Result<Value, String> {
    if value.get("schema_version").and_then(Value::as_str) != Some("galgame_line_presentation_v2") {
        return Err("历史游戏演出合同版本不受支持".into());
    }
    reference.validate()?;
    let mut result = value.clone();
    result["schema_version"] = "galgame_line_presentation_v3".into();
    result["origin"] = "historical".into();
    result["beat_ref"] = serde_json::to_value(&reference).map_err(|error| error.to_string())?;
    if !result.get("after_beat_ref").is_some_and(Value::is_null) {
        let old: LegacyShootingBeatRef = serde_json::from_value(result["after_beat_ref"].clone())
            .map_err(|error| error.to_string())?;
        if old.scene_index == 0 || old.shot_index == 0 || old.beat_index == 0 {
            return Err("历史游戏选择点的节拍身份无效".into());
        }
        result["after_beat_ref"] =
            serde_json::to_value(reference).map_err(|error| error.to_string())?;
    }
    serde_json::from_value::<crate::galgame::GalgameLinePresentationDocumentV3>(result.clone())
        .map_err(|error| error.to_string())?;
    Ok(result)
}

pub fn upgrade_galgame_spec_presentations(
    value: &mut Value,
    lines: &BTreeMap<i64, crate::galgame::GalgamePipelineBeatRefV4>,
) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            let line_id = object.get("id").and_then(Value::as_i64);
            for field in ["presentation", "presentation_json"] {
                if let Some(presentation) = object.get_mut(field)
                    && presentation.get("schema_version").and_then(Value::as_str)
                        == Some("galgame_line_presentation_v2")
                {
                    let reference = line_id
                        .and_then(|id| lines.get(&id))
                        .ok_or("历史生成意图的演出行来源不存在")?;
                    *presentation = upgrade_galgame_presentation(presentation, reference.clone())?;
                }
            }
            for nested in object.values_mut() {
                upgrade_galgame_spec_presentations(nested, lines)?;
            }
        }
        Value::Array(values) => {
            for nested in values {
                upgrade_galgame_spec_presentations(nested, lines)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn historical_galgame_unit(scene: HistoricalGalgameScene) -> Result<AdaptationUnit, String> {
    if scene.id <= 0
        || scene.lines.iter().any(|line| {
            line.id <= 0
                || line.index <= 0
                || crate::galgame::GalgameLineType::parse(&line.line_type).is_none()
        })
    {
        return Err("历史 Galgame 场景或演出行身份无效".into());
    }
    let source_span_ids = scene
        .lines
        .iter()
        .flat_map(|line| line.source_span_ids.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut beats = scene
        .lines
        .into_iter()
        .map(|line| {
            let speaking = matches!(line.line_type.as_str(), "dialogue" | "monologue");
            AdaptationBeat {
                id: historical_galgame_beat_id(line.id),
                kind: if speaking {
                    AdaptationBeatKind::Dialogue
                } else {
                    AdaptationBeatKind::Action
                },
                is_climax: false,
                origin: AdaptationOrigin::Historical,
                importance: AdaptationImportance::Required,
                dialogue_intent: speaking.then(|| line.text.clone()),
                action: if line.text.trim().is_empty() {
                    format!("已保存的演出指令（{}）", line.line_type)
                } else {
                    line.text
                },
                reaction: None,
                entry_state: format!("现有场景第 {} 行之前", line.index),
                exit_state: format!("已呈现现有第 {} 行", line.index),
                source_span_ids: line.source_span_ids,
            }
        })
        .collect::<Vec<_>>();
    if beats.is_empty() {
        beats.push(AdaptationBeat {
            id: "unfinished".into(),
            kind: AdaptationBeatKind::Action,
            is_climax: false,
            origin: AdaptationOrigin::Historical,
            importance: AdaptationImportance::Supporting,
            action: "保留尚未写入演出行的历史场景".into(),
            reaction: None,
            dialogue_intent: None,
            entry_state: String::new(),
            exit_state: String::new(),
            source_span_ids: Vec::new(),
        });
    }
    Ok(AdaptationUnit {
        id: historical_galgame_unit_id(scene.id),
        origin: AdaptationOrigin::Historical,
        title: scene.title,
        location: scene.location,
        participants: scene.participants,
        dramatic_question: "历史脚本未单独保存戏剧问题，可在改编计划中补充".into(),
        objective: "保留现有场景和人工编辑内容".into(),
        obstacle: "历史脚本未单独保存角色阻力".into(),
        entry_state: "进入已保存的场景".into(),
        exit_state: "完成已保存的场景".into(),
        source_span_ids,
        beats,
    })
}
