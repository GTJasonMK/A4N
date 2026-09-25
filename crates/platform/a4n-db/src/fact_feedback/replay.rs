use a4n_db_models::models::{
    character::protagonist_profile,
    project::{material_fact_change, novel_blueprint},
};
use a4n_domain::fact_feedback::{
    FactChangeOperation, FactMaterialChange, FactMaterialKind, FactSourceRevision,
    apply_fact_json_change,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::{Value, json};

use super::{
    FactFeedbackDbError,
    apply::{persist_blueprint, world_document},
};

pub async fn world_setting_before_chapter<C: ConnectionTrait>(
    db: &C,
    blueprint: &novel_blueprint::Model,
    chapter_number: i32,
) -> Result<Value, FactFeedbackDbError> {
    let mut world = world_document(blueprint.world_setting.as_ref());
    let changes = applied_changes(db, blueprint.project_id).await?;
    for change in changes {
        if change.material_kind == "world_setting"
            && change.target_id == Some(blueprint.id)
            && sources_reach_chapter(&change, chapter_number)?
        {
            undo_value(&mut world, &change)?;
        }
    }
    Ok(world)
}

#[derive(Default)]
pub struct FeedbackInvalidation {
    pub blueprint_changed: bool,
    pub characters_changed: bool,
    pub foreshadowing_ids: Vec<i64>,
}

pub async fn invalidate_feedback_sources<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    from_chapter: i32,
) -> Result<FeedbackInvalidation, FactFeedbackDbError> {
    let rows = crate::outline::load_outline_generation_source_rows_for_update(db, project_id)
        .await?
        .ok_or_else(|| FactFeedbackDbError::Invalid("项目不存在".into()))?;
    let mut result = FeedbackInvalidation::default();
    let mut blueprint = rows.blueprint;
    let mut blueprint_changed = false;
    for change in applied_changes(db, project_id).await? {
        if !sources_reach_chapter(&change, from_chapter)? {
            continue;
        }
        let profile_change = (change.status == "applied"
            && matches!(
                change.material_kind.as_str(),
                "object_profile" | "scene_profile"
            ))
        .then(|| change.clone());
        if change.status == "applied" {
            match change.material_kind.as_str() {
                "world_setting" | "blueprint_synopsis" => {
                    if let Some(current) = blueprint
                        .as_mut()
                        .filter(|blueprint| Some(blueprint.id) == change.target_id)
                    {
                        let is_world = change.material_kind == "world_setting";
                        let mut value = if is_world {
                            world_document(current.world_setting.as_ref())
                        } else {
                            current
                                .full_synopsis
                                .as_ref()
                                .map(|text| json!(text))
                                .unwrap_or(Value::Null)
                        };
                        if undo_value(&mut value, &change)? {
                            if is_world {
                                current.world_setting = Some(value);
                            } else {
                                current.full_synopsis = value.as_str().map(str::to_string);
                            }
                            blueprint_changed = true;
                        }
                    }
                }
                "character_voice" | "character_arc_progress" => {
                    undo_character(db, project_id, &change).await?;
                    result.characters_changed = true;
                }
                "foreshadowing" => {
                    crate::project::restore_foreshadowing_fact(db, &change).await?;
                    if let Some(id) = change.target_id {
                        result.foreshadowing_ids.push(id);
                    }
                }
                "chapter_feedback" | "planning" | "object_profile" | "scene_profile" => {}
                other => {
                    return Err(FactFeedbackDbError::Invalid(format!(
                        "未知资料反馈类型: {other}"
                    )));
                }
            }
        }
        let mut active: material_fact_change::ActiveModel = change.into();
        active.status = Set("retracted".into());
        active.invalidated_at = Set(Some(Utc::now().naive_utc()));
        active.update(db).await?;
        if let Some(change) = profile_change {
            super::restore_profile_fact(db, &change).await?;
        }
    }
    if blueprint_changed {
        persist_blueprint(db, blueprint.unwrap(), &rows.project).await?;
    }
    result.blueprint_changed = blueprint_changed;
    Ok(result)
}

pub async fn profile_enrichment_before_chapter<C: ConnectionTrait>(
    db: &C,
    profile: &mut protagonist_profile::Model,
    chapter_number: i32,
) -> Result<(), FactFeedbackDbError> {
    for change in applied_changes(db, profile.project_id).await? {
        if change.target_id != Some(profile.id) || !sources_reach_chapter(&change, chapter_number)?
        {
            continue;
        }
        match change.material_kind.as_str() {
            "character_voice" => {
                let mut value = profile.voice_fingerprint.clone().unwrap_or(Value::Null);
                if undo_value(&mut value, &change)? {
                    profile.voice_fingerprint = (!value.is_null()).then_some(value);
                }
            }
            "character_arc_progress" => {
                if let Some(arc) = profile
                    .character_arc
                    .as_mut()
                    .and_then(Value::as_object_mut)
                {
                    let mut value = arc.get("observed_progress").cloned().unwrap_or(Value::Null);
                    if undo_value(&mut value, &change)? {
                        if value.is_null() {
                            arc.remove("observed_progress");
                        } else {
                            arc.insert("observed_progress".into(), value);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

async fn applied_changes<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
) -> Result<Vec<material_fact_change::Model>, FactFeedbackDbError> {
    Ok(material_fact_change::Entity::find()
        .filter(material_fact_change::Column::ProjectId.eq(project_id))
        .filter(material_fact_change::Column::Status.ne("retracted"))
        .order_by_desc(material_fact_change::Column::Id)
        .all(db)
        .await?)
}

fn sources_reach_chapter(
    change: &material_fact_change::Model,
    chapter_number: i32,
) -> Result<bool, FactFeedbackDbError> {
    let sources: Vec<FactSourceRevision> = serde_json::from_value(change.source_refs_json.clone())?;
    Ok(sources
        .iter()
        .any(|source| source.chapter_number >= chapter_number))
}

fn undo_value(
    value: &mut Value,
    record: &material_fact_change::Model,
) -> Result<bool, FactFeedbackDbError> {
    // A later explicit edit owns its new value. Never rewind unrelated manual work.
    let expected = if record.operation == "remove" {
        if record.field_path.is_empty() {
            Some(&Value::Null)
        } else {
            None
        }
    } else {
        Some(&record.after_json)
    };
    if value.pointer(&record.field_path) != expected {
        return Ok(false);
    }
    let (operation, before, after) = match record.operation.as_str() {
        "add" => (
            FactChangeOperation::Remove,
            record.after_json.clone(),
            Value::Null,
        ),
        "replace" => (
            FactChangeOperation::Replace,
            record.after_json.clone(),
            record.before_json.clone(),
        ),
        "remove" => (
            if record.field_path.is_empty() {
                FactChangeOperation::Replace
            } else {
                FactChangeOperation::Add
            },
            Value::Null,
            record.before_json.clone(),
        ),
        other => {
            return Err(FactFeedbackDbError::Invalid(format!(
                "未知资料变更操作: {other}"
            )));
        }
    };
    // Array removal shifts following indexes; reinsert at the original position while rewinding.
    if operation == FactChangeOperation::Add {
        if let Some((parent, key)) = record.field_path.rsplit_once('/') {
            if let Some(items) = value.pointer_mut(parent).and_then(Value::as_array_mut) {
                let index = key
                    .parse::<usize>()
                    .map_err(|_| FactFeedbackDbError::Invalid("撤回的数组索引无效".into()))?;
                if index > items.len() {
                    return Err(FactFeedbackDbError::Invalid(
                        "撤回的数组位置已经变化".into(),
                    ));
                }
                items.insert(index, after);
                return Ok(true);
            }
        }
    }
    apply_fact_json_change(
        value,
        &FactMaterialChange {
            material_kind: FactMaterialKind::WorldSetting,
            target_id: record.target_id,
            path: record.field_path.clone(),
            operation,
            before,
            after,
            reason: record.reason.clone(),
            source_span_ids: Vec::new(),
        },
    )
    .map_err(FactFeedbackDbError::Invalid)
}

async fn undo_character<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    change: &material_fact_change::Model,
) -> Result<(), FactFeedbackDbError> {
    let Some(id) = change.target_id else {
        return Err(FactFeedbackDbError::Invalid("角色反馈缺少目标 ID".into()));
    };
    let Some(profile) = protagonist_profile::Entity::find_by_id(id).one(db).await? else {
        return Ok(());
    };
    if profile.project_id != project_id {
        return Err(FactFeedbackDbError::Invalid(
            "角色反馈目标项目不匹配".into(),
        ));
    }
    let voice = change.material_kind == "character_voice";
    let mut value = if voice {
        profile.voice_fingerprint.clone().unwrap_or(Value::Null)
    } else {
        profile
            .character_arc
            .as_ref()
            .and_then(|arc| arc.get("observed_progress"))
            .cloned()
            .unwrap_or(Value::Null)
    };
    if !undo_value(&mut value, change)? {
        return Ok(());
    }
    let mut active: protagonist_profile::ActiveModel = profile.clone().into();
    if voice {
        active.voice_fingerprint = Set((!value.is_null()).then_some(value));
    } else {
        let mut arc = profile.character_arc.unwrap_or_else(|| json!({}));
        let object = arc
            .as_object_mut()
            .ok_or_else(|| FactFeedbackDbError::Invalid("角色弧线结构无效".into()))?;
        if value.is_null() {
            object.remove("observed_progress");
        } else {
            object.insert("observed_progress".into(), value);
        }
        active.character_arc = Set(Some(arc));
    }
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await?;
    Ok(())
}
