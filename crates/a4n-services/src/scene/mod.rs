//! 场景档案服务。

use crate::generation::{
    GenerationRunTraceTarget, RecordCompletedGenerationRunInput, delete_generation_run_targets_for,
    record_completed_generation_run,
};
use crate::profile_text::{
    merge_text_set_json, normalize_aliases, normalize_required_text_set, required_text,
    validate_importance,
};
use a4n_core::error::AppError;
use a4n_core::models::asset::profile_primary_material;
use a4n_core::models::chapter::chapter;
use a4n_core::models::project::novel_project;
use a4n_core::models::scene::{
    scene_entity_state, scene_occurrence, scene_profile, scene_transition,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub const SCENE_TYPES: &[&str] = &[
    "room",
    "building",
    "street",
    "city",
    "wilderness",
    "route",
    "landmark",
    "other",
];
pub const SCENE_TRANSITION_MOVEMENT_TYPES: &[&str] = &[
    "enter",
    "leave",
    "travel",
    "chase",
    "teleport",
    "hidden_passage",
    "cutaway",
];
pub const SCENE_ENTITY_TYPES: &[&str] = &["character", "object", "event"];
pub const SCENE_ENTITY_STATE_KEYS: &[&str] = &[
    "position",
    "possession",
    "condition",
    "action",
    "relation",
    "event_progress",
];

#[derive(Clone)]
pub struct SceneProfileService {
    db: Arc<DatabaseConnection>,
}

#[derive(Debug, Clone)]
pub struct SceneProfileDraft {
    pub scene_ref: String,
    pub scene_profile_id: Option<i64>,
    pub name: String,
    pub aliases: Vec<String>,
    pub scene_type: String,
    pub importance: String,
    pub parent_scene_id: Option<i64>,
    pub base_description: String,
    pub visual_description: String,
    pub spatial_layout: String,
    pub atmosphere: String,
    pub entry_points: Value,
    pub rules: Value,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    pub evidence: Vec<String>,
    pub image_prompt_seed: String,
}

#[derive(Debug, Clone)]
pub struct SceneOccurrenceDraft {
    pub occurrence_ref: String,
    pub scene_ref: String,
    pub occurrence_order: i32,
    pub timeline_position: Option<String>,
    pub summary: String,
    pub mood: Option<String>,
    pub weather_or_lighting: Option<String>,
    pub source_anchor_ids: Vec<i64>,
    pub evidence: Vec<String>,
    pub characters: Value,
    pub objects: Value,
    pub key_events: Value,
    pub entry_state: Value,
    pub exit_state: Value,
}

#[derive(Debug, Clone)]
pub struct SceneTransitionDraft {
    pub from_scene_ref: Option<String>,
    pub to_scene_ref: String,
    pub occurrence_ref: Option<String>,
    pub transition_order: i32,
    pub movement_type: String,
    pub route_description: String,
    pub time_cost: Option<String>,
    pub distance_hint: Option<String>,
    pub participants: Value,
    pub object_refs: Value,
    pub source_anchor_ids: Vec<i64>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SceneEntityStateDraft {
    pub scene_ref: String,
    pub occurrence_ref: String,
    pub entity_type: String,
    pub entity_id: Option<i64>,
    pub entity_name_snapshot: String,
    pub state_key: String,
    pub state_before: Value,
    pub state_after: Value,
    pub position_description: Option<String>,
    pub action_description: String,
    pub source_anchor_ids: Vec<i64>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SceneSyncInput {
    pub profiles: Vec<SceneProfileDraft>,
    pub occurrences: Vec<SceneOccurrenceDraft>,
    pub transitions: Vec<SceneTransitionDraft>,
    pub entity_states: Vec<SceneEntityStateDraft>,
}

#[derive(Debug, Clone, Default)]
pub struct SceneSyncResult {
    pub profile_ids: Vec<i64>,
    pub profile_ref_ids: BTreeMap<String, i64>,
    pub occurrence_count: usize,
    pub transition_count: usize,
    pub entity_state_count: usize,
}

impl SceneProfileService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn list_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<scene_profile::Model>, AppError> {
        self.ensure_project_exists(project_id).await?;
        scene_profile::Entity::find()
            .filter(scene_profile::Column::ProjectId.eq(project_id))
            .order_by_asc(scene_profile::Column::FirstSeenChapter)
            .order_by_asc(scene_profile::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn get(&self, scene_profile_id: i64) -> Result<scene_profile::Model, AppError> {
        scene_profile::Entity::find_by_id(scene_profile_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("场景档案不存在".to_string()))
    }

    pub async fn delete(&self, scene_profile_id: i64) -> Result<(), AppError> {
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let profile = scene_profile::Entity::find_by_id(scene_profile_id)
            .one(&txn)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("场景档案不存在".to_string()))?;
        scene_entity_state::Entity::delete_many()
            .filter(scene_entity_state::Column::SceneProfileId.eq(profile.id))
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;
        scene_transition::Entity::delete_many()
            .filter(
                scene_transition::Column::FromSceneProfileId
                    .eq(profile.id)
                    .or(scene_transition::Column::ToSceneProfileId.eq(profile.id)),
            )
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;
        scene_occurrence::Entity::delete_many()
            .filter(scene_occurrence::Column::SceneProfileId.eq(profile.id))
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;
        profile_primary_material::Entity::delete_many()
            .filter(profile_primary_material::Column::ProfileKind.eq("scene"))
            .filter(profile_primary_material::Column::ProfileId.eq(profile.id))
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;
        delete_generation_run_targets_for(&txn, "scene_profile", &[profile.id]).await?;
        scene_profile::Entity::delete_by_id(profile.id)
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;
        txn.commit().await.map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn create_one(
        &self,
        project_id: i64,
        draft: SceneProfileDraft,
    ) -> Result<scene_profile::Model, AppError> {
        self.ensure_project_exists(project_id).await?;
        let normalized = normalize_scene_name(&draft.name)?;
        let draft = normalize_profile_draft(draft)?;
        let existing = scene_profile::Entity::find()
            .filter(scene_profile::Column::ProjectId.eq(project_id))
            .filter(scene_profile::Column::NormalizedName.eq(&normalized))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if existing.is_some() {
            return Err(AppError::InvalidArgument(format!(
                "场景档案已存在: {}",
                draft.name
            )));
        }
        create_profile_in(self.db.as_ref(), project_id, draft, normalized).await
    }

    pub async fn update_one(
        &self,
        scene_profile_id: i64,
        draft: SceneProfileDraft,
    ) -> Result<scene_profile::Model, AppError> {
        let existing = self.get(scene_profile_id).await?;
        let normalized = normalize_scene_name(&draft.name)?;
        let draft = normalize_profile_draft(draft)?;
        let duplicate = scene_profile::Entity::find()
            .filter(scene_profile::Column::ProjectId.eq(existing.project_id))
            .filter(scene_profile::Column::NormalizedName.eq(&normalized))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if duplicate
            .as_ref()
            .is_some_and(|profile| profile.id != scene_profile_id)
        {
            return Err(AppError::InvalidArgument(format!(
                "场景档案已存在: {}",
                draft.name
            )));
        }
        replace_profile_in(self.db.as_ref(), existing, draft, normalized).await
    }

    pub async fn replace_chapter_scene_sync_with_generation_run(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        input: SceneSyncInput,
        mut generation_run: RecordCompletedGenerationRunInput,
    ) -> Result<SceneSyncResult, AppError> {
        if generation_run.project_id != project_id {
            return Err(AppError::InvalidArgument(
                "场景同步生成运行记录 project_id 与项目不一致".to_string(),
            ));
        }
        if !generation_run.targets.is_empty() {
            return Err(AppError::InvalidArgument(
                "场景同步生成运行记录目标由场景服务绑定，调用方不能预填 targets".to_string(),
            ));
        }
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let result = self
            .replace_chapter_scene_sync_in(&txn, project_id, chapter_id, chapter_number, input)
            .await?;
        generation_run.targets = result
            .profile_ids
            .iter()
            .map(|id| GenerationRunTraceTarget {
                target_kind: "scene_profile_record".to_string(),
                target_table: "scene_profile".to_string(),
                target_id: *id,
            })
            .collect();
        if !generation_run.targets.is_empty() {
            generation_run.response_summary_json = scene_sync_generation_response_summary(
                generation_run.response_summary_json,
                &result,
            );
            record_completed_generation_run(&txn, generation_run).await?;
        }
        txn.commit().await.map_err(AppError::Database)?;
        Ok(result)
    }

    async fn replace_chapter_scene_sync_in<C>(
        &self,
        db: &C,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        input: SceneSyncInput,
    ) -> Result<SceneSyncResult, AppError>
    where
        C: ConnectionTrait,
    {
        self.ensure_chapter_in(db, project_id, chapter_id, chapter_number)
            .await?;
        let old_scene_ids = scene_occurrence::Entity::find()
            .filter(scene_occurrence::Column::ProjectId.eq(project_id))
            .filter(scene_occurrence::Column::ChapterId.eq(chapter_id))
            .all(db)
            .await
            .map_err(AppError::Database)?
            .into_iter()
            .map(|occurrence| occurrence.scene_profile_id)
            .collect::<BTreeSet<_>>();

        let scene_ref_map = self
            .upsert_profiles_in(db, project_id, &input.profiles)
            .await?;
        delete_chapter_scene_records(db, project_id, chapter_id).await?;
        let occurrence_ref_map = insert_occurrences(
            db,
            project_id,
            chapter_id,
            chapter_number,
            &input.occurrences,
            &scene_ref_map,
        )
        .await?;
        let transition_count = insert_transitions(
            db,
            project_id,
            chapter_id,
            chapter_number,
            &input.transitions,
            &scene_ref_map,
            &occurrence_ref_map,
        )
        .await?;
        let entity_state_count = insert_entity_states(
            db,
            project_id,
            &input.entity_states,
            &scene_ref_map,
            &occurrence_ref_map,
        )
        .await?;

        let mut affected_scene_ids = old_scene_ids;
        affected_scene_ids.extend(scene_ref_map.values().copied());
        refresh_scene_profiles_after_occurrence_change_in(db, project_id, &affected_scene_ids)
            .await?;

        Ok(SceneSyncResult {
            profile_ids: scene_ref_map.values().copied().collect(),
            profile_ref_ids: scene_ref_map,
            occurrence_count: input.occurrences.len(),
            transition_count,
            entity_state_count,
        })
    }

    pub async fn delete_chapter_scene_sync_in<C>(
        &self,
        db: &C,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<bool, AppError>
    where
        C: ConnectionTrait,
    {
        if project_id <= 0 || chapter_id <= 0 {
            return Err(AppError::InvalidArgument(
                "场景清理 project_id/chapter_id 必须大于 0".to_string(),
            ));
        }
        let old_scene_ids = scene_occurrence::Entity::find()
            .filter(scene_occurrence::Column::ProjectId.eq(project_id))
            .filter(scene_occurrence::Column::ChapterId.eq(chapter_id))
            .all(db)
            .await
            .map_err(AppError::Database)?
            .into_iter()
            .map(|occurrence| occurrence.scene_profile_id)
            .collect::<BTreeSet<_>>();
        let old_transition_count = scene_transition::Entity::find()
            .filter(scene_transition::Column::ProjectId.eq(project_id))
            .filter(scene_transition::Column::ChapterId.eq(chapter_id))
            .count(db)
            .await
            .map_err(AppError::Database)?;
        if old_scene_ids.is_empty() && old_transition_count == 0 {
            return Ok(false);
        }

        delete_chapter_scene_records(db, project_id, chapter_id).await?;
        refresh_scene_profiles_after_occurrence_change_in(db, project_id, &old_scene_ids).await?;
        Ok(true)
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

    async fn ensure_chapter_in<C>(
        &self,
        db: &C,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        if project_id <= 0 || chapter_id <= 0 || chapter_number <= 0 {
            return Err(AppError::InvalidArgument(
                "场景同步 project_id/chapter_id/chapter_number 必须大于 0".to_string(),
            ));
        }
        let ch = chapter::Entity::find_by_id(chapter_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;
        if ch.project_id != project_id || ch.chapter_number != chapter_number {
            return Err(AppError::InvalidArgument(format!(
                "章节 #{chapter_id} 不属于项目 {project_id} 的第 {chapter_number} 章"
            )));
        }
        Ok(())
    }

    async fn upsert_profiles_in<C>(
        &self,
        db: &C,
        project_id: i64,
        drafts: &[SceneProfileDraft],
    ) -> Result<BTreeMap<String, i64>, AppError>
    where
        C: ConnectionTrait,
    {
        let mut refs = BTreeSet::new();
        let mut normalized_names = BTreeSet::new();
        let mut explicit_profile_ids = BTreeSet::new();
        let mut map = BTreeMap::new();
        for draft in drafts {
            let draft = normalize_profile_draft(draft.clone())?;
            if !refs.insert(draft.scene_ref.clone()) {
                return Err(AppError::InvalidArgument(format!(
                    "场景同步存在重复 scene_ref: {}",
                    draft.scene_ref
                )));
            }
            let scene_ref = draft.scene_ref.clone();
            let normalized = normalize_scene_name(&draft.name)?;
            if !normalized_names.insert(normalized.clone()) {
                return Err(AppError::InvalidArgument(format!(
                    "场景同步存在重复场景名称: {}",
                    draft.name
                )));
            }
            if let Some(id) = draft.scene_profile_id
                && !explicit_profile_ids.insert(id)
            {
                return Err(AppError::InvalidArgument(format!(
                    "场景同步重复引用同一场景档案: {id}"
                )));
            }
            let profile = match draft.scene_profile_id {
                Some(id) => {
                    let existing = scene_profile::Entity::find_by_id(id)
                        .one(db)
                        .await
                        .map_err(AppError::Database)?
                        .ok_or_else(|| AppError::NotFound(format!("场景档案不存在: {id}")))?;
                    if existing.project_id != project_id {
                        return Err(AppError::InvalidArgument(format!(
                            "场景档案 #{id} 不属于项目 {project_id}"
                        )));
                    }
                    update_profile_in(db, existing, draft, normalized).await?
                }
                None => {
                    match scene_profile::Entity::find()
                        .filter(scene_profile::Column::ProjectId.eq(project_id))
                        .filter(scene_profile::Column::NormalizedName.eq(&normalized))
                        .one(db)
                        .await
                        .map_err(AppError::Database)?
                    {
                        Some(existing) => {
                            update_profile_in(db, existing, draft, normalized).await?
                        }
                        None => create_profile_in(db, project_id, draft, normalized).await?,
                    }
                }
            };
            map.insert(scene_ref, profile.id);
        }
        Ok(map)
    }
}

pub fn normalize_scene_name(name: &str) -> Result<String, AppError> {
    let normalized = name.trim().to_lowercase();
    if normalized.is_empty() {
        return Err(AppError::InvalidArgument("场景名称不能为空".to_string()));
    }
    Ok(normalized)
}

fn normalize_profile_draft(mut draft: SceneProfileDraft) -> Result<SceneProfileDraft, AppError> {
    draft.scene_ref = required_text(&draft.scene_ref, "场景引用")?;
    draft.name = required_text(&draft.name, "场景名称")?;
    draft.aliases = normalize_aliases(draft.aliases);
    draft.scene_type = normalize_scene_type_value(&draft.scene_type)?;
    draft.importance = validate_importance(&draft.importance, "场景重要性")?;
    draft.base_description = required_text(&draft.base_description, "场景基础描述")?;
    draft.visual_description = required_text(&draft.visual_description, "场景视觉描述")?;
    draft.spatial_layout = required_text(&draft.spatial_layout, "场景空间布局")?;
    draft.atmosphere = required_text(&draft.atmosphere, "场景氛围")?;
    draft.image_prompt_seed = required_text(&draft.image_prompt_seed, "场景图片提示词种子")?;
    if draft.first_seen_chapter <= 0 || draft.last_seen_chapter <= 0 {
        return Err(AppError::InvalidArgument(
            "场景首次/最近出现章节必须大于 0".to_string(),
        ));
    }
    if draft.last_seen_chapter < draft.first_seen_chapter {
        return Err(AppError::InvalidArgument(
            "场景最近出现章节不能早于首次出现章节".to_string(),
        ));
    }
    draft.evidence = normalize_required_text_set(draft.evidence, "场景证据")?;
    Ok(draft)
}

pub fn normalize_scene_type_value(value: &str) -> Result<String, AppError> {
    let value = required_text(value, "场景类型")?;
    if SCENE_TYPES.contains(&value.as_str()) {
        return Ok(value);
    }
    Err(AppError::InvalidArgument(format!(
        "场景类型无效: {value}；允许值：{}",
        SCENE_TYPES.join("/")
    )))
}

async fn create_profile_in<C>(
    db: &C,
    project_id: i64,
    draft: SceneProfileDraft,
    normalized_name: String,
) -> Result<scene_profile::Model, AppError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    scene_profile::ActiveModel {
        project_id: Set(project_id),
        name: Set(draft.name),
        normalized_name: Set(normalized_name),
        aliases_json: Set(json!(draft.aliases)),
        scene_type: Set(draft.scene_type),
        importance: Set(draft.importance),
        parent_scene_id: Set(draft.parent_scene_id),
        base_description: Set(draft.base_description),
        visual_description: Set(draft.visual_description),
        spatial_layout: Set(draft.spatial_layout),
        atmosphere: Set(draft.atmosphere),
        entry_points_json: Set(normalize_json_array(draft.entry_points)),
        rules_json: Set(normalize_json_array(draft.rules)),
        current_state_json: Set(json!({})),
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

async fn update_profile_in<C>(
    db: &C,
    existing: scene_profile::Model,
    draft: SceneProfileDraft,
    normalized_name: String,
) -> Result<scene_profile::Model, AppError>
where
    C: ConnectionTrait,
{
    let first_seen_chapter = existing.first_seen_chapter.min(draft.first_seen_chapter);
    let last_seen_chapter = existing.last_seen_chapter.max(draft.last_seen_chapter);
    let evidence_json = merge_text_set_json(existing.evidence_json.clone(), draft.evidence);
    let mut active: scene_profile::ActiveModel = existing.into();
    active.name = Set(draft.name);
    active.normalized_name = Set(normalized_name);
    active.aliases_json = Set(json!(draft.aliases));
    active.scene_type = Set(draft.scene_type);
    active.importance = Set(draft.importance);
    active.parent_scene_id = Set(draft.parent_scene_id);
    active.base_description = Set(draft.base_description);
    active.visual_description = Set(draft.visual_description);
    active.spatial_layout = Set(draft.spatial_layout);
    active.atmosphere = Set(draft.atmosphere);
    active.entry_points_json = Set(normalize_json_array(draft.entry_points));
    active.rules_json = Set(normalize_json_array(draft.rules));
    active.first_seen_chapter = Set(first_seen_chapter);
    active.last_seen_chapter = Set(last_seen_chapter);
    active.evidence_json = Set(evidence_json);
    active.image_prompt_seed = Set(draft.image_prompt_seed);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(AppError::Database)
}

async fn replace_profile_in<C>(
    db: &C,
    existing: scene_profile::Model,
    draft: SceneProfileDraft,
    normalized_name: String,
) -> Result<scene_profile::Model, AppError>
where
    C: ConnectionTrait,
{
    let mut active: scene_profile::ActiveModel = existing.into();
    active.name = Set(draft.name);
    active.normalized_name = Set(normalized_name);
    active.aliases_json = Set(json!(draft.aliases));
    active.scene_type = Set(draft.scene_type);
    active.importance = Set(draft.importance);
    active.parent_scene_id = Set(draft.parent_scene_id);
    active.base_description = Set(draft.base_description);
    active.visual_description = Set(draft.visual_description);
    active.spatial_layout = Set(draft.spatial_layout);
    active.atmosphere = Set(draft.atmosphere);
    active.entry_points_json = Set(normalize_json_array(draft.entry_points));
    active.rules_json = Set(normalize_json_array(draft.rules));
    active.first_seen_chapter = Set(draft.first_seen_chapter);
    active.last_seen_chapter = Set(draft.last_seen_chapter);
    active.evidence_json = Set(json!(draft.evidence));
    active.image_prompt_seed = Set(draft.image_prompt_seed);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(AppError::Database)
}

fn normalize_json_array(value: Value) -> Value {
    if value.is_array() { value } else { json!([]) }
}

fn scene_sync_generation_response_summary(
    summary: Option<Value>,
    result: &SceneSyncResult,
) -> Option<Value> {
    let generated = json!({
        "profile_ids": result.profile_ids,
        "occurrence_count": result.occurrence_count,
        "transition_count": result.transition_count,
        "entity_state_count": result.entity_state_count,
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
            "scene_sync": generated,
        })),
        None => Some(generated),
    }
}

async fn delete_chapter_scene_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let occurrences = scene_occurrence::Entity::find()
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .filter(scene_occurrence::Column::ChapterId.eq(chapter_id))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let occurrence_ids = occurrences.iter().map(|item| item.id).collect::<Vec<_>>();
    if !occurrence_ids.is_empty() {
        scene_entity_state::Entity::delete_many()
            .filter(scene_entity_state::Column::OccurrenceId.is_in(occurrence_ids.clone()))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
        scene_transition::Entity::delete_many()
            .filter(
                scene_transition::Column::OccurrenceId
                    .is_in(occurrence_ids.clone())
                    .or(scene_transition::Column::ChapterId.eq(chapter_id)),
            )
            .exec(db)
            .await
            .map_err(AppError::Database)?;
        scene_occurrence::Entity::delete_many()
            .filter(scene_occurrence::Column::Id.is_in(occurrence_ids))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
    } else {
        scene_transition::Entity::delete_many()
            .filter(scene_transition::Column::ProjectId.eq(project_id))
            .filter(scene_transition::Column::ChapterId.eq(chapter_id))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
    }
    Ok(())
}

async fn insert_occurrences<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    drafts: &[SceneOccurrenceDraft],
    scene_ref_map: &BTreeMap<String, i64>,
) -> Result<BTreeMap<String, i64>, AppError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let mut refs = BTreeSet::new();
    let mut map = BTreeMap::new();
    for draft in drafts {
        if !refs.insert(draft.occurrence_ref.clone()) {
            return Err(AppError::InvalidArgument(format!(
                "场景同步存在重复 occurrence_ref: {}",
                draft.occurrence_ref
            )));
        }
        let scene_profile_id =
            resolve_ref(scene_ref_map, &draft.scene_ref, "occurrence.scene_ref")?;
        let occurrence = scene_occurrence::ActiveModel {
            project_id: Set(project_id),
            scene_profile_id: Set(scene_profile_id),
            chapter_id: Set(chapter_id),
            chapter_number: Set(chapter_number),
            occurrence_order: Set(validate_positive_i32(
                draft.occurrence_order,
                "occurrence_order",
            )?),
            timeline_position: Set(trimmed_optional(draft.timeline_position.as_deref())),
            summary: Set(required_text(&draft.summary, "场景出场摘要")?),
            mood: Set(trimmed_optional(draft.mood.as_deref())),
            weather_or_lighting: Set(trimmed_optional(draft.weather_or_lighting.as_deref())),
            source_span_ids_json: Set(json!(draft.source_anchor_ids)),
            evidence_json: Set(json!(draft.evidence)),
            characters_json: Set(draft.characters.clone()),
            objects_json: Set(draft.objects.clone()),
            key_events_json: Set(draft.key_events.clone()),
            entry_state_json: Set(draft.entry_state.clone()),
            exit_state_json: Set(draft.exit_state.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(AppError::Database)?;
        map.insert(draft.occurrence_ref.clone(), occurrence.id);
    }
    Ok(map)
}

async fn insert_transitions<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    drafts: &[SceneTransitionDraft],
    scene_ref_map: &BTreeMap<String, i64>,
    occurrence_ref_map: &BTreeMap<String, i64>,
) -> Result<usize, AppError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    for draft in drafts {
        let from_scene_profile_id = draft
            .from_scene_ref
            .as_deref()
            .map(|value| resolve_ref(scene_ref_map, value, "transition.from_scene_ref"))
            .transpose()?;
        let to_scene_profile_id = resolve_ref(
            scene_ref_map,
            &draft.to_scene_ref,
            "transition.to_scene_ref",
        )?;
        let occurrence_id = draft
            .occurrence_ref
            .as_deref()
            .map(|value| resolve_ref(occurrence_ref_map, value, "transition.occurrence_ref"))
            .transpose()?;
        scene_transition::ActiveModel {
            project_id: Set(project_id),
            chapter_id: Set(chapter_id),
            chapter_number: Set(chapter_number),
            from_scene_profile_id: Set(from_scene_profile_id),
            to_scene_profile_id: Set(to_scene_profile_id),
            occurrence_id: Set(occurrence_id),
            transition_order: Set(validate_positive_i32(
                draft.transition_order,
                "transition_order",
            )?),
            movement_type: Set(normalize_movement_type_value(&draft.movement_type)?),
            route_description: Set(required_text(&draft.route_description, "位移描述")?),
            time_cost: Set(trimmed_optional(draft.time_cost.as_deref())),
            distance_hint: Set(trimmed_optional(draft.distance_hint.as_deref())),
            participants_json: Set(draft.participants.clone()),
            object_refs_json: Set(draft.object_refs.clone()),
            source_span_ids_json: Set(json!(draft.source_anchor_ids)),
            evidence_json: Set(json!(draft.evidence)),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(AppError::Database)?;
    }
    Ok(drafts.len())
}

async fn insert_entity_states<C>(
    db: &C,
    project_id: i64,
    drafts: &[SceneEntityStateDraft],
    scene_ref_map: &BTreeMap<String, i64>,
    occurrence_ref_map: &BTreeMap<String, i64>,
) -> Result<usize, AppError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    for draft in drafts {
        let scene_profile_id =
            resolve_ref(scene_ref_map, &draft.scene_ref, "entity_state.scene_ref")?;
        let occurrence_id = resolve_ref(
            occurrence_ref_map,
            &draft.occurrence_ref,
            "entity_state.occurrence_ref",
        )?;
        let occurrence = scene_occurrence::Entity::find_by_id(occurrence_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "场景出场不存在: occurrence_ref={}",
                    draft.occurrence_ref
                ))
            })?;
        scene_entity_state::ActiveModel {
            project_id: Set(project_id),
            scene_profile_id: Set(scene_profile_id),
            occurrence_id: Set(occurrence_id),
            chapter_number: Set(occurrence.chapter_number),
            entity_type: Set(normalize_scene_entity_type_value(&draft.entity_type)?),
            entity_id: Set(draft.entity_id),
            entity_name_snapshot: Set(required_text(&draft.entity_name_snapshot, "实体名称快照")?),
            state_key: Set(normalize_scene_entity_state_key_value(&draft.state_key)?),
            state_before_json: Set(draft.state_before.clone()),
            state_after_json: Set(draft.state_after.clone()),
            position_description: Set(trimmed_optional(draft.position_description.as_deref())),
            action_description: Set(required_text(&draft.action_description, "实体动作描述")?),
            source_span_ids_json: Set(json!(draft.source_anchor_ids)),
            evidence_json: Set(json!(draft.evidence)),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(AppError::Database)?;
    }
    Ok(drafts.len())
}

async fn delete_orphan_scene_profile_in<C>(
    db: &C,
    project_id: i64,
    scene_profile_id: i64,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let occurrence_count = scene_occurrence::Entity::find()
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .filter(scene_occurrence::Column::SceneProfileId.eq(scene_profile_id))
        .count(db)
        .await
        .map_err(AppError::Database)?;
    if occurrence_count > 0 {
        return Ok(());
    }
    scene_entity_state::Entity::delete_many()
        .filter(scene_entity_state::Column::ProjectId.eq(project_id))
        .filter(scene_entity_state::Column::SceneProfileId.eq(scene_profile_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    scene_transition::Entity::delete_many()
        .filter(scene_transition::Column::ProjectId.eq(project_id))
        .filter(
            scene_transition::Column::FromSceneProfileId
                .eq(scene_profile_id)
                .or(scene_transition::Column::ToSceneProfileId.eq(scene_profile_id)),
        )
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    delete_generation_run_targets_for(db, "scene_profile", &[scene_profile_id]).await?;
    scene_profile::Entity::delete_many()
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .filter(scene_profile::Column::Id.eq(scene_profile_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

async fn refresh_scene_profiles_after_occurrence_change_in<C>(
    db: &C,
    project_id: i64,
    scene_profile_ids: &BTreeSet<i64>,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    for scene_id in scene_profile_ids {
        if !rebuild_scene_profile_state_in(db, project_id, *scene_id).await? {
            delete_orphan_scene_profile_in(db, project_id, *scene_id).await?;
        }
    }
    Ok(())
}

async fn rebuild_scene_profile_state_in<C>(
    db: &C,
    project_id: i64,
    scene_profile_id: i64,
) -> Result<bool, AppError>
where
    C: ConnectionTrait,
{
    let occurrences = scene_occurrence::Entity::find()
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .filter(scene_occurrence::Column::SceneProfileId.eq(scene_profile_id))
        .order_by_asc(scene_occurrence::Column::ChapterNumber)
        .order_by_asc(scene_occurrence::Column::OccurrenceOrder)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    if occurrences.is_empty() {
        return Ok(false);
    }
    let first = occurrences
        .iter()
        .map(|item| item.chapter_number)
        .min()
        .unwrap_or(1);
    let last = occurrences
        .iter()
        .map(|item| item.chapter_number)
        .max()
        .unwrap_or(first);
    let Some(latest) = occurrences.last() else {
        return Ok(false);
    };
    let current_state = json!({
        "chapter_number": latest.chapter_number,
        "occurrence_order": latest.occurrence_order,
        "summary": latest.summary,
        "mood": latest.mood,
        "weather_or_lighting": latest.weather_or_lighting,
        "entry_state": latest.entry_state_json,
        "exit_state": latest.exit_state_json,
        "characters": latest.characters_json,
        "objects": latest.objects_json,
        "key_events": latest.key_events_json,
    });
    let Some(profile) = scene_profile::Entity::find_by_id(scene_profile_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
    else {
        return Ok(false);
    };
    let mut active: scene_profile::ActiveModel = profile.into();
    active.first_seen_chapter = Set(first);
    active.last_seen_chapter = Set(last);
    active.current_state_json = Set(current_state);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(AppError::Database)?;
    Ok(true)
}

fn resolve_ref(refs: &BTreeMap<String, i64>, key: &str, field: &str) -> Result<i64, AppError> {
    refs.get(key.trim()).copied().ok_or_else(|| {
        AppError::InvalidArgument(format!("{field} 引用了不存在的场景/出场引用: {key}"))
    })
}

fn validate_positive_i32(value: i32, field: &str) -> Result<i32, AppError> {
    if value <= 0 {
        Err(AppError::InvalidArgument(format!("{field} 必须大于 0")))
    } else {
        Ok(value)
    }
}

fn trimmed_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn normalize_movement_type_value(value: &str) -> Result<String, AppError> {
    let value = required_text(value, "位移类型")?;
    if SCENE_TRANSITION_MOVEMENT_TYPES.contains(&value.as_str()) {
        return Ok(value);
    }
    Err(AppError::InvalidArgument(format!(
        "位移类型无效: {value}；允许值：{}",
        SCENE_TRANSITION_MOVEMENT_TYPES.join("/")
    )))
}

pub fn normalize_scene_entity_type_value(value: &str) -> Result<String, AppError> {
    let value = required_text(value, "实体类型")?;
    if SCENE_ENTITY_TYPES.contains(&value.as_str()) {
        return Ok(value);
    }
    Err(AppError::InvalidArgument(format!(
        "实体类型无效: {value}；允许值：{}",
        SCENE_ENTITY_TYPES.join("/")
    )))
}

pub fn normalize_scene_entity_state_key_value(value: &str) -> Result<String, AppError> {
    let value = required_text(value, "实体状态键")?;
    if SCENE_ENTITY_STATE_KEYS.contains(&value.as_str()) {
        return Ok(value);
    }
    Err(AppError::InvalidArgument(format!(
        "实体状态键无效: {value}；允许值：{}",
        SCENE_ENTITY_STATE_KEYS.join("/")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_scene_type_accepts_canonical_values_only() {
        assert_eq!(normalize_scene_type_value(" room ").unwrap(), "room");
        assert_eq!(normalize_scene_type_value("route").unwrap(), "route");
        for alias in ["corridor", "hallway", "classroom", "医院", "荒野"] {
            let err = normalize_scene_type_value(alias)
                .expect_err("scene type aliases must remain visible");
            assert!(
                matches!(err, AppError::InvalidArgument(message) if message.contains("场景类型无效")),
                "{alias} should be rejected"
            );
        }
    }

    #[test]
    fn normalize_scene_type_rejects_unknown_values() {
        let err = normalize_scene_type_value("memory")
            .expect_err("unknown scene type should remain visible");
        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("场景类型无效"))
        );
    }

    #[test]
    fn normalize_movement_type_accepts_canonical_values_only() {
        assert_eq!(normalize_movement_type_value(" travel ").unwrap(), "travel");
        assert_eq!(
            normalize_movement_type_value("hidden_passage").unwrap(),
            "hidden_passage"
        );
        for alias in ["move", "walk", "switch", "secret-passage", "进入"] {
            let err = normalize_movement_type_value(alias)
                .expect_err("movement type aliases must remain visible");
            assert!(
                matches!(err, AppError::InvalidArgument(message) if message.contains("位移类型无效")),
                "{alias} should be rejected"
            );
        }
    }

    #[test]
    fn normalize_movement_type_rejects_unknown_values() {
        let err = normalize_movement_type_value("dream_merge")
            .expect_err("unknown movement type should remain visible");
        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("位移类型无效"))
        );
    }

    #[test]
    fn normalize_scene_entity_type_accepts_canonical_values_only() {
        assert_eq!(
            normalize_scene_entity_type_value(" character ").unwrap(),
            "character"
        );
        assert_eq!(
            normalize_scene_entity_type_value("object").unwrap(),
            "object"
        );
        for alias in ["person", "item", "plot-event", "角色", "道具"] {
            let err = normalize_scene_entity_type_value(alias)
                .expect_err("entity type aliases must remain visible");
            assert!(
                matches!(err, AppError::InvalidArgument(message) if message.contains("实体类型无效")),
                "{alias} should be rejected"
            );
        }
    }

    #[test]
    fn normalize_scene_entity_type_rejects_unknown_values() {
        let err = normalize_scene_entity_type_value("memory")
            .expect_err("unknown entity type should remain visible");
        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("实体类型无效"))
        );
    }

    #[test]
    fn normalize_scene_entity_state_key_accepts_canonical_values_only() {
        assert_eq!(
            normalize_scene_entity_state_key_value(" position ").unwrap(),
            "position"
        );
        assert_eq!(
            normalize_scene_entity_state_key_value("event_progress").unwrap(),
            "event_progress"
        );
        for alias in [
            "location",
            "held-by",
            "status",
            "movement",
            "relationship",
            "plot-progress",
            "位置",
        ] {
            let err = normalize_scene_entity_state_key_value(alias)
                .expect_err("entity state key aliases must remain visible");
            assert!(
                matches!(err, AppError::InvalidArgument(message) if message.contains("实体状态键无效")),
                "{alias} should be rejected"
            );
        }
    }

    #[test]
    fn normalize_scene_entity_state_key_rejects_unknown_values() {
        let err = normalize_scene_entity_state_key_value("memory")
            .expect_err("unknown entity state key should remain visible");
        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("实体状态键无效"))
        );
    }
}
