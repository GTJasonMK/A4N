use a4n_db_models::models::{
    character::protagonist_profile,
    project::{material_fact_change, novel_blueprint},
};
use a4n_domain::fact_feedback::{
    FactChangeOperation, FactMaterialChange, FactMaterialKind, FactSourceRevision,
    apply_fact_json_change, world_setting_document,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Select, Set,
};
use serde_json::Value;

use super::{
    FactFeedbackDbError,
    apply::{
        character_enrichment, persist_blueprint, persist_character_enrichment,
        set_character_enrichment,
    },
};

pub async fn world_setting_before_chapter<C: ConnectionTrait>(
    db: &C,
    blueprint: &novel_blueprint::Model,
    chapter_number: i32,
) -> Result<Value, FactFeedbackDbError> {
    let mut world = world_setting_document(blueprint.world_setting.as_ref());
    let changes = change_history(blueprint.project_id)
        .filter(material_fact_change::Column::Status.eq("applied"))
        .filter(material_fact_change::Column::MaterialKind.eq("world_setting"))
        .filter(material_fact_change::Column::TargetId.eq(blueprint.id))
        .all(db)
        .await?;
    for change in changes {
        if sources_reach_chapter(&change, chapter_number)? {
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
    let changes = change_history(project_id)
        .filter(material_fact_change::Column::Status.ne("retracted"))
        .all(db)
        .await?;
    for change in changes {
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
                "world_setting" => {
                    if let Some(current) = blueprint
                        .as_mut()
                        .filter(|blueprint| Some(blueprint.id) == change.target_id)
                    {
                        let mut value = world_setting_document(current.world_setting.as_ref());
                        if undo_value(&mut value, &change)? {
                            current.world_setting = Some(value);
                            blueprint_changed = true;
                        }
                    }
                }
                "character_voice" | "character_arc_progress" => {
                    result.characters_changed |= undo_character(db, project_id, &change).await?;
                }
                "foreshadowing" => {
                    crate::project::restore_foreshadowing_fact(db, &change).await?;
                    if let Some(id) = change.target_id {
                        result.foreshadowing_ids.push(id);
                    }
                }
                "chapter_feedback" | "planning" | "blueprint_synopsis" | "object_profile"
                | "scene_profile" => {}
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
    let changes = change_history(profile.project_id)
        .filter(material_fact_change::Column::Status.eq("applied"))
        .filter(
            material_fact_change::Column::MaterialKind
                .is_in(["character_voice", "character_arc_progress"]),
        )
        .filter(material_fact_change::Column::TargetId.eq(profile.id))
        .all(db)
        .await?;
    for change in changes {
        if !sources_reach_chapter(&change, chapter_number)? {
            continue;
        }
        let mut value = character_enrichment(profile, &change.material_kind)?;
        if undo_value(&mut value, &change)? {
            set_character_enrichment(profile, &change.material_kind, value)?;
        }
    }
    Ok(())
}

fn change_history(project_id: i64) -> Select<material_fact_change::Entity> {
    material_fact_change::Entity::find()
        .filter(material_fact_change::Column::ProjectId.eq(project_id))
        .order_by_desc(material_fact_change::Column::Id)
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
) -> Result<bool, FactFeedbackDbError> {
    let Some(id) = change.target_id else {
        return Err(FactFeedbackDbError::Invalid("角色反馈缺少目标 ID".into()));
    };
    let Some(profile) = protagonist_profile::Entity::find_by_id(id)
        .lock_exclusive()
        .one(db)
        .await?
    else {
        return Ok(false);
    };
    if profile.project_id != project_id {
        return Err(FactFeedbackDbError::Invalid(
            "角色反馈目标项目不匹配".into(),
        ));
    }
    let mut value = character_enrichment(&profile, &change.material_kind)?;
    if !undo_value(&mut value, change)? {
        return Ok(false);
    }
    persist_character_enrichment(db, profile, &change.material_kind, value).await?;
    Ok(true)
}
