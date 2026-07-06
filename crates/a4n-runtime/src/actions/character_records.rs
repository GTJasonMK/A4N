use crate::AppState;
use crate::actions::access;
use a4n_core::event::snapshot::SnapshotStore;
use a4n_core::models::character::{
    attribute_event, protagonist_behavior_record, protagonist_deletion_mark, protagonist_profile,
    snapshot_reference,
};
use a4n_services::character::{
    AttributeHistoryFilter, DELETION_THRESHOLD, apply_profile_current_state,
    snapshot_attributes_from_data, snapshot_store::Sha256SnapshotStore,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeCountsSummary {
    pub explicit: usize,
    pub implicit: usize,
    pub social: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtagonistProfileSummary {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub base_attributes: Value,
    pub current_state: Value,
    pub character_arc: Option<Value>,
    pub voice_fingerprint: Option<Value>,
    pub appearance: Option<String>,
    pub background: Option<String>,
    pub personality: Option<String>,
    pub last_synced_chapter: i32,
    pub attribute_counts: AttributeCountsSummary,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateProfileInput {
    pub project_id: i64,
    pub name: String,
    pub base_attributes: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListProfilesInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteProfileResultSummary {
    pub deleted_profile_rag: usize,
    pub deleted_state_rag: usize,
    pub deleted_attribute_event_rag: usize,
    pub deleted_profile_id: i64,
    pub deleted_portrait_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotReferenceSummary {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub snapshot_hash: String,
    pub reference_count: i32,
    pub attribute_counts: AttributeCountsSummary,
    pub state_summary: Option<String>,
    pub current_location: Option<String>,
    pub current_goal: Option<String>,
    pub emotional_state: Option<String>,
    pub changes_in_chapter: usize,
    pub behaviors_in_chapter: usize,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileChapterStateSummary {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub snapshot_hash: String,
    pub attribute_counts: AttributeCountsSummary,
    pub explicit_attributes: Value,
    pub implicit_attributes: Value,
    pub social_attributes: Value,
    pub state_summary: Option<String>,
    pub current_location: Option<String>,
    pub current_goal: Option<String>,
    pub emotional_state: Option<String>,
    pub changes_in_chapter: usize,
    pub behaviors_in_chapter: usize,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListSnapshotsInput {
    pub profile_id: i64,
    pub start_chapter: Option<i32>,
    pub end_chapter: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetProfileStateAtChapterInput {
    pub profile_id: i64,
    pub chapter_number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSummary {
    pub content_hash: String,
    pub entity_type: String,
    pub entity_id: i64,
    pub sequence: i64,
    pub data: Value,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetSnapshotInput {
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDeltaSummary {
    pub from_hash: String,
    pub to_hash: String,
    pub delta: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffProfileChaptersInput {
    pub profile_id: i64,
    pub from_chapter: i32,
    pub to_chapter: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSyncConflictSummary {
    pub profile_id: i64,
    pub has_conflict: bool,
    pub last_synced_chapter: i32,
    pub max_available_chapter: i32,
    pub available_snapshot_chapters: Vec<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckProfileSyncConflictInput {
    pub profile_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResultSummary {
    pub deleted_events: u64,
    pub deleted_snapshots: u64,
    pub deleted_behaviors: u64,
    pub deleted_state_indexes: u64,
    pub deleted_state_rag: usize,
    pub deleted_attribute_event_rag: usize,
    pub deleted_deletion_marks: u64,
    pub rewound_deletion_marks: u64,
    pub last_synced_chapter: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollbackToChapterInput {
    pub profile_id: i64,
    pub chapter_number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeEventSummary {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub timestamp: Option<String>,
    pub category: String,
    pub key: String,
    pub operation: String,
    pub change_description: String,
    pub old_value: Value,
    pub new_value: Value,
    pub cause: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddProfileAttributeInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub category: String,
    pub key: String,
    pub value: Value,
    pub cause: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModifyProfileAttributeInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub category: String,
    pub key: String,
    pub new_value: Value,
    pub cause: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeHistoryInput {
    pub profile_id: i64,
    pub chapter_from: Option<i32>,
    pub chapter_to: Option<i32>,
    pub category: Option<String>,
    pub key: Option<String>,
    pub operation: Option<String>,
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeMutationResultSummary {
    pub event: AttributeEventSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorRecordSummary {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub behavior_description: String,
    pub behavior_type: String,
    pub original_text: String,
    pub behavior_tags: Value,
    pub classification_results: Value,
    pub related_characters: Option<Value>,
    pub impact_analysis: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListBehaviorsInput {
    pub profile_id: i64,
    pub chapter_from: Option<i32>,
    pub chapter_to: Option<i32>,
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionMarkSummary {
    pub id: i64,
    pub profile_id: i64,
    pub category: String,
    pub key: String,
    pub chapter_number: i32,
    pub reason: String,
    pub evidence: String,
    pub consecutive_count: i32,
    pub threshold: i32,
    pub last_marked_chapter: i32,
    pub is_executed: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListDeletionMarksInput {
    pub profile_id: i64,
    pub category: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestAttributeDeletionInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub category: String,
    pub key: String,
    pub reason: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestAttributeDeletionResultSummary {
    pub mark: DeletionMarkSummary,
    pub ready_for_deletion: bool,
    pub threshold: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecuteAttributeDeletionInput {
    pub profile_id: i64,
    pub chapter_number: Option<i32>,
    pub category: String,
    pub key: String,
    pub cause: Option<String>,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteAttributeDeletionResultSummary {
    pub event: AttributeEventSummary,
    pub consecutive_count: i32,
    pub threshold: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResetDeletionMarksInput {
    pub profile_id: i64,
    pub category: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetDeletionMarksResultSummary {
    pub deleted_marks: u64,
}

pub async fn create_profile(
    state: &AppState,
    user_id: i64,
    input: CreateProfileInput,
) -> Result<ProtagonistProfileSummary, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let profile = state
        .character
        .create_profile(input.project_id, &input.name, input.base_attributes)
        .await
        .map_err(|err| err.to_string())?;
    profile_summary(&profile)
}

pub async fn list_profiles(
    state: &AppState,
    user_id: i64,
    input: ListProfilesInput,
) -> Result<Vec<ProtagonistProfileSummary>, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(input.project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    profiles.iter().map(profile_summary).collect()
}

pub async fn delete_profile(
    state: &AppState,
    user_id: i64,
    profile_id: i64,
) -> Result<DeleteProfileResultSummary, String> {
    require_profile_for_user(state, user_id, profile_id).await?;
    let deleted_profile = state
        .character
        .delete_profile(profile_id)
        .await
        .map_err(|err| err.to_string())?;

    let deleted_profile_rag = crate::workflow::delete_protagonist_rag_by_ids(
        state,
        deleted_profile.project_id,
        &[deleted_profile.profile_id],
    )
    .await?;
    let deleted_state_rag = crate::workflow::delete_character_state_rag_by_ids(
        state,
        deleted_profile.project_id,
        &deleted_profile.state_index_ids,
    )
    .await?;
    let deleted_attribute_event_rag = crate::workflow::delete_character_attribute_event_rag_by_ids(
        state,
        deleted_profile.project_id,
        &deleted_profile.attribute_event_ids,
    )
    .await?;

    let deleted_portrait_files = deleted_profile.portrait_paths.len();
    for path in deleted_profile.portrait_paths {
        crate::file_utils::remove_data_file_in_data_dir(
            &state.config.data_dir,
            &path,
            "角色立绘图片",
        )?;
    }

    Ok(DeleteProfileResultSummary {
        deleted_profile_rag,
        deleted_state_rag,
        deleted_attribute_event_rag,
        deleted_profile_id: deleted_profile.profile_id,
        deleted_portrait_files,
    })
}

pub async fn get_character_arc_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: String,
) -> Result<Option<Value>, String> {
    let profile = find_profile_by_name_for_user(state, user_id, project_id, &name).await?;
    Ok(profile.character_arc)
}

pub async fn get_character_voice_fingerprint(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: String,
) -> Result<Option<Value>, String> {
    let profile = find_profile_by_name_for_user(state, user_id, project_id, &name).await?;
    Ok(profile.voice_fingerprint)
}

pub async fn list_snapshots(
    state: &AppState,
    user_id: i64,
    input: ListSnapshotsInput,
) -> Result<Vec<SnapshotReferenceSummary>, String> {
    validate_snapshot_range(input.start_chapter, input.end_chapter)?;
    let profile = require_profile_for_user(state, user_id, input.profile_id).await?;
    let mut query = snapshot_reference::Entity::find()
        .filter(snapshot_reference::Column::ProfileId.eq(profile.id));
    if let Some(start_chapter) = input.start_chapter {
        query = query.filter(snapshot_reference::Column::ChapterNumber.gte(start_chapter));
    }
    if let Some(end_chapter) = input.end_chapter {
        query = query.filter(snapshot_reference::Column::ChapterNumber.lte(end_chapter));
    }
    let snapshots = query
        .order_by_asc(snapshot_reference::Column::ChapterNumber)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    snapshots.iter().map(snapshot_reference_summary).collect()
}

pub async fn get_profile_state_at_chapter(
    state: &AppState,
    user_id: i64,
    input: GetProfileStateAtChapterInput,
) -> Result<ProfileChapterStateSummary, String> {
    if input.chapter_number <= 0 {
        return Err("章节号必须大于 0".to_string());
    }
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let snapshot = require_profile_snapshot_ref(
        state.db.as_ref(),
        input.profile_id,
        input.chapter_number,
        "",
    )
    .await?;
    profile_state_summary_from_snapshot(&snapshot)
}

pub async fn get_snapshot(
    state: &AppState,
    user_id: i64,
    input: GetSnapshotInput,
) -> Result<SnapshotSummary, String> {
    let content_hash = required_text(&input.content_hash, "快照哈希")?;
    let snapshot = state
        .snapshot_store
        .load_snapshot(&content_hash)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("快照不存在: {content_hash}"))?;
    let profile = require_profile_for_user(state, user_id, snapshot.entity_id).await?;
    if snapshot.entity_type != "protagonist_profile" {
        return Err(format!(
            "快照类型不匹配: expected protagonist_profile, got {}",
            snapshot.entity_type
        ));
    }
    Ok(SnapshotSummary {
        content_hash: snapshot.content_hash,
        entity_type: snapshot.entity_type,
        entity_id: profile.id,
        sequence: snapshot.sequence,
        data: snapshot.data,
        created_at: Some(snapshot.created_at.to_string()),
    })
}

pub async fn diff_profile_chapters(
    state: &AppState,
    user_id: i64,
    input: DiffProfileChaptersInput,
) -> Result<SnapshotDeltaSummary, String> {
    if input.from_chapter < 0 {
        return Err("起始章节号不能小于 0".to_string());
    }
    if input.to_chapter <= 0 {
        return Err("目标章节号必须大于 0".to_string());
    }
    if input.from_chapter > input.to_chapter {
        return Err("起始章节号不能大于目标章节号".to_string());
    }
    require_profile_for_user(state, user_id, input.profile_id).await?;

    let to_ref =
        require_profile_snapshot_ref(state.db.as_ref(), input.profile_id, input.to_chapter, "")
            .await?;
    let to_snapshot = state
        .snapshot_store
        .load_snapshot(&to_ref.snapshot_hash)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("快照不存在: {}", to_ref.snapshot_hash))?;
    let (from_hash, from_data) = if input.from_chapter == 0 {
        ("initial-state".to_string(), initial_snapshot_data())
    } else {
        let from_ref = require_profile_snapshot_ref(
            state.db.as_ref(),
            input.profile_id,
            input.from_chapter,
            "",
        )
        .await?;
        let from_snapshot = state
            .snapshot_store
            .load_snapshot(&from_ref.snapshot_hash)
            .await
            .map_err(|err| err.to_string())?
            .ok_or_else(|| format!("快照不存在: {}", from_ref.snapshot_hash))?;
        (from_ref.snapshot_hash, from_snapshot.data)
    };

    Ok(SnapshotDeltaSummary {
        from_hash,
        to_hash: to_ref.snapshot_hash,
        delta: Sha256SnapshotStore::diff_snapshot_data(&from_data, &to_snapshot.data)
            .map_err(|err| err.to_string())?,
    })
}

pub async fn check_profile_sync_conflict(
    state: &AppState,
    user_id: i64,
    input: CheckProfileSyncConflictInput,
) -> Result<ProfileSyncConflictSummary, String> {
    let profile = require_profile_for_user(state, user_id, input.profile_id).await?;
    let available_chapters =
        crate::workflow::character_sync_chapters(state, profile.project_id).await?;
    let max_available_chapter = available_chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .max()
        .unwrap_or_default();
    let snapshots = snapshot_reference::Entity::find()
        .filter(snapshot_reference::Column::ProfileId.eq(input.profile_id))
        .filter(snapshot_reference::Column::ChapterNumber.lte(max_available_chapter))
        .order_by_asc(snapshot_reference::Column::ChapterNumber)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    Ok(ProfileSyncConflictSummary {
        profile_id: input.profile_id,
        has_conflict: profile.last_synced_chapter > max_available_chapter,
        last_synced_chapter: profile.last_synced_chapter,
        max_available_chapter,
        available_snapshot_chapters: snapshots
            .iter()
            .map(|snapshot| snapshot.chapter_number)
            .collect(),
    })
}

pub async fn rollback_to_chapter(
    state: &AppState,
    user_id: i64,
    input: RollbackToChapterInput,
) -> Result<RollbackResultSummary, String> {
    if input.chapter_number < 0 {
        return Err("回滚章节号不能小于 0".to_string());
    }
    let txn = state.db.begin().await.map_err(|err| err.to_string())?;
    let profile = require_profile_for_user_in(state, user_id, &txn, input.profile_id).await?;
    let project_id = profile.project_id;
    let (restored_state, cleanup_start_chapter) = if input.chapter_number == 0 {
        (profile.base_attributes.clone(), 1)
    } else {
        let target_snapshot = require_profile_snapshot_ref(
            &txn,
            input.profile_id,
            input.chapter_number,
            "，不能回滚到该章节",
        )
        .await?;
        (
            snapshot_attributes_from_data(&target_snapshot.data, input.chapter_number)
                .map_err(|err| err.to_string())?,
            input.chapter_number + 1,
        )
    };
    let deleted_timeline = state
        .character
        .delete_timeline_artifacts_from_chapter(&txn, input.profile_id, cleanup_start_chapter, None)
        .await
        .map_err(|err| err.to_string())?;
    let deleted_attribute_event_ids = deleted_timeline.attribute_event_ids.clone();
    let deleted_state_index_ids = deleted_timeline.state_index_ids.clone();
    let deletion_mark_rewind = state
        .character
        .rewind_deletion_marks_after(&txn, input.profile_id, input.chapter_number)
        .await
        .map_err(|err| err.to_string())?;
    let mut active: protagonist_profile::ActiveModel = profile.into();
    apply_profile_current_state(&mut active, restored_state);
    active.last_synced_chapter = Set(input.chapter_number);
    active.updated_at = Set(chrono::Utc::now().naive_utc());
    active.update(&txn).await.map_err(|err| err.to_string())?;
    txn.commit().await.map_err(|err| err.to_string())?;

    let deleted_state_rag = crate::workflow::delete_character_state_rag_by_ids(
        state,
        project_id,
        &deleted_state_index_ids,
    )
    .await?;
    let deleted_attribute_event_rag = crate::workflow::delete_character_attribute_event_rag_by_ids(
        state,
        project_id,
        &deleted_attribute_event_ids,
    )
    .await?;

    Ok(RollbackResultSummary {
        deleted_events: deleted_timeline.deleted_attribute_events,
        deleted_snapshots: deleted_timeline.deleted_snapshots,
        deleted_behaviors: deleted_timeline.deleted_behaviors,
        deleted_state_indexes: deleted_timeline.deleted_state_indexes,
        deleted_state_rag,
        deleted_attribute_event_rag,
        deleted_deletion_marks: deletion_mark_rewind.deleted,
        rewound_deletion_marks: deletion_mark_rewind.rewound,
        last_synced_chapter: input.chapter_number,
    })
}

pub async fn add_profile_attribute(
    state: &AppState,
    user_id: i64,
    input: AddProfileAttributeInput,
) -> Result<AttributeMutationResultSummary, String> {
    let txn = state.db.begin().await.map_err(|err| err.to_string())?;
    let profile = require_profile_for_user_in(state, user_id, &txn, input.profile_id).await?;
    let event = state
        .character
        .add_attribute_in(
            &txn,
            a4n_services::character::AttributeValueChangeInput {
                profile_id: input.profile_id,
                chapter_number: input.chapter_number,
                category: &input.category,
                key: &input.key,
                value: input.value,
                change_description: None,
                cause: &input.cause,
                evidence: &input.evidence,
            },
        )
        .await
        .map_err(|err| err.to_string())?;
    txn.commit().await.map_err(|err| err.to_string())?;
    attribute_mutation_result(state, &profile, event).await
}

pub async fn modify_profile_attribute(
    state: &AppState,
    user_id: i64,
    input: ModifyProfileAttributeInput,
) -> Result<AttributeMutationResultSummary, String> {
    let txn = state.db.begin().await.map_err(|err| err.to_string())?;
    let profile = require_profile_for_user_in(state, user_id, &txn, input.profile_id).await?;
    let event = state
        .character
        .modify_attribute_in(
            &txn,
            a4n_services::character::AttributeValueChangeInput {
                profile_id: input.profile_id,
                chapter_number: input.chapter_number,
                category: &input.category,
                key: &input.key,
                value: input.new_value,
                change_description: None,
                cause: &input.cause,
                evidence: &input.evidence,
            },
        )
        .await
        .map_err(|err| err.to_string())?;
    txn.commit().await.map_err(|err| err.to_string())?;
    attribute_mutation_result(state, &profile, event).await
}

pub async fn attribute_history(
    state: &AppState,
    user_id: i64,
    input: AttributeHistoryInput,
) -> Result<Vec<AttributeEventSummary>, String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let events = state
        .character
        .attribute_history(
            input.profile_id,
            AttributeHistoryFilter {
                chapter_from: input.chapter_from,
                chapter_to: input.chapter_to,
                category: input.category,
                key: input.key,
                operation: input.operation,
                limit: input.limit,
            },
        )
        .await
        .map_err(|err| err.to_string())?;
    Ok(events.iter().map(attribute_event_summary).collect())
}

pub async fn list_behaviors(
    state: &AppState,
    user_id: i64,
    input: ListBehaviorsInput,
) -> Result<Vec<BehaviorRecordSummary>, String> {
    let has_chapter_range = input.chapter_from.is_some() || input.chapter_to.is_some();
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let mut query = protagonist_behavior_record::Entity::find()
        .filter(protagonist_behavior_record::Column::ProfileId.eq(input.profile_id));

    if let Some(from) = input.chapter_from {
        query = query.filter(protagonist_behavior_record::Column::ChapterNumber.gte(from));
    }
    if let Some(to) = input.chapter_to {
        query = query.filter(protagonist_behavior_record::Column::ChapterNumber.lte(to));
    }
    if let Some(limit) = input.limit {
        if !(1..=100).contains(&limit) {
            return Err("行为记录 limit 必须在 1..=100 之间".to_string());
        }
        query = query.limit(limit);
    }
    let query = if input.limit.is_some() && !has_chapter_range {
        query
            .order_by_desc(protagonist_behavior_record::Column::ChapterNumber)
            .order_by_desc(protagonist_behavior_record::Column::Id)
    } else {
        query
            .order_by_asc(protagonist_behavior_record::Column::ChapterNumber)
            .order_by_asc(protagonist_behavior_record::Column::Id)
    };
    let records = query
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    Ok(records.iter().map(behavior_record_summary).collect())
}

pub async fn list_deletion_marks(
    state: &AppState,
    user_id: i64,
    input: ListDeletionMarksInput,
) -> Result<Vec<DeletionMarkSummary>, String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let mut query = protagonist_deletion_mark::Entity::find()
        .filter(protagonist_deletion_mark::Column::ProfileId.eq(input.profile_id));

    if let Some(category) = input
        .category
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !matches!(category, "explicit" | "implicit" | "social") {
            return Err("删除标记 category 必须是 explicit/implicit/social".to_string());
        }
        query = query.filter(protagonist_deletion_mark::Column::Category.eq(category));
    }

    match input
        .status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        None | Some("all") => {}
        Some("pending") => {
            query = query
                .filter(protagonist_deletion_mark::Column::IsExecuted.eq(false))
                .filter(protagonist_deletion_mark::Column::ConsecutiveCount.lt(DELETION_THRESHOLD));
        }
        Some("ready") => {
            query = query
                .filter(protagonist_deletion_mark::Column::IsExecuted.eq(false))
                .filter(
                    protagonist_deletion_mark::Column::ConsecutiveCount.gte(DELETION_THRESHOLD),
                );
        }
        Some("executed") => {
            query = query.filter(protagonist_deletion_mark::Column::IsExecuted.eq(true));
        }
        Some(_) => return Err("删除标记 status 必须是 all/pending/ready/executed".to_string()),
    }

    let marks = query
        .order_by_desc(protagonist_deletion_mark::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    Ok(marks.iter().map(deletion_mark_summary).collect())
}

pub async fn request_attribute_deletion(
    state: &AppState,
    user_id: i64,
    input: RequestAttributeDeletionInput,
) -> Result<RequestAttributeDeletionResultSummary, String> {
    let txn = state.db.begin().await.map_err(|err| err.to_string())?;
    require_profile_for_user_in(state, user_id, &txn, input.profile_id).await?;
    let mark = state
        .character
        .request_attribute_deletion_in(
            &txn,
            a4n_services::character::AttributeDeletionRequestInput {
                profile_id: input.profile_id,
                category: &input.category,
                key: &input.key,
                chapter_number: input.chapter_number,
                reason: &input.reason,
                evidence: &input.evidence,
            },
        )
        .await
        .map_err(|err| err.to_string())?;
    txn.commit().await.map_err(|err| err.to_string())?;
    Ok(RequestAttributeDeletionResultSummary {
        ready_for_deletion: !mark.is_executed && mark.consecutive_count >= DELETION_THRESHOLD,
        threshold: DELETION_THRESHOLD,
        mark: deletion_mark_summary(&mark),
    })
}

pub async fn execute_attribute_deletion(
    state: &AppState,
    user_id: i64,
    input: ExecuteAttributeDeletionInput,
) -> Result<ExecuteAttributeDeletionResultSummary, String> {
    let txn = state.db.begin().await.map_err(|err| err.to_string())?;
    let profile = require_profile_for_user_in(state, user_id, &txn, input.profile_id).await?;
    let chapter_number = required_positive_chapter_number(input.chapter_number, "执行属性删除")?;
    let cause = required_trimmed_string(input.cause, "删除原因")?;
    let (event, consecutive_count) = state
        .character
        .execute_ready_attribute_deletion_in(
            &txn,
            a4n_services::character::AttributeDeletionExecutionInput {
                profile_id: input.profile_id,
                category: &input.category,
                key: &input.key,
                event_cause: &cause,
                evidence: &input.evidence,
                chapter_number,
            },
        )
        .await
        .map_err(|err| err.to_string())?;
    txn.commit().await.map_err(|err| err.to_string())?;
    ingest_attribute_event_after_write(state, &profile, &event).await?;
    Ok(ExecuteAttributeDeletionResultSummary {
        event: attribute_event_summary(&event),
        consecutive_count,
        threshold: DELETION_THRESHOLD,
    })
}

pub async fn reset_deletion_marks(
    state: &AppState,
    user_id: i64,
    input: ResetDeletionMarksInput,
) -> Result<ResetDeletionMarksResultSummary, String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let deleted_marks = state
        .character
        .reset_deletion_marks(input.profile_id, &input.category, &input.key)
        .await
        .map_err(|err| err.to_string())?;
    Ok(ResetDeletionMarksResultSummary { deleted_marks })
}

fn attribute_counts(value: &Value) -> Result<AttributeCountsSummary, String> {
    Ok(AttributeCountsSummary {
        explicit: category_count(value, "explicit")?,
        implicit: category_count(value, "implicit")?,
        social: category_count(value, "social")?,
    })
}

fn profile_summary(p: &protagonist_profile::Model) -> Result<ProtagonistProfileSummary, String> {
    Ok(ProtagonistProfileSummary {
        id: p.id,
        project_id: p.project_id,
        name: p.name.clone(),
        gender: p.gender.clone(),
        age: p.age.clone(),
        base_attributes: p.base_attributes.clone(),
        current_state: p.current_state.clone(),
        character_arc: p.character_arc.clone(),
        voice_fingerprint: p.voice_fingerprint.clone(),
        appearance: p.appearance.clone(),
        background: p.background.clone(),
        personality: p.personality.clone(),
        last_synced_chapter: p.last_synced_chapter,
        attribute_counts: attribute_counts(&p.current_state)?,
        created_at: Some(p.created_at.to_string()),
        updated_at: Some(p.updated_at.to_string()),
    })
}

fn attribute_event_summary(event: &attribute_event::Model) -> AttributeEventSummary {
    AttributeEventSummary {
        id: event.id,
        profile_id: event.profile_id,
        chapter_number: event.chapter_number,
        timestamp: Some(event.timestamp.to_string()),
        category: event.category.clone(),
        key: event.key.clone(),
        operation: event.operation.clone(),
        change_description: event.change_description.clone(),
        old_value: event.old_value.clone(),
        new_value: event.new_value.clone(),
        cause: event.cause.clone(),
        evidence: event.evidence.clone(),
    }
}

async fn ingest_attribute_event_after_write(
    state: &AppState,
    profile: &protagonist_profile::Model,
    event: &attribute_event::Model,
) -> Result<(), String> {
    crate::workflow::ingest_character_attribute_event_rag(
        state,
        profile.project_id,
        event,
        &profile.name,
    )
    .await
    .map_err(|err| format!("角色属性事件写入后同步 RAG 失败: {err}"))
}

async fn attribute_mutation_result(
    state: &AppState,
    profile: &protagonist_profile::Model,
    event: attribute_event::Model,
) -> Result<AttributeMutationResultSummary, String> {
    ingest_attribute_event_after_write(state, profile, &event).await?;
    Ok(AttributeMutationResultSummary {
        event: attribute_event_summary(&event),
    })
}

fn behavior_record_summary(record: &protagonist_behavior_record::Model) -> BehaviorRecordSummary {
    BehaviorRecordSummary {
        id: record.id,
        profile_id: record.profile_id,
        chapter_number: record.chapter_number,
        behavior_description: record.behavior_description.clone(),
        behavior_type: record.behavior_type.clone(),
        original_text: record.original_text.clone(),
        behavior_tags: record.behavior_tags.clone(),
        classification_results: record.classification_results.clone(),
        related_characters: record.related_characters.clone(),
        impact_analysis: record.impact_analysis.clone(),
        created_at: Some(record.created_at.to_string()),
    }
}

fn deletion_mark_summary(mark: &protagonist_deletion_mark::Model) -> DeletionMarkSummary {
    DeletionMarkSummary {
        id: mark.id,
        profile_id: mark.profile_id,
        category: mark.category.clone(),
        key: mark.key.clone(),
        chapter_number: mark.chapter_number,
        reason: mark.reason.clone(),
        evidence: mark.evidence.clone(),
        consecutive_count: mark.consecutive_count,
        threshold: DELETION_THRESHOLD,
        last_marked_chapter: mark.last_marked_chapter,
        is_executed: mark.is_executed,
        created_at: mark.created_at.to_string(),
        updated_at: mark.updated_at.to_string(),
    }
}

async fn require_profile<C>(db: &C, profile_id: i64) -> Result<protagonist_profile::Model, String>
where
    C: ConnectionTrait,
{
    protagonist_profile::Entity::find_by_id(profile_id)
        .one(db)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("角色档案不存在: {profile_id}"))
}

async fn require_profile_snapshot_ref<C>(
    db: &C,
    profile_id: i64,
    chapter_number: i32,
    missing_suffix: &str,
) -> Result<snapshot_reference::Model, String>
where
    C: ConnectionTrait,
{
    snapshot_reference::Entity::find()
        .filter(snapshot_reference::Column::ProfileId.eq(profile_id))
        .filter(snapshot_reference::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("第 {chapter_number} 章没有角色快照{missing_suffix}"))
}

async fn require_profile_for_user(
    state: &AppState,
    user_id: i64,
    profile_id: i64,
) -> Result<protagonist_profile::Model, String> {
    let profile = require_profile(state.db.as_ref(), profile_id).await?;
    verify_profile_owner(state, user_id, &profile).await?;
    Ok(profile)
}

async fn require_profile_for_user_in<C>(
    state: &AppState,
    user_id: i64,
    db: &C,
    profile_id: i64,
) -> Result<protagonist_profile::Model, String>
where
    C: ConnectionTrait,
{
    let profile = require_profile(db, profile_id).await?;
    verify_profile_owner(state, user_id, &profile).await?;
    Ok(profile)
}

async fn verify_profile_owner(
    state: &AppState,
    user_id: i64,
    profile: &protagonist_profile::Model,
) -> Result<(), String> {
    access::verify_project_editor_string(state, user_id, profile.project_id).await?;
    Ok(())
}

fn required_positive_chapter_number(value: Option<i32>, label: &str) -> Result<i32, String> {
    let chapter_number = value.ok_or_else(|| format!("{label}缺少章节号"))?;
    if chapter_number <= 0 {
        return Err(format!("{label}章节号必须大于 0"));
    }
    Ok(chapter_number)
}

fn required_trimmed_string(value: Option<String>, label: &str) -> Result<String, String> {
    let text = value.ok_or_else(|| format!("{label}不能为空"))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(format!("{label}不能为空"));
    }
    Ok(trimmed.to_string())
}

async fn find_profile_by_name_for_user(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
) -> Result<protagonist_profile::Model, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let name = required_text(name, "角色名称")?;
    protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Name.eq(name.as_str()))
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("角色不存在: project_id={project_id}, name={name}"))
}

fn validate_snapshot_range(
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
) -> Result<(), String> {
    if start_chapter.is_some_and(|value| value <= 0) {
        return Err("快照起始章节必须大于 0".to_string());
    }
    if end_chapter.is_some_and(|value| value <= 0) {
        return Err("快照结束章节必须大于 0".to_string());
    }
    if let (Some(start), Some(end)) = (start_chapter, end_chapter)
        && start > end
    {
        return Err("快照起始章节不能大于结束章节".to_string());
    }
    Ok(())
}

fn initial_snapshot_data() -> Value {
    json!({
        "synced_chapter": 0,
        "state_summary": "初始空状态",
        "changes_in_chapter": 0,
        "behaviors_in_chapter": 0,
        "attributes": {
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {},
        },
        "explicit_attributes": {},
        "implicit_attributes": {},
        "social_attributes": {},
    })
}

fn category_storage_key(category: &str) -> &str {
    match category {
        "explicit" => "explicit_attributes",
        "implicit" => "implicit_attributes",
        "social" => "social_attributes",
        other => other,
    }
}

fn category_count(value: &Value, category: &str) -> Result<usize, String> {
    let key = category_storage_key(category);
    match value.get(key) {
        Some(Value::Object(object)) => Ok(object.len()),
        Some(_) => Err(format!("角色属性分类 {key} 必须是对象")),
        None => Err(format!("角色属性缺少分类 {key}")),
    }
}

fn snapshot_data_summary(s: &snapshot_reference::Model) -> Result<SnapshotDataSummary, String> {
    let data = &s.data;
    let attributes =
        snapshot_attributes_from_data(data, s.chapter_number).map_err(|err| err.to_string())?;
    let label = format!("第 {} 章角色快照", s.chapter_number);
    Ok(SnapshotDataSummary {
        attribute_counts: attribute_counts(&attributes)?,
        state_summary: value_string(data.get("state_summary")),
        current_location: value_string(data.get("current_location")),
        current_goal: value_string(data.get("current_goal")),
        emotional_state: value_string(data.get("emotional_state")),
        changes_in_chapter: required_usize_field(data, "changes_in_chapter", &label)?,
        behaviors_in_chapter: required_usize_field(data, "behaviors_in_chapter", &label)?,
    })
}

#[derive(Debug, Clone)]
struct SnapshotDataSummary {
    attribute_counts: AttributeCountsSummary,
    state_summary: Option<String>,
    current_location: Option<String>,
    current_goal: Option<String>,
    emotional_state: Option<String>,
    changes_in_chapter: usize,
    behaviors_in_chapter: usize,
}

fn snapshot_reference_summary(
    s: &snapshot_reference::Model,
) -> Result<SnapshotReferenceSummary, String> {
    let data = snapshot_data_summary(s)?;
    Ok(SnapshotReferenceSummary {
        id: s.id,
        profile_id: s.profile_id,
        chapter_number: s.chapter_number,
        snapshot_hash: s.snapshot_hash.clone(),
        reference_count: s.reference_count,
        attribute_counts: data.attribute_counts,
        state_summary: data.state_summary,
        current_location: data.current_location,
        current_goal: data.current_goal,
        emotional_state: data.emotional_state,
        changes_in_chapter: data.changes_in_chapter,
        behaviors_in_chapter: data.behaviors_in_chapter,
        created_at: Some(s.created_at.to_string()),
    })
}

fn profile_state_summary_from_snapshot(
    s: &snapshot_reference::Model,
) -> Result<ProfileChapterStateSummary, String> {
    let attributes =
        snapshot_attributes_from_data(&s.data, s.chapter_number).map_err(|err| err.to_string())?;
    let data = snapshot_data_summary(s)?;
    Ok(ProfileChapterStateSummary {
        id: s.id,
        profile_id: s.profile_id,
        chapter_number: s.chapter_number,
        snapshot_hash: s.snapshot_hash.clone(),
        attribute_counts: data.attribute_counts,
        explicit_attributes: snapshot_category_value(&attributes, "explicit")?,
        implicit_attributes: snapshot_category_value(&attributes, "implicit")?,
        social_attributes: snapshot_category_value(&attributes, "social")?,
        state_summary: data.state_summary,
        current_location: data.current_location,
        current_goal: data.current_goal,
        emotional_state: data.emotional_state,
        changes_in_chapter: data.changes_in_chapter,
        behaviors_in_chapter: data.behaviors_in_chapter,
        created_at: Some(s.created_at.to_string()),
    })
}

fn snapshot_category_value(attributes: &Value, category: &str) -> Result<Value, String> {
    let key = category_storage_key(category);
    match attributes.get(key) {
        Some(Value::Object(object)) => Ok(Value::Object(object.clone())),
        Some(_) => Err(format!("角色快照属性分类 {key} 必须是对象")),
        None => Err(format!("角色快照属性缺少分类 {key}")),
    }
}

fn required_usize_field(value: &Value, field: &str, label: &str) -> Result<usize, String> {
    let number = value
        .get(field)
        .ok_or_else(|| format!("{label}缺少字段 {field}"))?
        .as_u64()
        .ok_or_else(|| format!("{label}字段 {field} 必须是非负整数"))?;
    usize::try_from(number).map_err(|_| format!("{label}字段 {field} 超出 usize 范围"))
}

fn required_text(value: &str, label: &str) -> Result<String, String> {
    let text = value.trim();
    if text.is_empty() {
        return Err(format!("{label}不能为空"));
    }
    Ok(text.to_string())
}

fn value_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn attribute_counts_requires_canonical_category_objects() {
        let counts = attribute_counts(&json!({
            "explicit_attributes": {"appearance": "黑衣"},
            "implicit_attributes": {},
            "social_attributes": {"身份": "密探"}
        }))
        .expect("complete attribute categories should count");

        assert_eq!(counts.explicit, 1);
        assert_eq!(counts.implicit, 0);
        assert_eq!(counts.social, 1);

        let missing = attribute_counts(&json!({
            "explicit_attributes": {},
            "implicit_attributes": {}
        }))
        .expect_err("missing social attributes should fail");
        assert!(missing.contains("角色属性缺少分类 social_attributes"));

        let corrupt = attribute_counts(&json!({
            "explicit_attributes": [],
            "implicit_attributes": {},
            "social_attributes": {}
        }))
        .expect_err("corrupt explicit attributes should fail");
        assert!(corrupt.contains("角色属性分类 explicit_attributes 必须是对象"));
    }

    #[test]
    fn snapshot_data_summary_requires_explicit_chapter_counters() {
        let now = Utc::now().naive_utc();
        let mut data = json!({
            "attributes": {
                "explicit_attributes": {},
                "implicit_attributes": {},
                "social_attributes": {}
            },
            "state_summary": "潜入府衙",
            "current_location": "府衙",
            "current_goal": "寻找账册",
            "emotional_state": "冷静",
            "changes_in_chapter": 2,
            "behaviors_in_chapter": 3
        });
        let mut snapshot = snapshot_reference::Model {
            id: 10,
            profile_id: 7,
            chapter_number: 4,
            snapshot_hash: "abc123".to_string(),
            reference_count: 1,
            data: data.clone(),
            created_at: now,
        };

        let summary = snapshot_data_summary(&snapshot).expect("valid snapshot should summarize");
        assert_eq!(summary.changes_in_chapter, 2);
        assert_eq!(summary.behaviors_in_chapter, 3);
        assert_eq!(summary.current_goal, Some("寻找账册".to_string()));

        data.as_object_mut()
            .expect("object")
            .remove("changes_in_chapter");
        snapshot.data = data;
        let err =
            snapshot_data_summary(&snapshot).expect_err("missing explicit counter should fail");
        assert!(err.contains("changes_in_chapter"));
    }

    #[test]
    fn list_snapshots_input_rejects_unknown_fields() {
        let err = serde_json::from_value::<ListSnapshotsInput>(json!({
            "profile_id": 1,
            "start_chapter": null,
            "unexpected": true
        }))
        .expect_err("unknown fields should be rejected");

        assert!(err.to_string().contains("unknown field"));
    }
}
