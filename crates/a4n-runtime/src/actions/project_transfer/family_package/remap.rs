use super::validation::snapshot_profile_key;
use super::*;

pub(super) fn map_required(
    map: &HashMap<i64, i64>,
    old_id: i64,
    label: &str,
) -> Result<i64, String> {
    map.get(&old_id)
        .copied()
        .ok_or_else(|| format!("{label} 引用了导入包外或尚未导入的 ID: {old_id}"))
}

pub(super) fn map_optional(
    map: &HashMap<i64, i64>,
    old_id: Option<i64>,
    label: &str,
) -> Result<Option<i64>, String> {
    old_id.map(|id| map_required(map, id, label)).transpose()
}

pub(super) fn remap_profile_id(
    profile_kind: &str,
    old_id: i64,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<i64, String> {
    match profile_kind {
        "character" | "character_profile" => {
            map_required(&maps.character_profile_ids, old_id, label)
        }
        "object" => map_required(&maps.object_profile_ids, old_id, label),
        "scene" => map_required(&maps.scene_profile_ids, old_id, label),
        "event" => Err(format!(
            "{label} 引用了尚未持久化建模的 event 实体: {old_id}"
        )),
        other => Err(format!("{label} 实体类型无效: {other}")),
    }
}

pub(super) fn remap_optional_entity_id(
    entity_type: &str,
    old_id: Option<i64>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<Option<i64>, String> {
    old_id
        .map(|id| remap_profile_id(entity_type, id, maps, label))
        .transpose()
}

pub(super) fn remap_chapter_hashes_json(
    value: &Value,
    maps: &ImportIdMaps,
) -> Result<Value, String> {
    let rows = value
        .as_array()
        .ok_or_else(|| "源文本快照 chapter_hashes_json 必须是数组".to_string())?;
    let mut remapped = Vec::with_capacity(rows.len());
    for row in rows {
        let mut object = row
            .as_object()
            .cloned()
            .ok_or_else(|| "源文本快照 chapter_hashes_json 项必须是对象".to_string())?;
        let old_id = object
            .get("source_chapter_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| "chapter_hashes_json.source_chapter_id 必须是整数".to_string())?;
        object.insert(
            "source_chapter_id".to_string(),
            Value::from(map_required(
                &maps.chapter_ids,
                old_id,
                "chapter_hashes_json.source_chapter_id",
            )?),
        );
        remapped.push(Value::Object(object));
    }
    Ok(Value::Array(remapped))
}

pub(super) fn remap_profile_hashes_json(
    value: &Value,
    snapshot_id: i64,
    snapshot_profiles: &HashMap<(i64, String), &source_snapshot_profile::Model>,
    maps: &ImportIdMaps,
) -> Result<Value, String> {
    let rows = value
        .as_array()
        .ok_or_else(|| "源文本快照 profile_hashes_json 必须是数组".to_string())?;
    let mut remapped = Vec::with_capacity(rows.len());
    for row in rows {
        let mut object = row
            .as_object()
            .cloned()
            .ok_or_else(|| "源文本快照 profile_hashes_json 项必须是对象".to_string())?;
        let kind = object
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| "profile_hashes_json.kind 必须是字符串".to_string())?;
        let old_id = object
            .get("id")
            .and_then(Value::as_i64)
            .ok_or_else(|| "profile_hashes_json.id 必须是整数".to_string())?;
        let detail_key = (snapshot_id, snapshot_profile_key(kind, old_id));
        let detail = snapshot_profiles.get(&detail_key).ok_or_else(|| {
            format!(
                "profile_hashes_json 缺少对应源快照档案明细: snapshot_id={snapshot_id}, kind={kind}, id={old_id}"
            )
        })?;
        let (_, payload_hash) = remap_source_snapshot_profile_payload(detail, maps)?;
        object.insert(
            "id".to_string(),
            Value::from(remap_profile_id(
                kind,
                old_id,
                maps,
                "profile_hashes_json.id",
            )?),
        );
        object.insert("hash".to_string(), Value::from(payload_hash));
        remapped.push(Value::Object(object));
    }
    Ok(Value::Array(remapped))
}

pub(super) fn remap_source_snapshot_outline_payload(
    outline: &source_snapshot_outline::Model,
    maps: &ImportIdMaps,
) -> Result<(Value, String), String> {
    let mut object = json_object(&outline.payload_json, "源快照大纲 payload_json")?;
    match outline.outline_kind.as_str() {
        "part" => {
            remap_required_object_id(
                &mut object,
                "id",
                &maps.part_outline_ids,
                "源快照大纲 payload_json",
            )?;
            remap_required_object_id(
                &mut object,
                "project_id",
                &maps.project_ids,
                "源快照大纲 payload_json",
            )?;
        }
        "chapter" => {
            remap_required_object_id(
                &mut object,
                "id",
                &maps.chapter_outline_ids,
                "源快照大纲 payload_json",
            )?;
            remap_required_object_id(
                &mut object,
                "project_id",
                &maps.project_ids,
                "源快照大纲 payload_json",
            )?;
            remap_object_id(
                &mut object,
                "part_id",
                &maps.part_outline_ids,
                "源快照大纲 payload_json",
            )?;
        }
        other => return Err(format!("源快照大纲类型无效: {other}")),
    }
    let payload = Value::Object(object);
    let payload_hash = hash_json(&payload);
    Ok((payload, payload_hash))
}

pub(super) fn remap_source_snapshot_profile_payload(
    profile: &source_snapshot_profile::Model,
    maps: &ImportIdMaps,
) -> Result<(Value, String), String> {
    let mut object = json_object(&profile.payload_json, "源快照档案 payload_json")?;
    match profile.profile_kind.as_str() {
        "character" => {
            remap_required_object_id(
                &mut object,
                "id",
                &maps.character_profile_ids,
                "源快照档案 payload_json",
            )?;
        }
        "object" => {
            remap_required_object_id(
                &mut object,
                "id",
                &maps.object_profile_ids,
                "源快照档案 payload_json",
            )?;
        }
        "scene" => {
            remap_required_object_id(
                &mut object,
                "id",
                &maps.scene_profile_ids,
                "源快照档案 payload_json",
            )?;
            remap_object_id(
                &mut object,
                "parent_scene_id",
                &maps.scene_profile_ids,
                "源快照档案 payload_json",
            )?;
        }
        other => return Err(format!("源快照档案类型无效: {other}")),
    }
    remap_required_object_id(
        &mut object,
        "project_id",
        &maps.project_ids,
        "源快照档案 payload_json",
    )?;
    let payload = Value::Object(object);
    let payload_hash = hash_json(&payload);
    Ok((payload, payload_hash))
}

pub(super) fn remap_snapshot_outline_source_id(
    outline: &source_snapshot_outline::Model,
    maps: &ImportIdMaps,
) -> Result<i64, String> {
    match outline.outline_kind.as_str() {
        "part" => map_required(
            &maps.part_outline_ids,
            outline.source_outline_id,
            "源快照分部大纲 source_outline_id",
        ),
        "chapter" => map_required(
            &maps.chapter_outline_ids,
            outline.source_outline_id,
            "源快照章节大纲 source_outline_id",
        ),
        other => Err(format!("源快照大纲类型无效: {other}")),
    }
}

pub(super) fn remap_text_anchor_source_id(
    anchor: &text_anchor::Model,
    maps: &ImportIdMaps,
) -> Result<i64, String> {
    match anchor.source_kind.as_str() {
        "chapter_content" | "chapter_summary" => {
            if anchor.source_snapshot_id.is_some() {
                map_required(
                    &maps.source_snapshot_chapter_ids,
                    anchor.source_id,
                    "文本锚点 source_id",
                )
            } else {
                map_required(&maps.chapter_ids, anchor.source_id, "文本锚点 source_id")
            }
        }
        "outline" => remap_ambiguous_outline_id(anchor.source_id, maps),
        "blueprint" => remap_ambiguous_blueprint_id(anchor.source_id, maps),
        other => Err(format!("文本锚点 source_kind 无效: {other}")),
    }
}

fn remap_ambiguous_outline_id(old_id: i64, maps: &ImportIdMaps) -> Result<i64, String> {
    remap_ambiguous_id(
        old_id,
        &maps.part_outline_ids,
        &maps.chapter_outline_ids,
        "文本锚点 outline source_id",
    )
}

fn remap_ambiguous_blueprint_id(old_id: i64, maps: &ImportIdMaps) -> Result<i64, String> {
    remap_ambiguous_id(
        old_id,
        &maps.blueprint_ids,
        &maps.blueprint_version_ids,
        "文本锚点 blueprint source_id",
    )
}

fn remap_ambiguous_id(
    old_id: i64,
    first: &HashMap<i64, i64>,
    second: &HashMap<i64, i64>,
    label: &str,
) -> Result<i64, String> {
    match (first.get(&old_id).copied(), second.get(&old_id).copied()) {
        (Some(id), None) | (None, Some(id)) => Ok(id),
        (Some(_), Some(_)) => Err(format!("{label} 存在歧义: {old_id}")),
        (None, None) => Err(format!("{label} 不存在: {old_id}")),
    }
}

pub(super) fn remap_generation_target_id(
    spec: &generation_spec::Model,
    maps: &ImportIdMaps,
    allow_deferred: bool,
) -> Result<Option<i64>, String> {
    let Some(old_id) = spec.target_entity_id else {
        return Ok(None);
    };
    let mapped = match spec.target_entity_kind.as_str() {
        "character" | "object" | "scene" => remap_profile_id(
            &spec.target_entity_kind,
            old_id,
            maps,
            "生成意图 target_entity_id",
        )?,
        "character_profile" => map_required(
            &maps.character_profile_ids,
            old_id,
            "生成意图 character_profile target_entity_id",
        )?,
        "chapter" => map_required(
            &maps.source_snapshot_chapter_ids,
            old_id,
            "生成意图 chapter target_entity_id",
        )?,
        "galgame_scene" => map_required(
            &maps.galgame_scene_ids,
            old_id,
            "生成意图 galgame_scene target_entity_id",
        )?,
        "galgame_line" => map_required(
            &maps.galgame_line_ids,
            old_id,
            "生成意图 galgame_line target_entity_id",
        )?,
        "segment" if allow_deferred => map_required(
            &maps.video_segment_ids,
            old_id,
            "生成意图 segment target_entity_id",
        )?,
        "assembly" if allow_deferred => map_required(
            &maps.assembly_timeline_ids,
            old_id,
            "生成意图 assembly target_entity_id",
        )?,
        "segment" | "assembly" => return Ok(None),
        other => return Err(format!("生成意图 target_entity_kind 无效: {other}")),
    };
    Ok(Some(mapped))
}

pub(super) fn remap_generation_spec_source_version_hash(
    spec: &generation_spec::Model,
    maps: &ImportIdMaps,
) -> Result<String, String> {
    let source_snapshot_id = spec.source_snapshot_id.ok_or_else(|| {
        format!(
            "生成意图 source_version_hash 缺少源快照: generation_spec_id={}",
            spec.id
        )
    })?;
    maps.source_snapshot_version_hashes
        .get(&source_snapshot_id)
        .cloned()
        .ok_or_else(|| {
            format!(
                "生成意图 source_version_hash 引用了导入包外或尚未导入的源快照: {}",
                source_snapshot_id
            )
        })
}

pub(super) fn remap_generation_spec_params_json(
    spec: &generation_spec::Model,
    maps: &ImportIdMaps,
) -> Result<Value, String> {
    let mut object = json_object(&spec.params_json, "生成意图 params_json")?;
    match (spec.target_kind.as_str(), spec.target_entity_kind.as_str()) {
        ("galgame_background", "galgame_scene") => {
            remap_galgame_background_params(spec, &mut object, maps)?;
        }
        ("galgame_cg", "galgame_line") => {
            remap_galgame_cg_params(spec, &mut object, maps)?;
        }
        ("portrait_expression", "character_profile") => {
            remap_galgame_portrait_params(spec, &mut object, maps)?;
        }
        _ => {}
    }
    Ok(Value::Object(object))
}

fn remap_galgame_background_params(
    spec: &generation_spec::Model,
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
) -> Result<(), String> {
    let project_id = map_required(
        &maps.project_ids,
        spec.project_id,
        "Galgame 背景 canonical_key project_id",
    )?;
    let scene_id = required_mapped_target_entity_id(
        spec,
        &maps.galgame_scene_ids,
        "Galgame 背景 target_entity_id",
    )?;
    object.insert(
        "canonical_key".to_string(),
        Value::String(format!("galgame:{project_id}:background:scene:{scene_id}")),
    );
    remap_nested_object_ids(
        object,
        "scene",
        "Galgame 背景 params_json.scene",
        &[("id", &maps.galgame_scene_ids)],
    )?;
    remap_nested_object_ids(
        object,
        "source",
        "Galgame 背景 params_json.source",
        &[
            (
                "source_snapshot_chapter_id",
                &maps.source_snapshot_chapter_ids,
            ),
            ("source_chapter_id", &maps.chapter_ids),
        ],
    )
}

fn remap_galgame_cg_params(
    spec: &generation_spec::Model,
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
) -> Result<(), String> {
    let project_id = map_required(
        &maps.project_ids,
        spec.project_id,
        "Galgame CG canonical_key project_id",
    )?;
    let line_id = required_mapped_target_entity_id(
        spec,
        &maps.galgame_line_ids,
        "Galgame CG target_entity_id",
    )?;
    object.insert(
        "canonical_key".to_string(),
        Value::String(format!("galgame:{project_id}:cg:line:{line_id}")),
    );
    remap_nested_object_ids(
        object,
        "line",
        "Galgame CG params_json.line",
        &[
            ("id", &maps.galgame_line_ids),
            ("speaker_profile_id", &maps.character_profile_ids),
        ],
    )?;
    remap_nested_object_ids(
        object,
        "scene",
        "Galgame CG params_json.scene",
        &[("id", &maps.galgame_scene_ids)],
    )
}

fn remap_galgame_portrait_params(
    spec: &generation_spec::Model,
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
) -> Result<(), String> {
    let project_id = map_required(
        &maps.project_ids,
        spec.project_id,
        "Galgame 立绘差分 canonical_key project_id",
    )?;
    let profile_id = required_mapped_target_entity_id(
        spec,
        &maps.character_profile_ids,
        "Galgame 立绘差分 target_entity_id",
    )?;
    remap_object_id(
        object,
        "character_profile_id",
        &maps.character_profile_ids,
        "Galgame 立绘差分 params_json",
    )?;
    remap_object_array_ids(
        object,
        "source_line_ids",
        &maps.galgame_line_ids,
        "Galgame 立绘差分 params_json.source_line_ids",
    )?;
    let variant_key = galgame_portrait_variant_key(object)?;
    object.insert(
        "canonical_key".to_string(),
        Value::String(format!(
            "galgame:{project_id}:portrait:{profile_id}:{variant_key}"
        )),
    );
    Ok(())
}

fn required_mapped_target_entity_id(
    spec: &generation_spec::Model,
    map: &HashMap<i64, i64>,
    label: &str,
) -> Result<i64, String> {
    let old_id = spec
        .target_entity_id
        .ok_or_else(|| format!("{label} 缺失"))?;
    map_required(map, old_id, label)
}

fn remap_nested_object_ids(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    label: &str,
    fields: &[(&str, &HashMap<i64, i64>)],
) -> Result<(), String> {
    let Some(value) = object.get_mut(key) else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let nested = value
        .as_object_mut()
        .ok_or_else(|| format!("{label} 必须是 JSON 对象"))?;
    remap_object_ids(nested, label, fields)
}

fn remap_object_array_ids(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    map: &HashMap<i64, i64>,
    label: &str,
) -> Result<(), String> {
    let Some(value) = object.get_mut(key) else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let items = value
        .as_array_mut()
        .ok_or_else(|| format!("{label} 必须是整数数组"))?;
    for item in items {
        let old_id = item
            .as_i64()
            .ok_or_else(|| format!("{label} 必须是整数数组"))?;
        *item = Value::from(map_required(map, old_id, label)?);
    }
    Ok(())
}

fn galgame_portrait_variant_key(object: &serde_json::Map<String, Value>) -> Result<String, String> {
    let expression = object
        .get("expression")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let pose = object
        .get("pose")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match (expression, pose) {
        (Some(expression), None) => Ok(format!("expression:{expression}")),
        (Some(expression), Some(pose)) => Ok(format!("expression:{expression}:pose:{pose}")),
        (None, _) => Err("Galgame 立绘差分 params_json 缺少 expression".to_string()),
    }
}

pub(super) fn remap_media_binding_target_id(
    binding: &media_binding::Model,
    maps: &ImportIdMaps,
) -> Result<i64, String> {
    match binding.target_kind.as_str() {
        "character_profile" => map_required(
            &maps.character_profile_ids,
            binding.target_id,
            "媒体绑定 character_profile target_id",
        ),
        "object_profile" => map_required(
            &maps.object_profile_ids,
            binding.target_id,
            "媒体绑定 object_profile target_id",
        ),
        "scene_profile" => map_required(
            &maps.scene_profile_ids,
            binding.target_id,
            "媒体绑定 scene_profile target_id",
        ),
        "video_segment" => map_required(
            &maps.video_segment_ids,
            binding.target_id,
            "媒体绑定 video_segment target_id",
        ),
        "chapter" => map_required(
            &maps.chapter_ids,
            binding.target_id,
            "媒体绑定 chapter target_id",
        ),
        "assembly_timeline" => map_required(
            &maps.assembly_timeline_ids,
            binding.target_id,
            "媒体绑定 assembly_timeline target_id",
        ),
        "galgame_scene" => map_required(
            &maps.galgame_scene_ids,
            binding.target_id,
            "媒体绑定 galgame_scene target_id",
        ),
        "galgame_line" => map_required(
            &maps.galgame_line_ids,
            binding.target_id,
            "媒体绑定 galgame_line target_id",
        ),
        other => Err(format!("媒体绑定 target_kind 无效: {other}")),
    }
}

pub(super) fn remap_source_sync_target_id(
    conflict: &source_sync_conflict::Model,
    maps: &ImportIdMaps,
) -> Result<i64, String> {
    remap_source_sync_target_ref(
        &conflict.conflict_scope,
        &conflict.target_kind,
        conflict.target_id,
        maps,
    )
}

pub(super) fn remap_source_sync_target_ref(
    conflict_scope: &str,
    target_kind: &str,
    old_id: i64,
    maps: &ImportIdMaps,
) -> Result<i64, String> {
    match (conflict_scope, target_kind) {
        ("generation_spec", "segment") | ("text_anchor", "segment") => map_required(
            &maps.generation_spec_ids,
            old_id,
            "同步冲突 generation_spec target_id",
        ),
        ("text_anchor", "chapter") => {
            map_required(&maps.chapter_ids, old_id, "同步冲突 chapter target_id")
        }
        ("text_anchor", "galgame_scene") => map_required(
            &maps.galgame_scene_ids,
            old_id,
            "同步冲突 galgame_scene target_id",
        ),
        ("text_anchor", "galgame_line") => map_required(
            &maps.galgame_line_ids,
            old_id,
            "同步冲突 galgame_line target_id",
        ),
        ("profile", "character") => map_required(
            &maps.character_profile_ids,
            old_id,
            "同步冲突 character target_id",
        ),
        ("profile", "object") => map_required(
            &maps.object_profile_ids,
            old_id,
            "同步冲突 object target_id",
        ),
        ("profile", "scene") => {
            map_required(&maps.scene_profile_ids, old_id, "同步冲突 scene target_id")
        }
        _ => Err(format!(
            "同步冲突 target 引用类型无效: conflict_scope={conflict_scope}, target_kind={target_kind}"
        )),
    }
}

pub(super) fn remap_media_request_snapshot(
    value: Option<&Value>,
    maps: &ImportIdMaps,
) -> Result<Option<Value>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let mut object = json_object(value, "媒体来源 request_snapshot_json")?;
    remap_object_ids(
        &mut object,
        "媒体来源 request_snapshot_json",
        &[
            ("generation_spec_id", &maps.generation_spec_ids),
            ("prompt_artifact_id", &maps.prompt_artifact_ids),
            ("timeline_id", &maps.assembly_timeline_ids),
        ],
    )?;
    remap_object_id(
        &mut object,
        "target_line_id",
        &maps.galgame_line_ids,
        "媒体来源 request_snapshot_json",
    )?;
    remap_media_request_target_entity(&mut object, maps)?;
    object.remove("config_id");
    Ok(Some(Value::Object(object)))
}

fn remap_media_request_target_entity(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
) -> Result<(), String> {
    let Some(target_entity_id) = object.get("target_entity_id") else {
        return Ok(());
    };
    if target_entity_id.is_null() {
        return Ok(());
    }
    let Some(target_entity_kind) = object.get("target_entity_kind").and_then(Value::as_str) else {
        return Ok(());
    };
    let map = match target_entity_kind {
        "character_profile" => &maps.character_profile_ids,
        "galgame_scene" => &maps.galgame_scene_ids,
        "galgame_line" => &maps.galgame_line_ids,
        "segment" => &maps.video_segment_ids,
        "assembly" => &maps.assembly_timeline_ids,
        _ => return Ok(()),
    };
    remap_object_id(
        object,
        "target_entity_id",
        map,
        "媒体来源 request_snapshot_json",
    )
}

pub(super) fn remap_conflict_source_summary(
    conflict: &source_sync_conflict::Model,
    maps: &ImportIdMaps,
) -> Result<Value, String> {
    let mut object = json_object(
        &conflict.source_summary_json,
        "同步冲突 source_summary_json",
    )?;
    match conflict.conflict_scope.as_str() {
        "profile" => remap_profile_source_id(
            &mut object,
            "profile_kind",
            "source_profile_id",
            maps,
            "同步冲突 source_summary_json",
        )?,
        "text_anchor" => {
            remap_object_ids(
                &mut object,
                "同步冲突 source_summary_json",
                &[
                    ("old_text_anchor_id", &maps.text_anchor_ids),
                    ("new_text_anchor_id", &maps.text_anchor_ids),
                ],
            )?;
        }
        _ => {}
    }
    Ok(Value::Object(object))
}

pub(super) fn remap_conflict_local_summary(
    conflict: &source_sync_conflict::Model,
    maps: &ImportIdMaps,
) -> Result<Value, String> {
    let mut object = json_object(&conflict.local_summary_json, "同步冲突 local_summary_json")?;
    remap_object_ids(
        &mut object,
        "同步冲突 local_summary_json",
        &[
            ("generation_spec_id", &maps.generation_spec_ids),
            ("galgame_scene_id", &maps.galgame_scene_ids),
            ("galgame_line_id", &maps.galgame_line_ids),
        ],
    )?;
    Ok(Value::Object(object))
}

pub(super) fn remap_conflict_affected_refs(
    conflict: &source_sync_conflict::Model,
    maps: &ImportIdMaps,
) -> Result<Value, String> {
    let mut object = json_object(&conflict.affected_refs_json, "同步冲突 affected_refs_json")?;
    remap_object_ids(
        &mut object,
        "同步冲突 affected_refs_json",
        &[
            ("generation_spec_id", &maps.generation_spec_ids),
            ("source_chapter_id", &maps.chapter_ids),
            (
                "source_snapshot_chapter_id",
                &maps.source_snapshot_chapter_ids,
            ),
            (
                "old_source_snapshot_chapter_id",
                &maps.source_snapshot_chapter_ids,
            ),
            ("old_text_anchor_id", &maps.text_anchor_ids),
            ("new_text_anchor_id", &maps.text_anchor_ids),
            ("galgame_scene_id", &maps.galgame_scene_ids),
            ("galgame_line_id", &maps.galgame_line_ids),
        ],
    )?;
    remap_profile_source_id(
        &mut object,
        "profile_kind",
        "source_profile_id",
        maps,
        "同步冲突 affected_refs_json",
    )?;
    Ok(Value::Object(object))
}

pub(super) fn remap_conflict_decision_json(
    conflict: &source_sync_conflict::Model,
    maps: &ImportIdMaps,
) -> Result<Option<Value>, String> {
    match (
        &conflict.decision_json,
        conflict.resolution_strategy.as_deref(),
    ) {
        (Some(value), Some(strategy)) => Ok(Some(remap_source_sync_decision_payload(
            strategy,
            value,
            maps,
            "同步冲突 decision_json",
        )?)),
        (Some(_), None) => Err(format!(
            "同步冲突 decision_json 缺少 resolution_strategy: {}",
            conflict.id
        )),
        (None, _) => Ok(None),
    }
}

pub(super) fn remap_source_sync_decision_payload(
    strategy: &str,
    value: &Value,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<Value, String> {
    let mut object = json_object(value, label)?;
    match strategy {
        "keep_local" | "detach" | "delete_local" => {}
        "manual" => {
            remap_object_id(
                &mut object,
                "prompt_artifact_id",
                &maps.prompt_artifact_ids,
                label,
            )?;
        }
        "rebind_anchor" => {
            remap_object_id(
                &mut object,
                "generation_spec_id",
                &maps.generation_spec_ids,
                label,
            )?;
            remap_anchor_mappings(&mut object, maps, label)?;
        }
        "adopt_source" | "rebuild_prompt" => {
            remap_refresh_spec_payload(&mut object, maps, label)?;
        }
        "regenerate_scene" => {
            remap_regenerate_scene_payload(&mut object, maps, label)?;
        }
        "create_replacement" => {
            remap_refresh_spec_payload(&mut object, maps, label)?;
            remap_replacement_segment_payload(&mut object, maps, label)?;
        }
        other => return Err(format!("同步决策 strategy 无效: {other}")),
    }
    Ok(Value::Object(object))
}

fn remap_regenerate_scene_payload(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), String> {
    remap_object_id(object, "galgame_scene_id", &maps.galgame_scene_ids, label)?;
    let Some(result_value) = object.get_mut("result") else {
        return Ok(());
    };
    let result = result_value
        .as_object_mut()
        .ok_or_else(|| format!("{label}.result 必须是 JSON 对象"))?;
    remap_object_ids(
        result,
        label,
        &[
            ("galgame_scene_id", &maps.galgame_scene_ids),
            (
                "old_source_snapshot_chapter_id",
                &maps.source_snapshot_chapter_ids,
            ),
            (
                "new_source_snapshot_chapter_id",
                &maps.source_snapshot_chapter_ids,
            ),
        ],
    )?;
    if let Some(stage_outputs) = result.get_mut("stage_outputs")
        && !stage_outputs.is_null()
    {
        remap_galgame_stage_outputs(
            stage_outputs,
            maps,
            &format!("{label}.result.stage_outputs"),
        )?;
    }
    Ok(())
}

fn remap_galgame_stage_outputs(
    value: &mut Value,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            remap_object_ids(
                object,
                label,
                &[
                    ("text_anchor_id", &maps.text_anchor_ids),
                    ("speaker_profile_id", &maps.character_profile_ids),
                    ("scene_profile_id", &maps.scene_profile_ids),
                    (
                        "source_snapshot_chapter_id",
                        &maps.source_snapshot_chapter_ids,
                    ),
                    ("source_chapter_id", &maps.chapter_ids),
                    ("galgame_scene_id", &maps.galgame_scene_ids),
                    ("galgame_line_id", &maps.galgame_line_ids),
                ],
            )?;
            remap_source_span_id_array(object, "source_span_ids", maps, label)?;
            for nested in object.values_mut() {
                remap_galgame_stage_outputs(nested, maps, label)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                remap_galgame_stage_outputs(item, maps, label)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn remap_source_span_id_array(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), String> {
    let Some(value) = object.get_mut(key) else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let items = value
        .as_array_mut()
        .ok_or_else(|| format!("{label}.{key} 必须是字符串数组"))?;
    for item in items {
        let old = item
            .as_str()
            .ok_or_else(|| format!("{label}.{key} 必须是字符串数组"))?;
        *item = Value::String(remap_source_span_id(old, maps, &format!("{label}.{key}"))?);
    }
    Ok(())
}

fn remap_source_span_id(value: &str, maps: &ImportIdMaps, label: &str) -> Result<String, String> {
    let Some((chapter_prefix, anchor_id)) = value.rsplit_once(":ta") else {
        return Err(format!("{label} 格式无效: {value}"));
    };
    if !chapter_prefix.starts_with("ch") || chapter_prefix.len() <= 2 {
        return Err(format!("{label} 格式无效: {value}"));
    }
    let old_anchor_id = anchor_id
        .parse::<i64>()
        .map_err(|_| format!("{label} 文本锚点 ID 无效: {value}"))?;
    let new_anchor_id = map_required(&maps.text_anchor_ids, old_anchor_id, label)?;
    Ok(format!("{chapter_prefix}:ta{new_anchor_id}"))
}

fn remap_refresh_spec_payload(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), String> {
    remap_object_ids(
        object,
        label,
        &[("generation_spec_id", &maps.generation_spec_ids)],
    )?;
    remap_ref_array(object, "text_anchor_refs", label, |item| {
        remap_object_id(item, "text_anchor_id", &maps.text_anchor_ids, label)
    })?;
    remap_ref_array(object, "entity_refs", label, |item| {
        remap_entity_ref(item, maps, label)
    })?;
    remap_ref_array(object, "media_refs", label, |item| {
        remap_object_id(item, "media_asset_id", &maps.media_asset_ids, label)
    })
}

fn remap_replacement_segment_payload(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), String> {
    let value = object
        .get_mut("replacement_segment")
        .ok_or_else(|| format!("{label}.replacement_segment 缺失"))?;
    let segment = value
        .as_object_mut()
        .ok_or_else(|| format!("{label}.replacement_segment 必须是 JSON 对象"))?;
    remap_object_ids(
        segment,
        label,
        &[
            ("previous_segment_id", &maps.video_segment_ids),
            ("next_segment_id", &maps.video_segment_ids),
        ],
    )
}

fn remap_anchor_mappings(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), String> {
    let mappings = object
        .get_mut("anchor_mappings")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| format!("{label}.anchor_mappings 必须是数组"))?;
    for mapping in mappings {
        let mapping = mapping
            .as_object_mut()
            .ok_or_else(|| format!("{label}.anchor_mappings[] 必须是 JSON 对象"))?;
        remap_object_ids(
            mapping,
            label,
            &[
                ("old_text_anchor_id", &maps.text_anchor_ids),
                ("new_text_anchor_id", &maps.text_anchor_ids),
            ],
        )?;
    }
    Ok(())
}

fn remap_ref_array<F>(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    label: &str,
    remap: F,
) -> Result<(), String>
where
    F: FnMut(&mut serde_json::Map<String, Value>) -> Result<(), String>,
{
    let refs = object
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .ok_or_else(|| format!("{label}.{key} 必须是数组"))?;
    let mut remap = remap;
    for item in refs {
        let item = item
            .as_object_mut()
            .ok_or_else(|| format!("{label}.{key}[] 必须是 JSON 对象"))?;
        remap(item)?;
    }
    Ok(())
}

fn remap_entity_ref(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), String> {
    let entity_kind = object
        .get("entity_kind")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{label}.entity_kind 必须是字符串"))?;
    let entity_id = object
        .get("entity_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{label}.entity_id 必须是整数"))?;
    object.insert(
        "entity_id".to_string(),
        Value::from(remap_profile_id(
            entity_kind,
            entity_id,
            maps,
            &format!("{label}.entity_id"),
        )?),
    );
    Ok(())
}

fn remap_profile_source_id(
    object: &mut serde_json::Map<String, Value>,
    kind_key: &str,
    id_key: &str,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), String> {
    let Some(value) = object.get(id_key) else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let profile_kind = object
        .get(kind_key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{label}.{kind_key} 必须是字符串"))?;
    let old_id = value
        .as_i64()
        .ok_or_else(|| format!("{label}.{id_key} 必须是整数"))?;
    object.insert(
        id_key.to_string(),
        Value::from(remap_profile_id(
            profile_kind,
            old_id,
            maps,
            &format!("{label}.{id_key}"),
        )?),
    );
    Ok(())
}

fn remap_object_id(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    map: &HashMap<i64, i64>,
    label: &str,
) -> Result<(), String> {
    let Some(value) = object.get_mut(key) else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let old_id = value
        .as_i64()
        .ok_or_else(|| format!("{label}.{key} 必须是整数"))?;
    *value = Value::from(map_required(map, old_id, &format!("{label}.{key}"))?);
    Ok(())
}

fn remap_object_ids(
    object: &mut serde_json::Map<String, Value>,
    label: &str,
    fields: &[(&str, &HashMap<i64, i64>)],
) -> Result<(), String> {
    for (key, map) in fields {
        remap_object_id(object, key, map, label)?;
    }
    Ok(())
}

fn remap_required_object_id(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    map: &HashMap<i64, i64>,
    label: &str,
) -> Result<(), String> {
    let value = object
        .get_mut(key)
        .ok_or_else(|| format!("{label}.{key} 缺失"))?;
    let old_id = value
        .as_i64()
        .ok_or_else(|| format!("{label}.{key} 必须是整数"))?;
    *value = Value::from(map_required(map, old_id, &format!("{label}.{key}"))?);
    Ok(())
}

fn json_object(value: &Value, label: &str) -> Result<serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .cloned()
        .ok_or_else(|| format!("{label} 必须是 JSON 对象"))
}
