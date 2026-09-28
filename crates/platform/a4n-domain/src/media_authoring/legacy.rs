//! Explicit migration adapters. Normal generation never reconstructs a film
//! script, invents a missing source, or accepts retired beat-reference formats.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

pub mod film;
pub mod source_sync;
pub mod workflow;

use super::{
    AdaptationBeat, AdaptationBeatKind, AdaptationBeatRef, AdaptationImportance, AdaptationOrigin,
    AdaptationPlanV1, AdaptationPlanVersion, AdaptationTarget, AdaptationUnit,
};
use crate::{
    manga::{MangaPageGenerationLayoutV3, MangaPagePromptSourceSummary, MangaPageV4},
    shooting_script::{ChapterShootingScriptV2, ShootingCoverageV1},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyMangaSource {
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub source_revision_key: String,
    pub content_sha256: String,
    pub shooting_script_id: i64,
    pub shooting_script_revision: i32,
    pub shooting_script_sha256: String,
}

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

pub fn saved_manga_source(layout: &Value) -> Result<LegacyMangaSource, String> {
    if layout.get("schema_version").and_then(Value::as_str) != Some("manga_page_generation_v2") {
        return Err("历史漫画布局版本不受支持".into());
    }
    let value = layout.get("source").cloned().ok_or("历史漫画缺少来源")?;
    if ["source_snapshot_id", "source_snapshot_chapter_id"]
        .iter()
        .any(|field| value.get(*field).is_none())
    {
        return Err("历史漫画必须明确记录原文快照身份或 null".into());
    }
    let source: LegacyMangaSource =
        serde_json::from_value(value).map_err(|error| error.to_string())?;
    if source.root_project_id <= 0
        || source.source_chapter_id <= 0
        || source.shooting_script_id <= 0
        || source.shooting_script_revision <= 0
        || !valid_hash(&source.content_sha256)
        || !valid_hash(&source.shooting_script_sha256)
    {
        return Err("历史漫画来源身份或哈希无效".into());
    }
    let key = match (source.source_snapshot_id, source.source_snapshot_chapter_id) {
        (None, None) => format!(
            "live:{}:{}",
            source.source_chapter_id, source.content_sha256
        ),
        (Some(snapshot), Some(chapter)) if snapshot > 0 && chapter > 0 => format!(
            "snapshot:{snapshot}:chapter:{chapter}:{}",
            source.content_sha256
        ),
        _ => return Err("历史漫画来源快照身份不完整".into()),
    };
    if key != source.source_revision_key {
        return Err("历史漫画来源版本不一致".into());
    }
    Ok(source)
}

/// Preserve the actual edited page. The producer script may have been edited
/// or deleted since generation, so its current content is not migration evidence.
pub fn historical_manga_page(
    id: i64,
    layout: &Value,
) -> Result<(AdaptationUnit, MangaPageV4), String> {
    saved_manga_source(layout)?;
    if id <= 0 {
        return Err("历史漫画页面 ID 无效".into());
    }
    let unit_id = format!("legacy-page-{id}");
    let mut page = layout
        .get("page_plan")
        .cloned()
        .ok_or("历史漫画缺少分页分镜")?;
    validate_legacy_refs(page.get("beat_refs"))?;
    let panels = page
        .get_mut("panels")
        .and_then(Value::as_array_mut)
        .ok_or("历史漫画画格必须是数组")?;
    let mut refs = Vec::new();
    for panel in panels {
        validate_legacy_refs(panel.get("beat_refs"))?;
        let number = panel
            .get("panel_number")
            .and_then(Value::as_u64)
            .filter(|number| *number > 0)
            .ok_or("历史漫画画格序号无效")?;
        let reference = AdaptationBeatRef {
            unit_id: unit_id.clone(),
            beat_id: format!("panel-{number}"),
        };
        panel["beat_refs"] =
            serde_json::to_value(vec![&reference]).map_err(|error| error.to_string())?;
        refs.push(reference);
    }
    page["beat_refs"] = serde_json::to_value(refs).map_err(|error| error.to_string())?;
    let page: MangaPageV4 = serde_json::from_value(page).map_err(|error| error.to_string())?;
    let unit = AdaptationUnit {
        id: unit_id,
        origin: AdaptationOrigin::Historical,
        title: page.page_goal.clone(),
        location: None,
        participants: page
            .panels
            .iter()
            .flat_map(|panel| panel.participants.iter().cloned())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect(),
        dramatic_question: page.visual_thesis.clone(),
        objective: page.page_goal.clone(),
        obstacle: "历史页面未单独记录人物阻力".into(),
        entry_state: page.entry_state.clone(),
        exit_state: page.exit_state.clone(),
        source_span_ids: Vec::new(),
        beats: page
            .panels
            .iter()
            .map(|panel| AdaptationBeat {
                id: format!("panel-{}", panel.panel_number),
                kind: match panel.function {
                    crate::manga::MangaPanelFunctionV2::Dialogue => AdaptationBeatKind::Dialogue,
                    crate::manga::MangaPanelFunctionV2::Reaction => AdaptationBeatKind::Reaction,
                    crate::manga::MangaPanelFunctionV2::Revelation => {
                        AdaptationBeatKind::Revelation
                    }
                    crate::manga::MangaPanelFunctionV2::Transition => {
                        AdaptationBeatKind::Transition
                    }
                    _ => AdaptationBeatKind::Action,
                },
                is_climax: page.role == crate::manga::MangaPageRoleV2::Climax,
                origin: AdaptationOrigin::Historical,
                importance: AdaptationImportance::Required,
                action: panel.observable_action.clone(),
                reaction: panel.reaction.clone(),
                dialogue_intent: (!panel.text_layers.is_empty()).then(|| {
                    panel
                        .text_layers
                        .iter()
                        .map(|layer| layer.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                }),
                entry_state: page.entry_state.clone(),
                exit_state: page.exit_state.clone(),
                source_span_ids: Vec::new(),
            })
            .collect(),
    };
    Ok((unit, page))
}

pub fn upgrade_saved_manga_layout(
    layout: &Value,
    source: MangaPagePromptSourceSummary,
    page: MangaPageV4,
) -> Result<Value, String> {
    let old = saved_manga_source(layout)?;
    if source.root_project_id != old.root_project_id
        || source.source_chapter_id != old.source_chapter_id
        || source.content_sha256 != old.content_sha256
    {
        return Err("迁移后的漫画原文与保存页面不一致".into());
    }
    let mut value = layout.clone();
    value["schema_version"] = "manga_page_generation_v3".into();
    let number = page.page_number;
    value["page_plan"] = serde_json::to_value(page).map_err(|error| error.to_string())?;
    value["source"] = serde_json::to_value(source).map_err(|error| error.to_string())?;
    let parsed: MangaPageGenerationLayoutV3 =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    parsed.validate(number).map_err(|error| error.to_string())?;
    Ok(value)
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_legacy_refs(value: Option<&Value>) -> Result<(), String> {
    let values = value
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty())
        .ok_or("历史漫画缺少原始节拍引用")?;
    for value in values {
        let reference: LegacyShootingBeatRef =
            serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
        if reference.scene_index == 0 || reference.shot_index == 0 || reference.beat_index == 0 {
            return Err("历史漫画原始节拍引用无效".into());
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyShootingBeatRef {
    pub scene_index: u32,
    pub shot_index: u32,
    pub beat_index: u32,
}

pub struct LegacyShootingIdentity<'a> {
    pub id: i64,
    pub revision: i32,
    pub sha256: &'a str,
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub source_revision_key: &'a str,
    pub source_content_sha256: &'a str,
}

pub fn adaptation_from_shooting_script(
    script: &ChapterShootingScriptV2,
    target: AdaptationTarget,
) -> Result<
    (
        AdaptationPlanV1,
        BTreeMap<LegacyShootingBeatRef, AdaptationBeatRef>,
    ),
    String,
> {
    script.validate().map_err(|error| error.to_string())?;
    let mut mapping = BTreeMap::new();
    let mut units = Vec::with_capacity(script.scenes.len());
    for scene in &script.scenes {
        let mut beats = Vec::new();
        for shot in &scene.shots {
            for beat in &shot.performance_beats {
                let id = serde_json::to_string(&(&shot.shot_id, &beat.beat_id))
                    .map_err(|error| error.to_string())?;
                let reference = AdaptationBeatRef {
                    unit_id: scene.scene_identity.clone(),
                    beat_id: id.clone(),
                };
                let old = LegacyShootingBeatRef {
                    scene_index: scene.scene_index,
                    shot_index: shot.shot_index,
                    beat_index: beat.beat_index,
                };
                if mapping.insert(old, reference).is_some() {
                    return Err("历史拍摄节拍身份重复，不能转换改编来源".into());
                }
                beats.push(AdaptationBeat {
                    id,
                    kind: if beat.dialogue_intent.is_some() {
                        AdaptationBeatKind::Dialogue
                    } else if beat.reaction.is_some() {
                        AdaptationBeatKind::Reaction
                    } else if !beat.knowledge_delta.is_empty() {
                        AdaptationBeatKind::Revelation
                    } else if beat.spatial_change.is_some() {
                        AdaptationBeatKind::Transition
                    } else {
                        AdaptationBeatKind::Action
                    },
                    is_climax: beat.required
                        && (!beat.knowledge_delta.is_empty() || beat.relationship_delta.is_some()),
                    origin: if beat.source_span_ids.is_empty() {
                        AdaptationOrigin::Historical
                    } else {
                        AdaptationOrigin::Adapted
                    },
                    importance: if beat.required || shot.coverage == ShootingCoverageV1::MustShow {
                        AdaptationImportance::Required
                    } else if shot.coverage == ShootingCoverageV1::ShouldShow {
                        AdaptationImportance::Important
                    } else {
                        AdaptationImportance::Supporting
                    },
                    action: beat.visible_action.clone(),
                    reaction: beat.reaction.clone(),
                    dialogue_intent: beat.dialogue_intent.clone(),
                    entry_state: beat.entry_state.clone(),
                    exit_state: beat.exit_state.clone(),
                    source_span_ids: beat.source_span_ids.clone(),
                });
            }
        }
        units.push(AdaptationUnit {
            id: scene.scene_identity.clone(),
            origin: if scene.source_span_ids.is_empty() {
                AdaptationOrigin::Historical
            } else {
                AdaptationOrigin::Adapted
            },
            title: scene.title.clone(),
            location: scene.location.clone(),
            participants: scene.participants.clone(),
            dramatic_question: scene.dramatic_question.clone(),
            objective: scene.scene_objective.clone(),
            obstacle: scene.scene_obstacle.clone(),
            entry_state: scene.entry_state.clone(),
            exit_state: scene.exit_state.clone(),
            source_span_ids: scene.source_span_ids.clone(),
            beats,
        });
    }
    Ok((
        AdaptationPlanV1 {
            schema_version: AdaptationPlanVersion::V1,
            target,
            intention: "保留迁移前已经采用的叙事取舍，后续在目标项目独立改编".into(),
            decisions: Vec::new(),
            units,
        },
        mapping,
    ))
}

pub fn upgrade_manga_layout(
    value: &Value,
    old_script: LegacyShootingIdentity<'_>,
    source: MangaPagePromptSourceSummary,
    mapping: &BTreeMap<LegacyShootingBeatRef, AdaptationBeatRef>,
) -> Result<Value, String> {
    if value.get("schema_version").and_then(Value::as_str) != Some("manga_page_generation_v2") {
        return Err("迁移只接受已校验的 manga_page_generation_v2".into());
    }
    let old_source = value
        .get("source")
        .and_then(Value::as_object)
        .ok_or("历史漫画页缺少来源")?;
    let fields = [
        "root_project_id",
        "source_chapter_id",
        "source_snapshot_id",
        "source_snapshot_chapter_id",
        "source_revision_key",
        "content_sha256",
        "shooting_script_id",
        "shooting_script_revision",
        "shooting_script_sha256",
    ];
    if fields.iter().any(|field| !old_source.contains_key(*field))
        || old_source
            .keys()
            .any(|field| !fields.contains(&field.as_str()))
    {
        return Err("历史漫画页来源字段不完整或包含未知字段".into());
    }
    if old_source.get("shooting_script_id").and_then(Value::as_i64) != Some(old_script.id)
        || old_source
            .get("shooting_script_revision")
            .and_then(Value::as_i64)
            != Some(i64::from(old_script.revision))
        || old_source
            .get("shooting_script_sha256")
            .and_then(Value::as_str)
            != Some(old_script.sha256)
        || old_source.get("root_project_id").and_then(Value::as_i64)
            != Some(old_script.root_project_id)
        || old_source.get("source_chapter_id").and_then(Value::as_i64)
            != Some(old_script.source_chapter_id)
        || old_source.get("source_snapshot_id").and_then(Value::as_i64)
            != old_script.source_snapshot_id
        || old_source
            .get("source_snapshot_chapter_id")
            .and_then(Value::as_i64)
            != old_script.source_snapshot_chapter_id
        || old_source
            .get("source_revision_key")
            .and_then(Value::as_str)
            != Some(old_script.source_revision_key)
        || old_source.get("content_sha256").and_then(Value::as_str)
            != Some(old_script.source_content_sha256)
        || source.root_project_id != old_script.root_project_id
        || source.source_chapter_id != old_script.source_chapter_id
        || source.content_sha256 != old_script.source_content_sha256
    {
        return Err("历史漫画页与提供的拍摄剧本版本或原文不一致".into());
    }
    let mut result = value.clone();
    result["schema_version"] = "manga_page_generation_v3".into();
    result["source"] = serde_json::to_value(source).map_err(|error| error.to_string())?;
    let page = result
        .get_mut("page_plan")
        .and_then(Value::as_object_mut)
        .ok_or("历史漫画页缺少分页数据")?;
    upgrade_refs(page.get_mut("beat_refs"), mapping)?;
    let panels = page
        .get_mut("panels")
        .and_then(Value::as_array_mut)
        .ok_or("历史漫画页缺少画格")?;
    for panel in panels {
        upgrade_refs(panel.get_mut("beat_refs"), mapping)?;
    }
    let parsed: MangaPageGenerationLayoutV3 =
        serde_json::from_value(result.clone()).map_err(|error| error.to_string())?;
    parsed
        .validate(parsed.page_plan.page_number)
        .map_err(|error| error.to_string())?;
    Ok(result)
}

fn upgrade_refs(
    value: Option<&mut Value>,
    mapping: &BTreeMap<LegacyShootingBeatRef, AdaptationBeatRef>,
) -> Result<(), String> {
    let references = value
        .and_then(Value::as_array_mut)
        .ok_or("历史漫画节拍引用必须是数组")?;
    for value in references {
        let old: LegacyShootingBeatRef =
            serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
        let next = mapping
            .get(&old)
            .ok_or_else(|| format!("历史漫画引用的拍摄节拍不存在: {old:?}"))?;
        *value = serde_json::to_value(next).map_err(|error| error.to_string())?;
    }
    Ok(())
}
