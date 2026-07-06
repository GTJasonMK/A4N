//! 源文本同步计划与冲突决策服务。

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use a4n_core::error::AppError;
use a4n_core::models::{
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    generation::{
        generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
        generation_spec_text_anchor, prompt_artifact,
    },
    media::{media_asset, media_binding, media_provenance},
    project::{
        novel_project, project_derivation, project_source_snapshot, source_snapshot_chapter,
        source_snapshot_profile, text_anchor,
    },
    source_sync::{source_sync_conflict, source_sync_decision, source_sync_run},
    video_project::{assembly_clip, video_segment},
};
use a4n_engine::llm::provider::LlmProvider;
use a4n_engine::prompt::engine::PromptEngine;

use crate::galgame::GalgameService;
use crate::generation::{
    EntityRefInput, MediaRefInput, TextAnchorRefInput, insert_generation_entity_ref,
    insert_generation_media_ref, insert_generation_text_anchor_ref, normalize_entity_kind,
    normalize_entity_role, normalize_media_ref_role, normalize_text_anchor_role,
    normalize_video_prompt_kind, render_prompt_text, validate_entity_ref,
    validate_prompt_artifact_text, validate_required_prompt_refs,
};
use crate::project::{
    DerivedChapterScope, SOURCE_SYNC_CONFLICT, SOURCE_SYNC_CURRENT, SOURCE_SYNC_DETACHED,
    SOURCE_SYNC_ROOT_UPDATED, calculate_source_version_hash, create_source_snapshot_in,
};
use crate::project_family::load_derived_project_context;

mod decisions;
mod diff;

use decisions::{apply_decision, apply_unconflicted_items, prepare_decision_execution_context};
use diff::{
    SourceDiffService, insert_anchor_conflicts_for_run, insert_conflicts_for_change,
    insert_conflicts_for_profile_change, insert_galgame_anchor_conflicts_for_run,
    insert_galgame_conflicts_for_change,
};

pub const SYNC_RUN_MODE_DRY_RUN: &str = "dry_run";
pub const SYNC_RUN_MODE_APPLY: &str = "apply";
pub const SYNC_RUN_STATUS_PLANNED: &str = "planned";
pub const SYNC_RUN_STATUS_APPLYING: &str = "applying";
pub const SYNC_RUN_STATUS_COMPLETED: &str = "completed";
pub const CONFLICT_STATUS_PENDING: &str = "pending";
pub const CONFLICT_STATUS_RESOLVED: &str = "resolved";
pub const CONFLICT_STATUS_IGNORED: &str = "ignored";

const SYNC_RUN_MODES: &[&str] = &[SYNC_RUN_MODE_DRY_RUN, SYNC_RUN_MODE_APPLY];
pub const SYNC_RUN_STATUSES: &[&str] = &[
    SYNC_RUN_STATUS_PLANNED,
    SYNC_RUN_STATUS_APPLYING,
    SYNC_RUN_STATUS_COMPLETED,
];
pub const SOURCE_CHANGE_KINDS: &[&str] =
    &["added", "updated", "reordered", "deleted", "moved", "split"];
pub const LOCAL_STATE_KINDS: &[&str] = &[
    "user_edited",
    "generated",
    "imported",
    "assembled",
    "exported",
    "clean",
    "draft",
];
pub const CONFLICT_SEVERITIES: &[&str] = &["blocking", "warning", "info"];
pub const CONFLICT_STATUSES: &[&str] = &[
    CONFLICT_STATUS_PENDING,
    CONFLICT_STATUS_RESOLVED,
    CONFLICT_STATUS_IGNORED,
];
pub const RESOLUTION_STRATEGIES: &[&str] = &[
    "keep_local",
    "adopt_source",
    "rebuild_prompt",
    "rebind_anchor",
    "regenerate_scene",
    "create_replacement",
    "manual",
    "detach",
    "delete_local",
];
pub const DECISION_ACTORS: &[&str] = &["user", "system"];

pub fn is_valid_sync_run_mode(value: &str) -> bool {
    SYNC_RUN_MODES.contains(&value)
}

pub fn is_valid_sync_run_status(value: &str) -> bool {
    SYNC_RUN_STATUSES.contains(&value)
}

pub fn is_valid_source_change_kind(value: &str) -> bool {
    SOURCE_CHANGE_KINDS.contains(&value)
}

pub fn is_valid_local_state_kind(value: &str) -> bool {
    LOCAL_STATE_KINDS.contains(&value)
}

pub fn is_valid_conflict_severity(value: &str) -> bool {
    CONFLICT_SEVERITIES.contains(&value)
}

pub fn is_valid_conflict_status(value: &str) -> bool {
    CONFLICT_STATUSES.contains(&value)
}

pub fn is_valid_resolution_strategy(value: &str) -> bool {
    RESOLUTION_STRATEGIES.contains(&value)
}

pub fn is_valid_decision_actor(value: &str) -> bool {
    DECISION_ACTORS.contains(&value)
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceSyncPlan {
    pub run: source_sync_run::Model,
    pub conflicts: Vec<source_sync_conflict::Model>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSyncDecisionInput {
    pub conflict_id: i64,
    pub strategy: String,
    pub decision_payload_json: Value,
}

#[derive(Clone, Copy, Default)]
pub struct SourceSyncExecutionContext<'a> {
    pub galgame: Option<GalgameSourceSyncExecutionContext<'a>>,
}

#[derive(Clone, Copy)]
pub struct GalgameSourceSyncExecutionContext<'a> {
    pub service: &'a GalgameService,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub workflow_run_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RefreshSpecPayload {
    generation_spec_id: i64,
    prompt_kind: String,
    params_json: Value,
    text_anchor_refs: Vec<TextAnchorRefInput>,
    entity_refs: Vec<EntityRefInput>,
    media_refs: Vec<MediaRefInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RebindAnchorPayload {
    generation_spec_id: i64,
    anchor_mappings: Vec<AnchorMappingPayload>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnchorMappingPayload {
    old_text_anchor_id: i64,
    new_text_anchor_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManualPromptPayload {
    prompt_artifact_id: i64,
    prompt_text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplacementSpecPayload {
    generation_spec_id: i64,
    prompt_kind: String,
    params_json: Value,
    text_anchor_refs: Vec<TextAnchorRefInput>,
    entity_refs: Vec<EntityRefInput>,
    media_refs: Vec<MediaRefInput>,
    replacement_segment: ReplacementSegmentPayload,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplacementSegmentPayload {
    title: String,
    duration_secs: f32,
    order_index: i32,
    previous_segment_id: Option<i64>,
    next_segment_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegenerateGalgameScenePayload {
    galgame_scene_id: i64,
    reason: Option<String>,
}

#[derive(Clone)]
pub struct SourceSyncService {
    db: Arc<DatabaseConnection>,
}

impl SourceSyncService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn preview_source_sync(
        &self,
        derived_project_id: i64,
    ) -> Result<SourceSyncPlan, AppError> {
        let context = load_derived_project_context(self.db.as_ref(), derived_project_id).await?;
        if context.project.source_sync_status == SOURCE_SYNC_DETACHED {
            return Err(AppError::InvalidState {
                current: SOURCE_SYNC_DETACHED.to_string(),
                operation: "preview_source_sync".to_string(),
            });
        }
        let derivation = project_derivation::Entity::find()
            .filter(project_derivation::Column::DerivedProjectId.eq(derived_project_id))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("派生项目缺少派生关系: {derived_project_id}"))
            })?;
        let scope = parse_chapter_scope(&derivation.chapter_scope_json)?;
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let to_snapshot =
            create_source_snapshot_in(&tx, context.root_project.id, scope, Utc::now().naive_utc())
                .await?;
        let from_snapshot_id = context.source_snapshot.id;
        let diff =
            SourceDiffService::diff_source_snapshots(&tx, from_snapshot_id, to_snapshot.id).await?;
        let chapter_change_count = diff.chapter_changes.len();
        let profile_change_count = diff.profile_changes.len();
        let now = Utc::now().naive_utc();
        let run = source_sync_run::ActiveModel {
            derived_project_id: Set(context.project.id),
            root_project_id: Set(context.root_project.id),
            from_snapshot_id: Set(from_snapshot_id),
            to_snapshot_id: Set(to_snapshot.id),
            mode: Set(SYNC_RUN_MODE_DRY_RUN.to_string()),
            status: Set(SYNC_RUN_STATUS_PLANNED.to_string()),
            summary_json: Set(json!({
                "chapter_changes": chapter_change_count,
                "profile_changes": profile_change_count,
                "from_snapshot_id": from_snapshot_id,
                "to_snapshot_id": to_snapshot.id,
            })),
            error_message: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(AppError::Database)?;
        let mut conflicts = Vec::new();
        let mut galgame_scene_conflict_count = 0usize;
        for change in diff.chapter_changes {
            conflicts.extend(insert_conflicts_for_change(&tx, &run, &change).await?);
            let galgame_conflicts = insert_galgame_conflicts_for_change(&tx, &run, &change).await?;
            galgame_scene_conflict_count += galgame_conflicts.len();
            conflicts.extend(galgame_conflicts);
        }
        for change in diff.profile_changes {
            conflicts.extend(insert_conflicts_for_profile_change(&tx, &run, &change).await?);
        }
        let anchor_conflicts = insert_anchor_conflicts_for_run(&tx, &run).await?;
        let anchor_conflict_count = anchor_conflicts.len();
        conflicts.extend(anchor_conflicts);
        let galgame_anchor_conflicts = insert_galgame_anchor_conflicts_for_run(&tx, &run).await?;
        let galgame_anchor_conflict_count = galgame_anchor_conflicts.len();
        conflicts.extend(galgame_anchor_conflicts);
        let mut active_run: source_sync_run::ActiveModel = run.clone().into();
        active_run.summary_json = Set(json!({
            "chapter_changes": chapter_change_count,
            "profile_changes": profile_change_count,
            "anchor_conflicts": anchor_conflict_count,
            "galgame_scene_conflicts": galgame_scene_conflict_count,
            "galgame_anchor_conflicts": galgame_anchor_conflict_count,
            "conflicts": conflicts.len(),
            "from_snapshot_id": from_snapshot_id,
            "to_snapshot_id": to_snapshot.id,
        }));
        let run = active_run.update(&tx).await.map_err(AppError::Database)?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(SourceSyncPlan { run, conflicts })
    }

    pub async fn apply_source_sync_decisions(
        &self,
        sync_run_id: i64,
        decisions: Vec<SourceSyncDecisionInput>,
    ) -> Result<source_sync_run::Model, AppError> {
        self.apply_source_sync_decisions_with_context(
            sync_run_id,
            decisions,
            SourceSyncExecutionContext::default(),
        )
        .await
    }

    pub async fn apply_source_sync_decisions_with_context(
        &self,
        sync_run_id: i64,
        decisions: Vec<SourceSyncDecisionInput>,
        execution_context: SourceSyncExecutionContext<'_>,
    ) -> Result<source_sync_run::Model, AppError> {
        let run = load_source_sync_apply_run(self.db.as_ref(), sync_run_id).await?;
        let prepared_context = prepare_decision_execution_context(
            self.db.as_ref(),
            &run,
            &decisions,
            execution_context,
        )
        .await?;
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let run = load_source_sync_apply_run(&tx, sync_run_id).await?;
        let detach_requested = decisions
            .iter()
            .any(|decision| decision.strategy.trim() == "detach");
        if decisions.is_empty() {
            apply_unconflicted_items(&tx, &run).await?;
        } else {
            for decision in decisions {
                apply_decision(&tx, &run, decision, &prepared_context).await?;
            }
        }
        let pending_blocking = source_sync_conflict::Entity::find()
            .filter(source_sync_conflict::Column::SyncRunId.eq(run.id))
            .filter(source_sync_conflict::Column::Status.eq(CONFLICT_STATUS_PENDING))
            .filter(source_sync_conflict::Column::Severity.eq("blocking"))
            .count(&tx)
            .await
            .map_err(AppError::Database)?;
        let pending_any = source_sync_conflict::Entity::find()
            .filter(source_sync_conflict::Column::SyncRunId.eq(run.id))
            .filter(source_sync_conflict::Column::Status.eq(CONFLICT_STATUS_PENDING))
            .count(&tx)
            .await
            .map_err(AppError::Database)?;
        let new_status = if detach_requested {
            SOURCE_SYNC_DETACHED
        } else if pending_blocking > 0 {
            SOURCE_SYNC_CONFLICT
        } else if pending_any > 0 {
            SOURCE_SYNC_ROOT_UPDATED
        } else {
            SOURCE_SYNC_CURRENT
        };
        let mut project_update = novel_project::Entity::update_many();
        project_update = project_update
            .col_expr(
                novel_project::Column::SourceSyncStatus,
                Expr::value(new_status),
            )
            .col_expr(
                novel_project::Column::UpdatedAt,
                Expr::value(Utc::now().naive_utc()),
            )
            .filter(novel_project::Column::Id.eq(run.derived_project_id));
        if !detach_requested {
            project_update = project_update.col_expr(
                novel_project::Column::SourceSnapshotId,
                Expr::value(run.to_snapshot_id),
            );
        }
        project_update.exec(&tx).await.map_err(AppError::Database)?;
        if !detach_requested {
            project_derivation::Entity::update_many()
                .col_expr(
                    project_derivation::Column::SourceSnapshotId,
                    Expr::value(run.to_snapshot_id),
                )
                .filter(project_derivation::Column::DerivedProjectId.eq(run.derived_project_id))
                .exec(&tx)
                .await
                .map_err(AppError::Database)?;
        }
        let new_run_status = if pending_any > 0 && !detach_requested {
            SYNC_RUN_STATUS_APPLYING
        } else {
            SYNC_RUN_STATUS_COMPLETED
        };
        let mut active: source_sync_run::ActiveModel = run.into();
        active.mode = Set(SYNC_RUN_MODE_APPLY.to_string());
        active.status = Set(new_run_status.to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        let updated = active.update(&tx).await.map_err(AppError::Database)?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(updated)
    }

    pub async fn detach_derived_project(
        &self,
        derived_project_id: i64,
    ) -> Result<novel_project::Model, AppError> {
        let project = load_derived_project_context(self.db.as_ref(), derived_project_id)
            .await?
            .project;
        let mut active: novel_project::ActiveModel = project.into();
        active.source_sync_status = Set(SOURCE_SYNC_DETACHED.to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }
}

async fn load_source_sync_apply_run<C>(
    db: &C,
    sync_run_id: i64,
) -> Result<source_sync_run::Model, AppError>
where
    C: ConnectionTrait,
{
    let run = source_sync_run::Entity::find_by_id(sync_run_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("源同步运行不存在: {sync_run_id}")))?;
    if run.status != SYNC_RUN_STATUS_PLANNED && run.status != SYNC_RUN_STATUS_APPLYING {
        return Err(AppError::InvalidState {
            current: run.status,
            operation: "apply_source_sync_decisions".to_string(),
        });
    }
    let derived_project = novel_project::Entity::find_by_id(run.derived_project_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("派生项目不存在: {}", run.derived_project_id)))?;
    let expected_project_snapshot_id = if run.status == SYNC_RUN_STATUS_APPLYING {
        run.to_snapshot_id
    } else {
        run.from_snapshot_id
    };
    if derived_project.root_project_id != Some(run.root_project_id)
        || derived_project.source_snapshot_id != Some(expected_project_snapshot_id)
    {
        return Err(AppError::InvalidState {
            current: "source_sync_run_expired".to_string(),
            operation: "apply_source_sync_decisions".to_string(),
        });
    }
    if derived_project.source_sync_status == SOURCE_SYNC_DETACHED {
        return Err(AppError::InvalidState {
            current: SOURCE_SYNC_DETACHED.to_string(),
            operation: "apply_source_sync_decisions".to_string(),
        });
    }
    let derivation = project_derivation::Entity::find()
        .filter(project_derivation::Column::DerivedProjectId.eq(run.derived_project_id))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("派生项目缺少派生关系: {}", run.derived_project_id))
        })?;
    let scope = parse_chapter_scope(&derivation.chapter_scope_json)?;
    let current_hash = calculate_source_version_hash(db, run.root_project_id, scope).await?;
    let target_snapshot = project_source_snapshot::Entity::find_by_id(run.to_snapshot_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("目标源快照不存在: {}", run.to_snapshot_id)))?;
    if current_hash != target_snapshot.source_version_hash {
        return Err(AppError::InvalidState {
            current: "root_changed_after_dry_run".to_string(),
            operation: "apply_source_sync_decisions".to_string(),
        });
    }
    Ok(run)
}

fn parse_chapter_scope(value: &Value) -> Result<DerivedChapterScope, AppError> {
    let start = value
        .get("start")
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::InvalidArgument("chapter_scope_json.start 缺失".to_string()))?;
    let end = value
        .get("end")
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::InvalidArgument("chapter_scope_json.end 缺失".to_string()))?;
    if start <= 0 || end < start || start > i32::MAX as i64 || end > i32::MAX as i64 {
        return Err(AppError::InvalidArgument(
            "chapter_scope_json 范围无效".to_string(),
        ));
    }
    Ok(DerivedChapterScope {
        start_chapter: start as i32,
        end_chapter: end as i32,
    })
}

fn normalize_strategy(value: &str) -> Result<&str, AppError> {
    let value = value.trim();
    if is_valid_resolution_strategy(value) {
        Ok(value)
    } else {
        Err(AppError::InvalidArgument(format!(
            "同步冲突解决策略无效: {value}"
        )))
    }
}

fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{label} 不能为空")))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests;
