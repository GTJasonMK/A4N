use a4n_db_models::models::{
    asset::object_profile, chapter::chapter, project::material_fact_change, scene::scene_profile,
};
use a4n_domain::fact_feedback::FactSourceRevision;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::Serialize;
use serde_json::{Value, json};

use super::{FactChangeRecordInput, FactFeedbackDbError, record_fact_change};

fn fields(kind: &str) -> Result<&'static [&'static str], FactFeedbackDbError> {
    match kind {
        "object_profile" => Ok(&[
            "aliases_json",
            "object_type",
            "visual_description",
            "material_and_state",
            "story_usage",
            "location_context",
        ]),
        "scene_profile" => Ok(&[
            "aliases_json",
            "scene_type",
            "base_description",
            "visual_description",
            "spatial_layout",
            "atmosphere",
            "entry_points_json",
            "rules_json",
        ]),
        _ => Err(FactFeedbackDbError::Invalid("未知档案事实类型".into())),
    }
}

pub(crate) fn profile_fact_snapshot(
    profile: &impl Serialize,
    kind: &str,
) -> Result<Value, FactFeedbackDbError> {
    let value = serde_json::to_value(profile)?;
    let mut snapshot = serde_json::Map::new();
    for field in fields(kind)? {
        snapshot.insert(
            (*field).into(),
            value.get(field).cloned().unwrap_or(Value::Null),
        );
    }
    Ok(Value::Object(snapshot))
}

pub(crate) async fn record_profile_fact_observation<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    kind: &str,
    id: i64,
    before: Value,
    after: Value,
    chapters: &[chapter::Model],
    evidence: &Value,
) -> Result<(), FactFeedbackDbError> {
    let mut changed_before = serde_json::Map::new();
    let mut changed_after = serde_json::Map::new();
    for field in fields(kind)? {
        if before.get(field) != after.get(field) {
            changed_before.insert(
                (*field).into(),
                before.get(field).cloned().unwrap_or(Value::Null),
            );
            changed_after.insert(
                (*field).into(),
                after.get(field).cloned().unwrap_or(Value::Null),
            );
        }
    }
    if changed_after.is_empty() {
        return Ok(());
    }
    let quotes = evidence
        .as_array()
        .ok_or_else(|| FactFeedbackDbError::Invalid("档案事实证据必须是数组".into()))?;
    if quotes.is_empty() {
        return Err(FactFeedbackDbError::Invalid("档案事实缺少正文证据".into()));
    }
    let mut sources = std::collections::BTreeMap::new();
    let mut resolved = Vec::new();
    for quote in quotes {
        let quote = quote
            .as_str()
            .filter(|quote| !quote.trim().is_empty())
            .ok_or_else(|| FactFeedbackDbError::Invalid("档案证据必须是原文文本".into()))?;
        let matches: Vec<_> = chapters
            .iter()
            .filter(|chapter| chapter.project_id == project_id && chapter.content.contains(quote))
            .collect();
        if matches.is_empty() {
            return Err(FactFeedbackDbError::Invalid(
                "档案证据不属于当前正文来源".into(),
            ));
        }
        for chapter in matches {
            sources.insert(
                chapter.id,
                FactSourceRevision {
                    chapter_id: chapter.id,
                    chapter_number: chapter.chapter_number,
                    content_revision: a4n_domain::chapter::chapter_edit_revision(
                        &chapter.content,
                        chapter.title.as_deref(),
                    ),
                    analysis_hash: a4n_domain::source_text_hash(
                        &chapter
                            .analysis_data
                            .as_ref()
                            .unwrap_or(&Value::Null)
                            .to_string(),
                    ),
                },
            );
            resolved.push(json!({"chapter_number":chapter.chapter_number,"quote":quote}));
        }
    }
    record_fact_change(
        db,
        FactChangeRecordInput {
            project_id,
            workflow_run_id: None,
            material_kind: kind.into(),
            target_id: Some(id),
            field_path: String::new(),
            operation: "replace".into(),
            before: Value::Object(changed_before),
            after: Value::Object(changed_after),
            sources: sources.into_values().collect(),
            evidence: Value::Array(resolved),
            reason: if kind == "object_profile" {
                "依据本章正文更新物体事实"
            } else {
                "依据本章正文更新场景事实"
            }
            .into(),
            status: "applied".into(),
        },
    )
    .await?;
    if let Some(projected) =
        profile_facts_before_chapter(db, project_id, kind, id, i32::MAX).await?
    {
        write_projection(db, project_id, kind, id, projected, None).await?;
    }
    Ok(())
}

pub async fn profile_facts_before_chapter<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    kind: &str,
    id: i64,
    chapter_number: i32,
) -> Result<Option<Value>, FactFeedbackDbError> {
    fields(kind)?;
    let history = material_fact_change::Entity::find()
        .filter(material_fact_change::Column::ProjectId.eq(project_id))
        .filter(material_fact_change::Column::MaterialKind.eq(kind))
        .filter(material_fact_change::Column::TargetId.eq(id))
        .order_by_asc(material_fact_change::Column::Id)
        .all(db)
        .await?;
    if history.is_empty() {
        return Ok(None);
    }
    let mut state = serde_json::Map::new();
    let mut selected = Vec::new();
    for record in &history {
        let after = record
            .after_json
            .as_object()
            .ok_or_else(|| FactFeedbackDbError::Invalid("档案事实日志不是对象".into()))?;
        for field in after.keys() {
            state.entry(field.clone()).or_insert_with(|| {
                record
                    .before_json
                    .get(field)
                    .cloned()
                    .unwrap_or(Value::Null)
            });
        }
        if record.status != "applied" {
            continue;
        }
        let sources: Vec<FactSourceRevision> =
            serde_json::from_value(record.source_refs_json.clone())?;
        let through = sources
            .iter()
            .map(|source| source.chapter_number)
            .max()
            .ok_or_else(|| FactFeedbackDbError::Invalid("档案事实日志缺少来源".into()))?;
        if through >= chapter_number {
            continue;
        }
        selected.push(((through, record.id), after));
    }
    selected.sort_by_key(|(order, _)| *order);
    for (_, after) in selected {
        state.extend(after.clone());
    }
    Ok(Some(Value::Object(state)))
}

pub(crate) async fn restore_profile_fact<C: ConnectionTrait>(
    db: &C,
    change: &material_fact_change::Model,
) -> Result<(), FactFeedbackDbError> {
    let Some(id) = change.target_id else {
        return Err(FactFeedbackDbError::Invalid("档案事实缺少目标".into()));
    };
    if let Some(projected) =
        profile_facts_before_chapter(db, change.project_id, &change.material_kind, id, i32::MAX)
            .await?
    {
        write_projection(
            db,
            change.project_id,
            &change.material_kind,
            id,
            projected,
            Some(&change.after_json),
        )
        .await?;
    }
    Ok(())
}

async fn write_projection<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    kind: &str,
    id: i64,
    mut value: Value,
    expected: Option<&Value>,
) -> Result<(), FactFeedbackDbError> {
    match kind {
        "object_profile" => {
            let Some(profile) = object_profile::Entity::find_by_id(id).one(db).await? else {
                return Ok(());
            };
            if profile.project_id != project_id {
                return Err(FactFeedbackDbError::Invalid("物体不属于项目".into()));
            }
            preserve_later_edits(
                &mut value,
                &profile_fact_snapshot(&profile, kind)?,
                expected,
                fields(kind)?,
            )?;
            let mut active: object_profile::ActiveModel = profile.into();
            active.aliases_json = Set(array_or_empty(&value["aliases_json"])?);
            // Identity/type remains usable when the sole body source is withdrawn.
            if !value["object_type"].is_null() {
                active.object_type = Set(text(&value["object_type"])?);
            }
            active.visual_description = Set(text(&value["visual_description"])?);
            active.material_and_state = Set(text(&value["material_and_state"])?);
            active.story_usage = Set(text(&value["story_usage"])?);
            active.location_context =
                Set(serde_json::from_value(value["location_context"].clone())?);
            active.updated_at = Set(Utc::now().naive_utc());
            active.update(db).await?;
        }
        "scene_profile" => {
            let Some(profile) = scene_profile::Entity::find_by_id(id).one(db).await? else {
                return Ok(());
            };
            if profile.project_id != project_id {
                return Err(FactFeedbackDbError::Invalid("场景不属于项目".into()));
            }
            preserve_later_edits(
                &mut value,
                &profile_fact_snapshot(&profile, kind)?,
                expected,
                fields(kind)?,
            )?;
            let mut active: scene_profile::ActiveModel = profile.into();
            active.aliases_json = Set(array_or_empty(&value["aliases_json"])?);
            if !value["scene_type"].is_null() {
                active.scene_type = Set(text(&value["scene_type"])?);
            }
            active.base_description = Set(text(&value["base_description"])?);
            active.visual_description = Set(text(&value["visual_description"])?);
            active.spatial_layout = Set(text(&value["spatial_layout"])?);
            active.atmosphere = Set(text(&value["atmosphere"])?);
            active.entry_points_json = Set(array_or_empty(&value["entry_points_json"])?);
            active.rules_json = Set(array_or_empty(&value["rules_json"])?);
            active.updated_at = Set(Utc::now().naive_utc());
            active.update(db).await?;
        }
        _ => return Err(FactFeedbackDbError::Invalid("未知档案事实类型".into())),
    }
    Ok(())
}

fn preserve_later_edits(
    value: &mut Value,
    current: &Value,
    expected: Option<&Value>,
    fields: &[&str],
) -> Result<(), FactFeedbackDbError> {
    if value.is_null() {
        *value = json!({});
    }
    if !value.is_object() {
        return Err(FactFeedbackDbError::Invalid(
            "档案事实快照必须是对象".into(),
        ));
    }
    for field in fields {
        if value.get(field).is_none() {
            value[*field] = current[*field].clone();
        }
    }
    if let Some(expected) = expected {
        for field in fields {
            if current.get(field) != expected.get(field) {
                value[*field] = current[*field].clone();
            }
        }
    }
    Ok(())
}

fn text(value: &Value) -> Result<String, FactFeedbackDbError> {
    if value.is_null() {
        return Ok(String::new());
    }
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| FactFeedbackDbError::Invalid("档案文本事实结构无效".into()))
}

fn array_or_empty(value: &Value) -> Result<Value, FactFeedbackDbError> {
    if value.is_null() {
        return Ok(json!([]));
    }
    if value.is_array() {
        Ok(value.clone())
    } else {
        Err(FactFeedbackDbError::Invalid("档案数组事实结构无效".into()))
    }
}
