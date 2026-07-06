//! 素材档案服务。

use crate::generation::{
    GenerationRunTraceTarget, RecordCompletedGenerationRunInput, delete_generation_run_targets_for,
    record_completed_generation_run,
};
use crate::profile_text::{
    merge_text_set_json, normalize_aliases, normalize_required_text_set, required_text,
    validate_importance,
};
use a4n_core::error::AppError;
use a4n_core::models::asset::{object_profile, profile_primary_material};
use a4n_core::models::project::novel_project;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::Arc;

pub const OBJECT_TYPES: &[&str] = &[
    "prop",
    "tool",
    "weapon",
    "token",
    "vehicle",
    "clothing",
    "location_object",
    "environment_object",
    "other",
];

#[derive(Clone)]
pub struct ObjectProfileService {
    db: Arc<DatabaseConnection>,
}

#[derive(Debug, Clone)]
pub struct ObjectProfileDraft {
    pub name: String,
    pub aliases: Vec<String>,
    pub object_type: String,
    pub importance: String,
    pub visual_description: String,
    pub material_and_state: String,
    pub story_usage: String,
    pub visual_usage: String,
    pub generation_usage: String,
    pub location_context: Option<String>,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    pub evidence: Vec<String>,
    pub image_prompt_seed: String,
}

#[derive(Debug, Clone, Default)]
pub struct ObjectProfileUpsertResult {
    pub created: Vec<object_profile::Model>,
    pub updated: Vec<object_profile::Model>,
}

impl ObjectProfileService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn list_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<object_profile::Model>, AppError> {
        self.ensure_project_exists(project_id).await?;
        object_profile::Entity::find()
            .filter(object_profile::Column::ProjectId.eq(project_id))
            .order_by_asc(object_profile::Column::FirstSeenChapter)
            .order_by_asc(object_profile::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn get(&self, object_profile_id: i64) -> Result<object_profile::Model, AppError> {
        object_profile::Entity::find_by_id(object_profile_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("物体档案不存在".to_string()))
    }

    pub async fn delete(&self, object_profile_id: i64) -> Result<(), AppError> {
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        profile_primary_material::Entity::delete_many()
            .filter(profile_primary_material::Column::ProfileKind.eq("object"))
            .filter(profile_primary_material::Column::ProfileId.eq(object_profile_id))
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;
        delete_generation_run_targets_for(&txn, "object_profile", &[object_profile_id]).await?;
        let deleted = object_profile::Entity::delete_by_id(object_profile_id)
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;
        if deleted.rows_affected == 0 {
            return Err(AppError::NotFound("物体档案不存在".to_string()));
        }
        txn.commit().await.map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn create_one(
        &self,
        project_id: i64,
        draft: ObjectProfileDraft,
    ) -> Result<object_profile::Model, AppError> {
        self.ensure_project_exists(project_id).await?;
        let normalized = normalize_object_name(&draft.name)?;
        let draft = normalize_draft(draft)?;
        let existing = object_profile::Entity::find()
            .filter(object_profile::Column::ProjectId.eq(project_id))
            .filter(object_profile::Column::NormalizedName.eq(&normalized))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if existing.is_some() {
            return Err(AppError::InvalidArgument(format!(
                "物体档案已存在: {}",
                draft.name
            )));
        }
        self.create(project_id, draft, normalized).await
    }

    pub async fn update_one(
        &self,
        object_profile_id: i64,
        draft: ObjectProfileDraft,
    ) -> Result<object_profile::Model, AppError> {
        let existing = self.get(object_profile_id).await?;
        let normalized = normalize_object_name(&draft.name)?;
        let draft = normalize_draft(draft)?;
        let duplicate = object_profile::Entity::find()
            .filter(object_profile::Column::ProjectId.eq(existing.project_id))
            .filter(object_profile::Column::NormalizedName.eq(&normalized))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if duplicate
            .as_ref()
            .is_some_and(|profile| profile.id != object_profile_id)
        {
            return Err(AppError::InvalidArgument(format!(
                "物体档案已存在: {}",
                draft.name
            )));
        }
        self.replace_existing(existing, draft, normalized).await
    }

    pub async fn upsert_many_with_generation_run(
        &self,
        project_id: i64,
        drafts: Vec<ObjectProfileDraft>,
        mut generation_run: RecordCompletedGenerationRunInput,
    ) -> Result<ObjectProfileUpsertResult, AppError> {
        if generation_run.project_id != project_id {
            return Err(AppError::InvalidArgument(
                "物体档案生成运行记录 project_id 与项目不一致".to_string(),
            ));
        }
        if !generation_run.targets.is_empty() {
            return Err(AppError::InvalidArgument(
                "物体档案生成运行记录目标由物体服务绑定，调用方不能预填 targets".to_string(),
            ));
        }
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        novel_project::Entity::find_by_id(project_id)
            .one(&txn)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        let result = upsert_object_profiles_in(&txn, project_id, drafts).await?;
        generation_run.targets = result
            .created
            .iter()
            .chain(result.updated.iter())
            .map(|profile| GenerationRunTraceTarget {
                target_kind: "object_profile_record".to_string(),
                target_table: "object_profile".to_string(),
                target_id: profile.id,
            })
            .collect();
        if !generation_run.targets.is_empty() {
            generation_run.response_summary_json = object_profile_generation_response_summary(
                generation_run.response_summary_json,
                &result,
            );
            record_completed_generation_run(&txn, generation_run).await?;
        }
        txn.commit().await.map_err(AppError::Database)?;
        Ok(result)
    }

    async fn create(
        &self,
        project_id: i64,
        draft: ObjectProfileDraft,
        normalized_name: String,
    ) -> Result<object_profile::Model, AppError> {
        create_object_profile_in(self.db.as_ref(), project_id, draft, normalized_name).await
    }

    async fn replace_existing(
        &self,
        existing: object_profile::Model,
        draft: ObjectProfileDraft,
        normalized_name: String,
    ) -> Result<object_profile::Model, AppError> {
        replace_object_profile_in(self.db.as_ref(), existing, draft, normalized_name).await
    }

    async fn ensure_project_exists(&self, project_id: i64) -> Result<(), AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "project_id 必须大于 0".to_string(),
            ));
        }
        novel_project::Entity::find_by_id(project_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .map(|_| ())
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))
    }
}

async fn upsert_object_profiles_in<C>(
    db: &C,
    project_id: i64,
    drafts: Vec<ObjectProfileDraft>,
) -> Result<ObjectProfileUpsertResult, AppError>
where
    C: ConnectionTrait,
{
    let mut result = ObjectProfileUpsertResult::default();
    let mut seen = BTreeSet::new();
    for draft in drafts {
        let normalized = normalize_object_name(&draft.name)?;
        if !seen.insert(normalized.clone()) {
            return Err(AppError::InvalidArgument(format!(
                "物体发现结果存在重复物体: {}",
                draft.name
            )));
        }
        let draft = normalize_draft(draft)?;
        let existing = object_profile::Entity::find()
            .filter(object_profile::Column::ProjectId.eq(project_id))
            .filter(object_profile::Column::NormalizedName.eq(&normalized))
            .one(db)
            .await
            .map_err(AppError::Database)?;

        match existing {
            Some(model) => {
                result
                    .updated
                    .push(update_object_profile_in(db, model, draft, normalized).await?);
            }
            None => {
                result
                    .created
                    .push(create_object_profile_in(db, project_id, draft, normalized).await?);
            }
        }
    }
    Ok(result)
}

async fn create_object_profile_in<C>(
    db: &C,
    project_id: i64,
    draft: ObjectProfileDraft,
    normalized_name: String,
) -> Result<object_profile::Model, AppError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    object_profile::ActiveModel {
        project_id: Set(project_id),
        name: Set(draft.name),
        normalized_name: Set(normalized_name),
        aliases_json: Set(json!(draft.aliases)),
        object_type: Set(draft.object_type),
        importance: Set(draft.importance),
        visual_description: Set(draft.visual_description),
        material_and_state: Set(draft.material_and_state),
        story_usage: Set(draft.story_usage),
        visual_usage: Set(draft.visual_usage),
        generation_usage: Set(draft.generation_usage),
        location_context: Set(draft.location_context),
        first_seen_chapter: Set(draft.first_seen_chapter),
        last_seen_chapter: Set(draft.last_seen_chapter),
        evidence_json: Set(json!(draft.evidence)),
        image_prompt_seed: Set(draft.image_prompt_seed),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

async fn update_object_profile_in<C>(
    db: &C,
    existing: object_profile::Model,
    draft: ObjectProfileDraft,
    normalized_name: String,
) -> Result<object_profile::Model, AppError>
where
    C: ConnectionTrait,
{
    let first_seen_chapter = existing.first_seen_chapter.min(draft.first_seen_chapter);
    let last_seen_chapter = existing.last_seen_chapter.max(draft.last_seen_chapter);
    let evidence_json = merge_text_set_json(existing.evidence_json.clone(), draft.evidence);
    let mut active: object_profile::ActiveModel = existing.into();
    active.name = Set(draft.name);
    active.normalized_name = Set(normalized_name);
    active.aliases_json = Set(json!(draft.aliases));
    active.object_type = Set(draft.object_type);
    active.importance = Set(draft.importance);
    active.visual_description = Set(draft.visual_description);
    active.material_and_state = Set(draft.material_and_state);
    active.story_usage = Set(draft.story_usage);
    active.visual_usage = Set(draft.visual_usage);
    active.generation_usage = Set(draft.generation_usage);
    active.location_context = Set(draft.location_context);
    active.first_seen_chapter = Set(first_seen_chapter);
    active.last_seen_chapter = Set(last_seen_chapter);
    active.evidence_json = Set(evidence_json);
    active.image_prompt_seed = Set(draft.image_prompt_seed);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(AppError::Database)
}

async fn replace_object_profile_in<C>(
    db: &C,
    existing: object_profile::Model,
    draft: ObjectProfileDraft,
    normalized_name: String,
) -> Result<object_profile::Model, AppError>
where
    C: ConnectionTrait,
{
    let mut active: object_profile::ActiveModel = existing.into();
    active.name = Set(draft.name);
    active.normalized_name = Set(normalized_name);
    active.aliases_json = Set(json!(draft.aliases));
    active.object_type = Set(draft.object_type);
    active.importance = Set(draft.importance);
    active.visual_description = Set(draft.visual_description);
    active.material_and_state = Set(draft.material_and_state);
    active.story_usage = Set(draft.story_usage);
    active.visual_usage = Set(draft.visual_usage);
    active.generation_usage = Set(draft.generation_usage);
    active.location_context = Set(draft.location_context);
    active.first_seen_chapter = Set(draft.first_seen_chapter);
    active.last_seen_chapter = Set(draft.last_seen_chapter);
    active.evidence_json = Set(json!(draft.evidence));
    active.image_prompt_seed = Set(draft.image_prompt_seed);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(AppError::Database)
}

fn object_profile_generation_response_summary(
    summary: Option<Value>,
    result: &ObjectProfileUpsertResult,
) -> Option<Value> {
    let generated = json!({
        "created_count": result.created.len(),
        "updated_count": result.updated.len(),
        "profile_ids": result
            .created
            .iter()
            .chain(result.updated.iter())
            .map(|profile| profile.id)
            .collect::<Vec<_>>(),
    });
    match summary {
        Some(Value::Object(mut map)) => {
            if let Some(generated) = generated.as_object() {
                for (key, value) in generated {
                    map.insert(key.clone(), value.clone());
                }
            }
            Some(Value::Object(map))
        }
        Some(value) => Some(json!({
            "value": value,
            "object_profiles": generated,
        })),
        None => Some(generated),
    }
}

pub fn normalize_object_name(name: &str) -> Result<String, AppError> {
    let normalized = name.trim().to_lowercase();
    if normalized.is_empty() {
        return Err(AppError::InvalidArgument("物体名称不能为空".to_string()));
    }
    Ok(normalized)
}

fn normalize_draft(mut draft: ObjectProfileDraft) -> Result<ObjectProfileDraft, AppError> {
    draft.name = required_text(&draft.name, "物体名称")?;
    draft.aliases = normalize_aliases(draft.aliases);
    draft.object_type = normalize_object_type_value(&draft.object_type)?;
    draft.importance = validate_importance(&draft.importance, "物体重要性")?;
    draft.visual_description = required_text(&draft.visual_description, "物体外观描述")?;
    draft.material_and_state = required_text(&draft.material_and_state, "物体材质与状态")?;
    draft.story_usage = required_text(&draft.story_usage, "物体叙事用途")?;
    draft.visual_usage = required_text(&draft.visual_usage, "物体视觉用途")?;
    draft.generation_usage = required_text(&draft.generation_usage, "物体生成用途")?;
    draft.location_context = draft
        .location_context
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if draft.first_seen_chapter <= 0 || draft.last_seen_chapter <= 0 {
        return Err(AppError::InvalidArgument(
            "物体首次/最近出现章节必须大于 0".to_string(),
        ));
    }
    if draft.last_seen_chapter < draft.first_seen_chapter {
        return Err(AppError::InvalidArgument(
            "物体最近出现章节不能早于首次出现章节".to_string(),
        ));
    }
    draft.evidence = normalize_required_text_set(draft.evidence, "物体证据")?;
    draft.image_prompt_seed = required_text(&draft.image_prompt_seed, "物体图片提示词种子")?;
    Ok(draft)
}

pub fn normalize_object_type_value(value: &str) -> Result<String, AppError> {
    let value = required_text(value, "物体类型")?;
    if OBJECT_TYPES.contains(&value.as_str()) {
        return Ok(value);
    }
    Err(AppError::InvalidArgument(format!(
        "物体类型无效: {value}；允许值：{}",
        OBJECT_TYPES.join("/")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_object_type_accepts_canonical_values_only() {
        assert_eq!(normalize_object_type_value(" prop ").unwrap(), "prop");
        assert_eq!(normalize_object_type_value("token").unwrap(), "token");
        assert_eq!(
            normalize_object_type_value("environment_object").unwrap(),
            "environment_object"
        );
        for alias in [
            "item",
            "equipment",
            "knife",
            "key-item",
            "vehicle object",
            "uniform",
            "furniture",
            "puddle",
            "关键道具",
        ] {
            let err =
                normalize_object_type_value(alias).expect_err("object type aliases must fail");
            assert!(
                matches!(err, AppError::InvalidArgument(message) if message.contains("物体类型无效")),
                "{alias} should be rejected"
            );
        }
    }

    #[test]
    fn normalize_object_type_rejects_unknown_values() {
        let err = normalize_object_type_value("memory")
            .expect_err("unknown object type should remain visible");
        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("物体类型无效"))
        );
    }
}
