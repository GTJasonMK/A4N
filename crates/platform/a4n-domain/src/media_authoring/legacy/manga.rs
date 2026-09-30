use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use super::shooting_script::{LegacyShootingBeatRef, LegacyShootingIdentity};
use crate::{
    manga::{MangaPageGenerationLayoutV3, MangaPagePromptSourceSummary, MangaPageV4},
    media_authoring::{
        AdaptationBeat, AdaptationBeatKind, AdaptationBeatRef, AdaptationImportance,
        AdaptationOrigin, AdaptationUnit,
    },
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
