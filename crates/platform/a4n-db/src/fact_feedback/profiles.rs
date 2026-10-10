use std::collections::HashMap;

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
                super::observation_source(&crate::project::chapter_record(chapter.clone())),
            );
            resolved.push(json!({"chapter_number":chapter.chapter_number,"quote":quote}));
        }
    }
    let observed = Value::Object(changed_after);
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
            after: observed.clone(),
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
    // New object profiles already contain the first observation's current projection.
    if before.is_null() && kind == "object_profile" {
        return Ok(());
    }
    if let Some(projected) =
        profile_facts_before_chapter(db, project_id, kind, id, i32::MAX).await?
    {
        write_projection(db, project_id, kind, id, projected, &observed).await?;
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
    let history = profile_fact_history(db, project_id, kind, id).await?;
    project_profile_facts(&history, chapter_number)
}

pub(crate) async fn profile_facts_before_chapter_batch<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    kind: &str,
    profile_ids: &[i64],
    chapter_number: i32,
) -> Result<HashMap<i64, Value>, FactFeedbackDbError> {
    let histories = profile_fact_histories(db, project_id, kind, profile_ids).await?;
    let mut projections = HashMap::with_capacity(histories.len());
    for (profile_id, history) in histories {
        if let Some(facts) = project_profile_facts(&history, chapter_number)? {
            projections.insert(profile_id, facts);
        }
    }
    Ok(projections)
}

async fn profile_fact_history<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    kind: &str,
    id: i64,
) -> Result<Vec<material_fact_change::Model>, FactFeedbackDbError> {
    Ok(profile_fact_histories(db, project_id, kind, &[id])
        .await?
        .remove(&id)
        .unwrap_or_default())
}

async fn profile_fact_histories<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    kind: &str,
    profile_ids: &[i64],
) -> Result<HashMap<i64, Vec<material_fact_change::Model>>, FactFeedbackDbError> {
    fields(kind)?;
    if profile_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let changes = material_fact_change::Entity::find()
        .filter(material_fact_change::Column::ProjectId.eq(project_id))
        .filter(material_fact_change::Column::MaterialKind.eq(kind))
        .filter(material_fact_change::Column::TargetId.is_in(profile_ids.iter().copied()))
        .order_by_asc(material_fact_change::Column::Id)
        .all(db)
        .await?;
    let mut histories = HashMap::<i64, Vec<material_fact_change::Model>>::new();
    for change in changes {
        if let Some(profile_id) = change.target_id {
            histories.entry(profile_id).or_default().push(change);
        }
    }
    Ok(histories)
}

fn project_profile_facts(
    history: &[material_fact_change::Model],
    chapter_number: i32,
) -> Result<Option<Value>, FactFeedbackDbError> {
    if history.is_empty() {
        return Ok(None);
    }
    let mut state = serde_json::Map::new();
    let mut active_fields = std::collections::BTreeSet::new();
    let mut selected = Vec::new();
    for record in history {
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
        active_fields.extend(after.keys().cloned());
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
    state.retain(|field, _| active_fields.contains(field));
    Ok((!state.is_empty()).then_some(Value::Object(state)))
}

pub(crate) async fn restore_profile_fact<C: ConnectionTrait>(
    db: &C,
    change: &material_fact_change::Model,
) -> Result<(), FactFeedbackDbError> {
    let Some(id) = change.target_id else {
        return Err(FactFeedbackDbError::Invalid("档案事实缺少目标".into()));
    };
    let history = profile_fact_history(db, change.project_id, &change.material_kind, id).await?;
    let mut projected = project_profile_facts(&history, i32::MAX)?.unwrap_or_else(|| json!({}));
    let before = change
        .before_json
        .as_object()
        .ok_or_else(|| FactFeedbackDbError::Invalid("档案事实日志不是对象".into()))?;
    for (field, value) in before {
        // Later surviving observations can arrive out of chapter order. Otherwise undo to the
        // actual old value, which may include a manual edit between two observations.
        if !history.iter().any(|record| {
            record.status == "applied"
                && record.id > change.id
                && record.after_json.get(field).is_some()
        }) {
            projected[field] = value.clone();
        }
    }
    write_projection(
        db,
        change.project_id,
        &change.material_kind,
        id,
        projected,
        &change.after_json,
    )
    .await?;
    Ok(())
}

async fn write_projection<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    kind: &str,
    id: i64,
    mut value: Value,
    expected: &Value,
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
    expected: &Value,
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
    for field in fields {
        if current.get(field) != expected.get(field) {
            value[*field] = current[*field].clone();
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn observe(
        db: &impl crate::DbExecutor,
        chapter: &chapter::Model,
        id: i64,
        color: Option<&str>,
        condition: Option<&str>,
    ) {
        let before = object_profile::Entity::find_by_id(id)
            .one(db)
            .await
            .unwrap()
            .unwrap();
        let mut active: object_profile::ActiveModel = before.clone().into();
        if let Some(color) = color {
            active.visual_description = Set(color.into());
        }
        if let Some(condition) = condition {
            active.material_and_state = Set(condition.into());
        }
        let after = active.update(db).await.unwrap();
        record_profile_fact_observation(
            db,
            chapter.project_id,
            "object_profile",
            id,
            profile_fact_snapshot(&before, "object_profile").unwrap(),
            profile_fact_snapshot(&after, "object_profile").unwrap(),
            std::slice::from_ref(chapter),
            &json!([chapter.content]),
        )
        .await
        .unwrap();
    }

    async fn withdraw(db: &impl crate::DbExecutor, project_id: i64, chapter: i32) {
        let tx = db.begin().await.unwrap();
        super::super::invalidate_feedback_sources(&tx, project_id, chapter)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn replay_preserves_manual_edits_between_observations_and_orders_by_chapter() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let project = crate::project::create_project_record(
            &db,
            crate::project::CreateProjectRecordInput {
                user_id: 1,
                title: "档案回放".into(),
                description: String::new(),
                project_type: "novel_long".into(),
            },
        )
        .await
        .unwrap();
        let now = Utc::now().naive_utc();
        let mut chapters = Vec::new();
        for (index, content) in ["怀表是红色。", "怀表已经破损。", "怀表是金色。"]
            .iter()
            .enumerate()
        {
            chapters.push(
                chapter::ActiveModel {
                    project_id: Set(project.id),
                    chapter_number: Set(index as i32 + 1),
                    content: Set((*content).into()),
                    word_count: Set(7),
                    status: Set("draft".into()),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(&db)
                .await
                .unwrap(),
            );
        }
        let profile = object_profile::ActiveModel {
            project_id: Set(project.id),
            ownership_kind: Set("user".into()),
            name: Set("怀表".into()),
            normalized_name: Set("怀表".into()),
            aliases_json: Set(json!([])),
            object_type: Set("prop".into()),
            importance: Set("primary".into()),
            visual_description: Set("银色".into()),
            material_and_state: Set("完整".into()),
            story_usage: Set("作者用途".into()),
            visual_usage: Set("视觉用途".into()),
            generation_usage: Set("生成用途".into()),
            location_context: Set(None),
            first_seen_chapter: Set(1),
            last_seen_chapter: Set(3),
            evidence_json: Set(json!([])),
            image_prompt_seed: Set("作者提示词".into()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        observe(&db, &chapters[2], profile.id, Some("金色"), None).await;
        observe(&db, &chapters[0], profile.id, Some("红色"), None).await;
        assert_eq!(
            object_profile::Entity::find_by_id(profile.id)
                .one(&db)
                .await
                .unwrap()
                .unwrap()
                .visual_description,
            "金色"
        );
        withdraw(&db, project.id, 3).await;
        assert_eq!(
            object_profile::Entity::find_by_id(profile.id)
                .one(&db)
                .await
                .unwrap()
                .unwrap()
                .visual_description,
            "红色"
        );
        object_profile::ActiveModel {
            id: Set(profile.id),
            visual_description: Set("作者改成蓝色".into()),
            ..Default::default()
        }
        .update(&db)
        .await
        .unwrap();
        observe(&db, &chapters[1], profile.id, None, Some("破损")).await;
        assert_eq!(
            object_profile::Entity::find_by_id(profile.id)
                .one(&db)
                .await
                .unwrap()
                .unwrap()
                .visual_description,
            "作者改成蓝色"
        );
        observe(&db, &chapters[2], profile.id, Some("金色"), None).await;
        withdraw(&db, project.id, 3).await;
        assert_eq!(
            object_profile::Entity::find_by_id(profile.id)
                .one(&db)
                .await
                .unwrap()
                .unwrap()
                .visual_description,
            "作者改成蓝色"
        );
        withdraw(&db, project.id, 1).await;
        let current = object_profile::Entity::find_by_id(profile.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(current.visual_description, "作者改成蓝色");
        assert_eq!(current.material_and_state, "完整");
        assert_eq!(current.story_usage, "作者用途");
        assert_eq!(current.image_prompt_seed, "作者提示词");
        assert!(
            profile_facts_before_chapter(&db, project.id, "object_profile", profile.id, 4)
                .await
                .unwrap()
                .is_none()
        );
    }
}
