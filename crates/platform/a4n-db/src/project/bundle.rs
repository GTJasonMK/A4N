//! Unified project-bundle history transfer.
//!
//! The regular family/canvas importers own their domain rows. This module owns
//! the cross-cutting conversation and generation history rows that must follow
//! those projects without carrying runtime ownership to another machine.
use sea_orm::ConnectionTrait;

use std::collections::{HashMap, HashSet};

use a4n_db_models::models::{
    canvas::{canvas_text_asset, canvas_text_asset_version},
    chapter::chapter,
    generation::{
        generation_run, generation_run_target, llm_call_trace, model_usage_record,
        workflow_checkpoint, workflow_progress_event, workflow_run,
    },
    project::{
        novel_project, project_source_snapshot, source_snapshot_chapter,
        standalone_comic_text_source,
    },
    user::{conversation, message},
};
use a4n_domain::{
    canvas::CanvasTextAssetRecord,
    project::{ProjectRecord, ProjectSourceMode},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::{Value, json};

#[derive(Debug, thiserror::Error)]
pub enum ProjectBundleHistoryError {
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Database(#[from] sea_orm::DbErr),
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ProjectBundleHistoryRows {
    pub conversations: Vec<conversation::Model>,
    pub messages: Vec<message::Model>,
    pub workflow_runs: Vec<workflow_run::Model>,
    pub workflow_checkpoints: Vec<workflow_checkpoint::Model>,
    pub workflow_progress_events: Vec<workflow_progress_event::Model>,
    pub generation_runs: Vec<generation_run::Model>,
    pub generation_run_targets: Vec<generation_run_target::Model>,
    pub llm_call_traces: Vec<llm_call_trace::Model>,
    pub model_usage_records: Vec<model_usage_record::Model>,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectBundleEntityIdMaps {
    pub project_ids: HashMap<i64, i64>,
    pub root_project_ids: HashMap<i64, i64>,
    pub chapter_ids: HashMap<i64, i64>,
    pub source_snapshot_ids: HashMap<i64, i64>,
    pub source_snapshot_chapter_ids: HashMap<i64, i64>,
    pub entity_ids: HashMap<String, HashMap<i64, i64>>,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectBundleHistoryImportSummary {
    pub conversations_imported: usize,
    pub messages_imported: usize,
    pub workflow_runs_imported: usize,
    pub workflow_runs_marked_interrupted: usize,
    pub generation_runs_imported: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StandaloneComicTextSourceBundleRow {
    pub source: standalone_comic_text_source::Model,
    pub chapter: chapter::Model,
    pub snapshot: project_source_snapshot::Model,
    pub snapshot_chapter: source_snapshot_chapter::Model,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StandaloneComicProjectBundle {
    pub project: ProjectRecord,
    pub text_assets: Vec<CanvasTextAssetRecord>,
    pub sources: Vec<StandaloneComicTextSourceBundleRow>,
}

#[derive(Debug, thiserror::Error)]
pub enum StandaloneComicProjectBundleError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Database(#[from] sea_orm::DbErr),
}

#[derive(Debug, Clone, Default)]
pub struct StandaloneComicProjectImportMaps {
    pub project_id: i64,
    pub entity_ids: ProjectBundleEntityIdMaps,
}

pub async fn export_standalone_comic_project_bundle(
    db: &impl crate::DbExecutor,
    project_id: i64,
) -> Result<StandaloneComicProjectBundle, StandaloneComicProjectBundleError> {
    let project = crate::project::project_by_id_record(db, project_id)
        .await
        .map_err(|error| StandaloneComicProjectBundleError::Invalid(error.to_string()))?
        .ok_or_else(|| {
            StandaloneComicProjectBundleError::NotFound(format!("项目不存在: {project_id}"))
        })?;
    if project
        .source_mode()
        .map_err(|error| StandaloneComicProjectBundleError::Invalid(error.to_string()))?
        != ProjectSourceMode::Standalone
    {
        return Err(StandaloneComicProjectBundleError::Invalid(
            "项目包只支持独立漫画来源".to_string(),
        ));
    }

    let mut text_assets = crate::canvas::canvas_text_assets_by_project_record(db, project_id)
        .await
        .map_err(|error| StandaloneComicProjectBundleError::Invalid(error.to_string()))?;
    for asset in &mut text_assets {
        asset.versions = crate::canvas::canvas_text_asset_versions_record(db, project_id, asset.id)
            .await
            .map_err(|error| StandaloneComicProjectBundleError::Invalid(error.to_string()))?;
    }

    let source_rows = standalone_comic_text_source::Entity::find()
        .filter(standalone_comic_text_source::Column::ProjectId.eq(project_id))
        .order_by_asc(standalone_comic_text_source::Column::Id)
        .all(db)
        .await?;
    let mut sources = Vec::with_capacity(source_rows.len());
    for source in source_rows {
        let (chapter, snapshot, snapshot_chapter) = futures::try_join!(
            chapter::Entity::find_by_id(source.chapter_id).one(db),
            project_source_snapshot::Entity::find_by_id(source.source_snapshot_id).one(db),
            source_snapshot_chapter::Entity::find_by_id(source.source_snapshot_chapter_id).one(db),
        )?;
        sources.push(StandaloneComicTextSourceBundleRow {
            source,
            chapter: chapter.ok_or_else(|| {
                StandaloneComicProjectBundleError::NotFound("漫画文本来源章节不存在".into())
            })?,
            snapshot: snapshot.ok_or_else(|| {
                StandaloneComicProjectBundleError::NotFound("漫画文本来源快照不存在".into())
            })?,
            snapshot_chapter: snapshot_chapter.ok_or_else(|| {
                StandaloneComicProjectBundleError::NotFound("漫画文本来源快照章节不存在".into())
            })?,
        });
    }
    let package = StandaloneComicProjectBundle {
        project,
        text_assets,
        sources,
    };
    validate_standalone_comic_project_bundle(&package)
        .map_err(StandaloneComicProjectBundleError::Invalid)?;
    Ok(package)
}

pub async fn import_standalone_comic_project_bundle(
    db: &impl crate::DbExecutor,
    package: &StandaloneComicProjectBundle,
    user_id: i64,
) -> Result<StandaloneComicProjectImportMaps, StandaloneComicProjectBundleError> {
    validate_standalone_comic_project_bundle(package)
        .map_err(StandaloneComicProjectBundleError::Invalid)?;
    if user_id <= 0 {
        return Err(StandaloneComicProjectBundleError::Invalid(
            "user_id 必须大于 0".to_string(),
        ));
    }
    let txn = db.begin().await?;
    let original = crate::project::project_record_into_model(package.project.clone());
    let now = chrono::Utc::now().naive_utc();
    let project = novel_project::ActiveModel {
        user_id: Set(user_id),
        title: Set(original.title.clone()),
        description: Set(original.description.clone()),
        project_type: Set(original.project_type.clone()),
        project_kind: Set("comic".to_string()),
        root_project_id: Set(None),
        source_snapshot_id: Set(None),
        source_sync_status: Set(crate::project::SOURCE_SYNC_CURRENT.to_string()),
        status: Set(original.status.clone()),
        target_chapters: Set(original.target_chapters),
        current_chapter: Set(0),
        total_chapters: Set(None),
        chapters_per_part: Set(None),
        avatar_svg: Set(original.avatar_svg.clone()),
        avatar_animal: Set(original.avatar_animal.clone()),
        raw_blueprint: Set(None),
        import_analysis_progress: Set(None),
        export_config: Set(original.export_config.clone()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&txn)
    .await?;

    let mut maps = ProjectBundleEntityIdMaps::default();
    maps.project_ids.insert(package.project.id, project.id);
    maps.root_project_ids.insert(package.project.id, project.id);
    let mut asset_ids = HashMap::new();
    for asset in &package.text_assets {
        let copied = canvas_text_asset::ActiveModel {
            project_id: Set(project.id),
            title: Set(asset.title.clone()),
            text_kind: Set(asset.text_kind.clone()),
            content: Set(asset.content.clone()),
            content_hash: Set(asset.content_hash.clone()),
            current_version: Set(asset.current_version),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        asset_ids.insert(asset.id, copied.id);
        maps.entity_ids
            .entry("canvas_text_asset".to_string())
            .or_default()
            .insert(asset.id, copied.id);
        for version in &asset.versions {
            canvas_text_asset_version::ActiveModel {
                canvas_text_asset_id: Set(copied.id),
                version: Set(version.version),
                content: Set(version.content.clone()),
                content_hash: Set(version.content_hash.clone()),
                created_at: Set(version.created_at),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
    }

    for row in &package.sources {
        let asset_id = *asset_ids
            .get(&row.source.canvas_text_asset_id)
            .ok_or_else(|| {
                StandaloneComicProjectBundleError::Invalid(format!(
                    "素材 ID 缺少导入映射: {}",
                    row.source.canvas_text_asset_id
                ))
            })?;
        let mut analysis_data = row
            .chapter
            .analysis_data
            .clone()
            .unwrap_or_else(|| json!({}));
        if let Some(object) = analysis_data.as_object_mut() {
            object.insert("text_asset_id".into(), json!(asset_id));
        }
        let copied_chapter = chapter::ActiveModel {
            project_id: Set(project.id),
            outline_id: Set(None),
            chapter_number: Set(row.chapter.chapter_number),
            title: Set(row.chapter.title.clone()),
            content: Set(row.chapter.content.clone()),
            word_count: Set(row.chapter.word_count),
            status: Set(chapter::STATUS_COMPLETED.to_string()),
            summary: Set(row.chapter.summary.clone()),
            style_tags: Set(row.chapter.style_tags.clone()),
            analysis_data: Set(Some(analysis_data.clone())),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        maps.chapter_ids.insert(row.chapter.id, copied_chapter.id);
        maps.entity_ids
            .entry("chapter".to_string())
            .or_default()
            .insert(row.chapter.id, copied_chapter.id);

        let copied_snapshot = project_source_snapshot::ActiveModel {
            root_project_id: Set(project.id),
            source_version_hash: Set(row.snapshot.source_version_hash.clone()),
            blueprint_version_id: Set(None),
            chapter_scope_json: Set(row.snapshot.chapter_scope_json.clone()),
            chapter_hashes_json: Set(row.snapshot.chapter_hashes_json.clone()),
            profile_hashes_json: Set(row.snapshot.profile_hashes_json.clone()),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        maps.source_snapshot_ids
            .insert(row.snapshot.id, copied_snapshot.id);

        let copied_snapshot_chapter = source_snapshot_chapter::ActiveModel {
            snapshot_id: Set(copied_snapshot.id),
            root_project_id: Set(project.id),
            source_chapter_id: Set(copied_chapter.id),
            chapter_number: Set(row.snapshot_chapter.chapter_number),
            title: Set(row.snapshot_chapter.title.clone()),
            content: Set(row.snapshot_chapter.content.clone()),
            summary: Set(row.snapshot_chapter.summary.clone()),
            analysis_data_json: Set(Some(analysis_data)),
            content_hash: Set(row.snapshot_chapter.content_hash.clone()),
            analysis_hash: Set(row.snapshot_chapter.analysis_hash.clone()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        maps.source_snapshot_chapter_ids
            .insert(row.snapshot_chapter.id, copied_snapshot_chapter.id);

        let copied_source = standalone_comic_text_source::ActiveModel {
            project_id: Set(project.id),
            chapter_id: Set(copied_chapter.id),
            source_snapshot_id: Set(copied_snapshot.id),
            source_snapshot_chapter_id: Set(copied_snapshot_chapter.id),
            canvas_text_asset_id: Set(asset_id),
            text_asset_version: Set(row.source.text_asset_version),
            start_section_index: Set(row.source.start_section_index),
            end_section_index: Set(row.source.end_section_index),
            source_title: Set(row.source.source_title.clone()),
            content_hash: Set(row.source.content_hash.clone()),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        maps.entity_ids
            .entry("standalone_comic_text_source".to_string())
            .or_default()
            .insert(row.source.id, copied_source.id);
    }
    txn.commit().await?;
    Ok(StandaloneComicProjectImportMaps {
        project_id: project.id,
        entity_ids: maps,
    })
}

pub fn validate_standalone_comic_project_bundle(
    package: &StandaloneComicProjectBundle,
) -> Result<(), String> {
    if package
        .project
        .source_mode()
        .map_err(|error| error.to_string())?
        != ProjectSourceMode::Standalone
    {
        return Err("独立漫画项目包项目来源无效".to_string());
    }
    let project_id = package.project.id;
    let assets = package
        .text_assets
        .iter()
        .map(|asset| (asset.id, asset))
        .collect::<HashMap<_, _>>();
    if assets.len() != package.text_assets.len()
        || package
            .text_assets
            .iter()
            .any(|asset| asset.project_id != project_id)
    {
        return Err("独立漫画项目包文本素材身份重复或项目不匹配".to_string());
    }
    for asset in &package.text_assets {
        let versions = asset
            .versions
            .iter()
            .map(|version| (version.version, version))
            .collect::<HashMap<_, _>>();
        if asset.id <= 0
            || asset.current_version <= 0
            || versions.len() != asset.versions.len()
            || asset.versions.iter().any(|version| {
                version.version <= 0
                    || version.content_hash != a4n_domain::source_text_hash(&version.content)
            })
        {
            return Err(format!(
                "独立漫画项目包文本素材版本无效: asset_id={}",
                asset.id
            ));
        }
        let current = versions
            .get(&asset.current_version)
            .ok_or_else(|| format!("独立漫画项目包素材缺少当前版本: asset_id={}", asset.id))?;
        if current.content != asset.content || current.content_hash != asset.content_hash {
            return Err(format!(
                "独立漫画项目包素材当前版本与正文不一致: asset_id={}",
                asset.id
            ));
        }
    }
    let mut source_ids = HashSet::new();
    for row in &package.sources {
        let source = &row.source;
        let versions = assets
            .get(&source.canvas_text_asset_id)
            .filter(|asset| asset.project_id == project_id)
            .ok_or_else(|| {
                format!(
                    "漫画来源引用了不存在的文本素材: {}",
                    source.canvas_text_asset_id
                )
            })?
            .versions
            .iter()
            .map(|version| (version.version, version))
            .collect::<HashMap<_, _>>();
        let version = versions.get(&source.text_asset_version).ok_or_else(|| {
            format!(
                "漫画来源引用了不存在的文本素材版本: {}/{}",
                source.canvas_text_asset_id, source.text_asset_version
            )
        })?;
        let (selected_content, selected_title) =
            crate::project::standalone_comic_source::select_text_sections(
                &version.content,
                source.start_section_index,
                source.end_section_index,
            )
            .map_err(|error| error.to_string())?;
        if source.project_id != project_id
            || source.chapter_id != row.chapter.id
            || source.source_snapshot_id != row.snapshot.id
            || source.source_snapshot_chapter_id != row.snapshot_chapter.id
            || row.chapter.project_id != project_id
            || row.chapter.content != selected_content
            || row.chapter.title.as_deref() != Some(selected_title.as_str())
            || row.snapshot.root_project_id != project_id
            || row.snapshot.source_version_hash != source.content_hash
            || row.snapshot_chapter.snapshot_id != row.snapshot.id
            || row.snapshot_chapter.root_project_id != project_id
            || row.snapshot_chapter.source_chapter_id != row.chapter.id
            || row.snapshot_chapter.content != row.chapter.content
            || source.content_hash != a4n_domain::source_text_hash(&row.chapter.content)
            || row.snapshot_chapter.content_hash != source.content_hash
            || !source_ids.insert((
                source.canvas_text_asset_id,
                source.text_asset_version,
                source.start_section_index,
                source.end_section_index,
            ))
        {
            return Err(format!(
                "独立漫画文本来源快照无效: chapter_id={}",
                row.chapter.id
            ));
        }
    }
    Ok(())
}

fn selected_project_scope<C: ColumnTrait>(project_id: C, project_ids: &[i64]) -> Condition {
    // Root lineage must not pull an excluded derived project's history into the bundle.
    Condition::any()
        .add(project_id.is_null())
        .add(project_id.is_in(project_ids.iter().copied()))
}

pub async fn export_project_bundle_history<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<ProjectBundleHistoryRows, ProjectBundleHistoryError>
where
    C: ConnectionTrait,
{
    let project_ids = unique_positive_ids(project_ids, "project_id")?;
    if project_ids.is_empty() {
        return Ok(ProjectBundleHistoryRows::default());
    }

    let conversations = conversation::Entity::find()
        .filter(conversation::Column::ProjectId.is_in(project_ids.clone()))
        .order_by_asc(conversation::Column::Id)
        .all(db)
        .await?;
    let conversation_ids = conversations.iter().map(|row| row.id).collect::<Vec<_>>();
    let messages = if conversation_ids.is_empty() {
        Vec::new()
    } else {
        message::Entity::find()
            .filter(message::Column::ConversationId.is_in(conversation_ids))
            .order_by_asc(message::Column::ConversationId)
            .order_by_asc(message::Column::CreatedAt)
            .order_by_asc(message::Column::Id)
            .all(db)
            .await?
    };

    let project_condition = Condition::any()
        .add(workflow_run::Column::ProjectId.is_in(project_ids.clone()))
        .add(workflow_run::Column::RootProjectId.is_in(project_ids.clone()));
    let workflow_runs = workflow_run::Entity::find()
        .filter(project_condition)
        .filter(selected_project_scope(
            workflow_run::Column::ProjectId,
            &project_ids,
        ))
        .order_by_asc(workflow_run::Column::Id)
        .all(db)
        .await?;
    let workflow_ids = workflow_runs.iter().map(|row| row.id).collect::<Vec<_>>();
    let workflow_checkpoints = if workflow_ids.is_empty() {
        Vec::new()
    } else {
        workflow_checkpoint::Entity::find()
            .filter(workflow_checkpoint::Column::WorkflowRunId.is_in(workflow_ids.clone()))
            .order_by_asc(workflow_checkpoint::Column::WorkflowRunId)
            .order_by_asc(workflow_checkpoint::Column::StepOrder)
            .order_by_asc(workflow_checkpoint::Column::Id)
            .all(db)
            .await?
    };
    let workflow_progress_events = if workflow_ids.is_empty() {
        Vec::new()
    } else {
        workflow_progress_event::Entity::find()
            .filter(workflow_progress_event::Column::WorkflowRunId.is_in(workflow_ids.clone()))
            .order_by_asc(workflow_progress_event::Column::WorkflowRunId)
            .order_by_asc(workflow_progress_event::Column::EventIndex)
            .order_by_asc(workflow_progress_event::Column::Id)
            .all(db)
            .await?
    };

    let generation_condition = Condition::any()
        .add(generation_run::Column::ProjectId.is_in(project_ids.clone()))
        .add(generation_run::Column::RootProjectId.is_in(project_ids.clone()));
    let generation_condition = if workflow_ids.is_empty() {
        generation_condition
    } else {
        generation_condition.add(generation_run::Column::WorkflowRunId.is_in(workflow_ids.clone()))
    };
    let generation_runs = generation_run::Entity::find()
        .filter(generation_condition)
        .filter(selected_project_scope(
            generation_run::Column::ProjectId,
            &project_ids,
        ))
        .order_by_asc(generation_run::Column::Id)
        .all(db)
        .await?;
    let generation_ids = generation_runs.iter().map(|row| row.id).collect::<Vec<_>>();
    let generation_run_targets = if generation_ids.is_empty() {
        Vec::new()
    } else {
        generation_run_target::Entity::find()
            .filter(generation_run_target::Column::GenerationRunId.is_in(generation_ids.clone()))
            .order_by_asc(generation_run_target::Column::GenerationRunId)
            .order_by_asc(generation_run_target::Column::Id)
            .all(db)
            .await?
    };

    let llm_trace_condition = Condition::any()
        .add(llm_call_trace::Column::ProjectId.is_in(project_ids.clone()))
        .add(llm_call_trace::Column::RootProjectId.is_in(project_ids.clone()));
    let llm_trace_condition = if workflow_ids.is_empty() {
        llm_trace_condition
    } else {
        llm_trace_condition.add(llm_call_trace::Column::WorkflowRunId.is_in(workflow_ids.clone()))
    };
    let llm_trace_condition = if generation_ids.is_empty() {
        llm_trace_condition
    } else {
        llm_trace_condition
            .add(llm_call_trace::Column::GenerationRunId.is_in(generation_ids.clone()))
    };
    let llm_call_traces = llm_call_trace::Entity::find()
        .filter(llm_trace_condition)
        .filter(selected_project_scope(
            llm_call_trace::Column::ProjectId,
            &project_ids,
        ))
        .order_by_asc(llm_call_trace::Column::Id)
        .all(db)
        .await?;

    let usage_condition = Condition::any()
        .add(model_usage_record::Column::ProjectId.is_in(project_ids.clone()))
        .add(model_usage_record::Column::RootProjectId.is_in(project_ids.clone()));
    let usage_condition = if workflow_ids.is_empty() {
        usage_condition
    } else {
        usage_condition.add(model_usage_record::Column::WorkflowRunId.is_in(workflow_ids.clone()))
    };
    let usage_condition = if generation_ids.is_empty() {
        usage_condition
    } else {
        usage_condition.add(model_usage_record::Column::GenerationRunId.is_in(generation_ids))
    };
    let model_usage_records = model_usage_record::Entity::find()
        .filter(usage_condition)
        .filter(selected_project_scope(
            model_usage_record::Column::ProjectId,
            &project_ids,
        ))
        .order_by_asc(model_usage_record::Column::Id)
        .all(db)
        .await?;

    Ok(ProjectBundleHistoryRows {
        conversations,
        messages,
        workflow_runs,
        workflow_checkpoints,
        workflow_progress_events,
        generation_runs,
        generation_run_targets,
        llm_call_traces,
        model_usage_records,
    })
}

pub async fn import_project_bundle_history(
    db: &impl crate::DbExecutor,
    rows: &ProjectBundleHistoryRows,
    maps: &ProjectBundleEntityIdMaps,
    bundle_id: &str,
    user_id: i64,
) -> Result<ProjectBundleHistoryImportSummary, ProjectBundleHistoryError> {
    let bundle_id = format!(
        "{}:{}",
        required_bundle_id(bundle_id)?,
        uuid::Uuid::new_v4()
    );
    if user_id <= 0 {
        return Err(ProjectBundleHistoryError::Invalid(
            "user_id 必须大于 0".to_string(),
        ));
    }
    let txn = db.begin().await?;
    let mut summary = ProjectBundleHistoryImportSummary::default();
    let mut conversation_ids = HashMap::new();
    for source in &rows.conversations {
        let project_id = map_required(
            &maps.project_ids,
            source.project_id,
            "conversation.project_id",
        )?;
        let mut active: conversation::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.project_id = Set(project_id);
        let inserted = active.insert(&txn).await?;
        conversation_ids.insert(source.id, inserted.id);
        summary.conversations_imported += 1;
    }
    for source in &rows.messages {
        let conversation_id = map_required(
            &conversation_ids,
            source.conversation_id,
            "message.conversation_id",
        )?;
        let mut active: message::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.conversation_id = Set(conversation_id);
        active.insert(&txn).await?;
        summary.messages_imported += 1;
    }

    let mut workflow_ids = HashMap::new();
    for source in &rows.workflow_runs {
        let project_id = source
            .project_id
            .map(|id| map_required(&maps.project_ids, id, "workflow_run.project_id"))
            .transpose()?;
        let root_project_id = source
            .root_project_id
            .map(|id| map_required(&maps.root_project_ids, id, "workflow_run.root_project_id"))
            .transpose()?;
        let chapter_id = source
            .chapter_id
            .map(|id| map_required(&maps.chapter_ids, id, "workflow_run.chapter_id"))
            .transpose()?;
        let source_snapshot_id = source
            .source_snapshot_id
            .map(|id| {
                map_required(
                    &maps.source_snapshot_ids,
                    id,
                    "workflow_run.source_snapshot_id",
                )
            })
            .transpose()?;
        let (status, can_resume, resume_block_reason, interrupted) = imported_status(
            &source.status,
            source.can_resume,
            source.resume_block_reason.clone(),
        );
        let (scope_table, scope_id, scope_is_mapped) =
            remap_workflow_scope(source.scope_table.as_deref(), source.scope_id, maps);
        let (can_resume, resume_block_reason) = if scope_is_mapped || !can_resume {
            (can_resume, resume_block_reason)
        } else {
            (
                false,
                Some("原机器工作流作用域无法在目标项目中映射；请重新提交当前步骤".to_string()),
            )
        };
        let mut active: workflow_run::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.business_key = Set(format!("{bundle_id}:{}", source.business_key));
        active.project_id = Set(project_id);
        active.root_project_id = Set(root_project_id);
        active.source_snapshot_id = Set(source_snapshot_id);
        active.scope_table = Set(scope_table);
        active.scope_id = Set(scope_id);
        active.chapter_id = Set(chapter_id);
        active.input_fingerprint_json = Set(remap_history_json(
            source.input_fingerprint_json.clone(),
            maps,
        )?);
        active.status = Set(status);
        active.active_task_id = Set(None);
        active.can_resume = Set(can_resume);
        active.resume_block_reason = Set(resume_block_reason);
        active.completed_at = Set(if can_resume {
            None
        } else {
            source.completed_at
        });
        let inserted = active.insert(&txn).await?;
        workflow_ids.insert(source.id, inserted.id);
        summary.workflow_runs_imported += 1;
        if interrupted {
            summary.workflow_runs_marked_interrupted += 1;
        }
    }

    let mut generation_ids = HashMap::new();
    for source in &rows.generation_runs {
        let project_id = map_required(
            &maps.project_ids,
            source.project_id,
            "generation_run.project_id",
        )?;
        let root_project_id = map_required(
            &maps.root_project_ids,
            source.root_project_id,
            "generation_run.root_project_id",
        )?;
        let workflow_run_id = source
            .workflow_run_id
            .map(|id| map_required(&workflow_ids, id, "generation_run.workflow_run_id"))
            .transpose()?;
        let (status, _, _, _) = imported_status(&source.status, false, None);
        let mut active: generation_run::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.project_id = Set(project_id);
        active.root_project_id = Set(root_project_id);
        active.workflow_run_id = Set(workflow_run_id);
        active.status = Set(status);
        active.request_json = Set(sanitize_runtime_json(source.request_json.clone()));
        active.raw_response_json = Set(sanitize_runtime_json(source.raw_response_json.clone()));
        active.completed_at = Set(if source.status == "running" {
            None
        } else {
            source.completed_at
        });
        let inserted = active.insert(&txn).await?;
        generation_ids.insert(source.id, inserted.id);
        summary.generation_runs_imported += 1;
    }

    for source in &rows.generation_run_targets {
        let generation_run_id = map_required(
            &generation_ids,
            source.generation_run_id,
            "generation_run_target.generation_run_id",
        )?;
        let mut active: generation_run_target::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.generation_run_id = Set(generation_run_id);
        if let Some(mapped) = history_entity_map(&source.target_table, maps)
            .and_then(|ids| ids.get(&source.target_id))
        {
            active.target_id = Set(*mapped);
        } else if !source.target_table.starts_with("source_archive:") {
            // The package can retain calls for entities it does not export.
            // Keep those source IDs in an explicit archive namespace; they must
            // never resolve to unrelated rows in the receiving database.
            active.target_table = Set(format!("source_archive:{}", source.target_table));
        }
        active.insert(&txn).await?;
    }

    for source in &rows.workflow_checkpoints {
        let workflow_run_id = map_required(
            &workflow_ids,
            source.workflow_run_id,
            "workflow_checkpoint.workflow_run_id",
        )?;
        let mut active: workflow_checkpoint::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.workflow_run_id = Set(workflow_run_id);
        if let Some(artifacts) = source.artifacts_json.as_ref().filter(|artifacts| {
            artifacts
                .get(a4n_domain::shooting_script::compat::checkpoint::TIMING_UPGRADE_KEY)
                .is_some()
        }) {
            let original = source.checkpoint_json.as_ref().ok_or_else(|| {
                ProjectBundleHistoryError::Invalid(format!(
                    "工作流断点 {} 的升级回执缺少输出",
                    source.id
                ))
            })?;
            a4n_domain::shooting_script::compat::checkpoint::validate_checkpoint_upgrade(
                original, artifacts,
            )
            .map_err(ProjectBundleHistoryError::Invalid)?;
        }
        let (input_fingerprint, fingerprint_hash) = remap_checkpoint_input(
            source.input_fingerprint_json.clone(),
            source.input_fingerprint_sha256.as_deref(),
            maps,
        )
        .map_err(|error| {
            ProjectBundleHistoryError::Invalid(format!("工作流断点 {}: {error}", source.id))
        })?;
        active.input_fingerprint_sha256 = Set(fingerprint_hash);
        active.input_fingerprint_json = Set(input_fingerprint);
        let checkpoint = remap_history_json(source.checkpoint_json.clone(), maps)?;
        let mut artifacts = remap_history_json(source.artifacts_json.clone(), maps)?;
        if let Some(receipt) = artifacts.as_mut().and_then(|value| {
            value.get_mut(a4n_domain::shooting_script::compat::checkpoint::TIMING_UPGRADE_KEY)
        }) {
            receipt["upgraded_input_fingerprint_sha256"] = json!(
                crate::generation::generation_checkpoint_fingerprint_sha256(
                    &receipt["upgraded_input_fingerprint_json"]
                )
                .map_err(|e| ProjectBundleHistoryError::Invalid(e.to_string()))?
            );
            receipt["upgraded_parsed_output_sha256"] = json!(
                crate::generation::generation_checkpoint_fingerprint_sha256(
                    &checkpoint
                        .as_ref()
                        .ok_or_else(|| ProjectBundleHistoryError::Invalid(
                            "升级断点缺少结构化输出".into()
                        ))?["parsed_output"]
                )
                .map_err(|e| ProjectBundleHistoryError::Invalid(e.to_string()))?
            );
        }
        active.checkpoint_json = Set(checkpoint);
        active.artifacts_json = Set(artifacts);
        active.generation_run_ids_json = Set(remap_generation_ids(
            source.generation_run_ids_json.clone(),
            &generation_ids,
        )?);
        active.status = Set(if source.status == "running" {
            "interrupted".to_string()
        } else {
            source.status.clone()
        });
        active.completed_at = Set(if source.status == "running" {
            None
        } else {
            source.completed_at
        });
        active.insert(&txn).await?;
    }
    for source in &rows.workflow_progress_events {
        let workflow_run_id = map_required(
            &workflow_ids,
            source.workflow_run_id,
            "workflow_progress_event.workflow_run_id",
        )?;
        let mut active: workflow_progress_event::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.workflow_run_id = Set(workflow_run_id);
        active.task_id = Set(None);
        active.meta_json = Set(remap_history_json(source.meta_json.clone(), maps)?);
        active.insert(&txn).await?;
    }
    for source in &rows.llm_call_traces {
        let mut active: llm_call_trace::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.project_id = Set(source
            .project_id
            .map(|id| map_required(&maps.project_ids, id, "llm_call_trace.project_id"))
            .transpose()?);
        active.root_project_id = Set(source
            .root_project_id
            .map(|id| map_required(&maps.root_project_ids, id, "llm_call_trace.root_project_id"))
            .transpose()?);
        active.workflow_run_id = Set(source
            .workflow_run_id
            .map(|id| map_required(&workflow_ids, id, "llm_call_trace.workflow_run_id"))
            .transpose()?);
        active.generation_run_id = Set(source
            .generation_run_id
            .map(|id| map_required(&generation_ids, id, "llm_call_trace.generation_run_id"))
            .transpose()?);
        active.context_plan_json = Set(sanitize_runtime_json(source.context_plan_json.clone()));
        active.context_pack_refs_json =
            Set(sanitize_runtime_json(source.context_pack_refs_json.clone()));
        active.insert(&txn).await?;
    }
    for source in &rows.model_usage_records {
        let mut active: model_usage_record::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.request_id = Set(uuid::Uuid::new_v4().to_string());
        active.user_id = Set(Some(user_id));
        active.project_id = Set(source
            .project_id
            .map(|id| map_required(&maps.project_ids, id, "model_usage_record.project_id"))
            .transpose()?);
        active.root_project_id = Set(source
            .root_project_id
            .map(|id| {
                map_required(
                    &maps.root_project_ids,
                    id,
                    "model_usage_record.root_project_id",
                )
            })
            .transpose()?);
        active.workflow_run_id = Set(source
            .workflow_run_id
            .map(|id| map_required(&workflow_ids, id, "model_usage_record.workflow_run_id"))
            .transpose()?);
        active.generation_run_id = Set(source
            .generation_run_id
            .map(|id| map_required(&generation_ids, id, "model_usage_record.generation_run_id"))
            .transpose()?);
        active.provider_config_id = Set(None);
        active.pricing_snapshot_json = Set(sanitize_runtime_json(Some(
            source.pricing_snapshot_json.clone(),
        ))
        .unwrap_or_else(|| json!({"imported": true})));
        active.insert(&txn).await?;
    }
    txn.commit().await?;
    Ok(summary)
}

fn unique_positive_ids(ids: &[i64], field: &str) -> Result<Vec<i64>, ProjectBundleHistoryError> {
    let mut result = Vec::with_capacity(ids.len());
    let mut seen = HashSet::with_capacity(ids.len());
    for id in ids {
        if *id <= 0 {
            return Err(ProjectBundleHistoryError::Invalid(format!(
                "{field} 必须大于 0"
            )));
        }
        if seen.insert(*id) {
            result.push(*id);
        }
    }
    Ok(result)
}

fn map_required(
    map: &HashMap<i64, i64>,
    old_id: i64,
    field: &str,
) -> Result<i64, ProjectBundleHistoryError> {
    map.get(&old_id).copied().ok_or_else(|| {
        ProjectBundleHistoryError::Invalid(format!("{field} 缺少旧 ID {old_id} 的导入映射"))
    })
}

fn required_bundle_id(value: &str) -> Result<String, ProjectBundleHistoryError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 128 || value.contains(':') {
        return Err(ProjectBundleHistoryError::Invalid(
            "bundle_id 必须是 1-128 个不含冒号的字符".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn remap_workflow_scope(
    scope_table: Option<&str>,
    scope_id: Option<i64>,
    maps: &ProjectBundleEntityIdMaps,
) -> (Option<String>, Option<i64>, bool) {
    let Some(scope_table) = scope_table else {
        return (None, None, scope_id.is_none());
    };
    let Some(scope_id) = scope_id else {
        return (Some(scope_table.to_string()), None, true);
    };
    let mapped = match scope_table {
        "project" | "novel_project" => maps.project_ids.get(&scope_id).copied(),
        "chapter" | "chapter_shooting_script" => maps.chapter_ids.get(&scope_id).copied(),
        "source_snapshot" => maps.source_snapshot_ids.get(&scope_id).copied(),
        table => maps
            .entity_ids
            .get(table)
            .and_then(|ids| ids.get(&scope_id))
            .copied(),
    };
    match mapped {
        Some(mapped) => (Some(scope_table.to_string()), Some(mapped), true),
        None => (None, None, false),
    }
}

fn imported_status(
    status: &str,
    can_resume: bool,
    resume_block_reason: Option<String>,
) -> (String, bool, Option<String>, bool) {
    if matches!(status, "running" | "interrupted") {
        // A migration or source boundary can explicitly prohibit resuming. An
        // import must not turn that prohibition into runtime-restart recovery.
        let resumable = can_resume || resume_block_reason.is_none();
        return (
            "interrupted".to_string(),
            resumable,
            resume_block_reason.or_else(|| {
                Some("原机器的运行时任务不能跨机器接管；恢复时将重新提交当前步骤".to_string())
            }),
            status == "running",
        );
    }
    (status.to_string(), can_resume, resume_block_reason, false)
}

fn sanitize_runtime_json(value: Option<Value>) -> Option<Value> {
    let mut value = value?;
    a4n_domain::clear_canvas_transfer_runtime_identity(&mut value);
    Some(value)
}

fn remap_checkpoint_input(
    value: Option<Value>,
    stored_hash: Option<&str>,
    maps: &ProjectBundleEntityIdMaps,
) -> Result<(Option<Value>, Option<String>), ProjectBundleHistoryError> {
    let Some(mut value) = value else {
        if stored_hash.is_some() {
            return Err(ProjectBundleHistoryError::Invalid(
                "调用断点有指纹哈希但缺少原始输入".into(),
            ));
        }
        return Ok((None, None));
    };
    if let Some(stored_hash) = stored_hash {
        let actual = crate::generation::generation_checkpoint_fingerprint_sha256(&value)
            .map_err(|error| ProjectBundleHistoryError::Invalid(error.to_string()))?;
        if actual != stored_hash {
            return Err(ProjectBundleHistoryError::Invalid(
                "调用断点原始输入指纹不一致，不能通过重新计算修复".into(),
            ));
        }
    }
    remap_history_value(&mut value, maps)?;
    // Unfingerprinted history must not become a reusable completed call during import.
    let hash = stored_hash
        .map(|_| crate::generation::generation_checkpoint_fingerprint_sha256(&value))
        .transpose()
        .map_err(|error| ProjectBundleHistoryError::Invalid(error.to_string()))?;
    Ok((Some(value), hash))
}

fn remap_history_json(
    value: Option<Value>,
    maps: &ProjectBundleEntityIdMaps,
) -> Result<Option<Value>, ProjectBundleHistoryError> {
    let Some(mut value) = value else {
        return Ok(None);
    };
    remap_history_value(&mut value, maps)?;
    Ok(Some(value))
}

fn history_entity_map<'a>(
    table: &str,
    maps: &'a ProjectBundleEntityIdMaps,
) -> Option<&'a HashMap<i64, i64>> {
    match table {
        "project" | "novel_project" => Some(&maps.project_ids),
        "chapter" => Some(&maps.chapter_ids),
        "source_snapshot" | "project_source_snapshot" => Some(&maps.source_snapshot_ids),
        "source_snapshot_chapter" => Some(&maps.source_snapshot_chapter_ids),
        table => maps.entity_ids.get(table),
    }
}

fn remap_history_value(
    value: &mut Value,
    maps: &ProjectBundleEntityIdMaps,
) -> Result<(), ProjectBundleHistoryError> {
    match value {
        Value::Object(object) => {
            for (key, value) in object.iter_mut() {
                if matches!(
                    key.as_str(),
                    "original_input_fingerprint_json" | "original_checkpoint"
                ) {
                    continue;
                }
                if let Some(old_id) = value.as_i64() {
                    let mapping = match key.as_str() {
                        "project_id" => Some((&maps.project_ids, "history.project_id")),
                        "root_project_id" => {
                            Some((&maps.root_project_ids, "history.root_project_id"))
                        }
                        "chapter_id" | "source_chapter_id" => {
                            Some((&maps.chapter_ids, "history.chapter_id"))
                        }
                        "source_snapshot_id" => {
                            Some((&maps.source_snapshot_ids, "history.source_snapshot_id"))
                        }
                        "source_snapshot_chapter_id" => Some((
                            &maps.source_snapshot_chapter_ids,
                            "history.source_snapshot_chapter_id",
                        )),
                        key => {
                            let table = match key {
                                "visual_plan_id" => "chapter_visual_plan",
                                "shooting_script_id" => "chapter_shooting_script",
                                "adaptation_plan_id"
                                | "branch_plan_id"
                                | "authoring_document_id" => "media_authoring_document",
                                "galgame_scene_id" => "galgame_scene",
                                "galgame_line_id" => "galgame_line",
                                "galgame_script_id" => "galgame_script",
                                "text_anchor_id" => "text_anchor",
                                "scene_profile_id" => "scene_profile",
                                "speaker_profile_id" | "character_profile_id" => {
                                    "protagonist_profile"
                                }
                                _ => "",
                            };
                            maps.entity_ids
                                .get(table)
                                .map(|ids| (ids, "history.entity_id"))
                        }
                    };
                    if let Some((map, field)) = mapping {
                        *value = Value::from(map_required(map, old_id, field)?);
                        continue;
                    }
                }
                remap_history_value(value, maps)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                remap_history_value(value, maps)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn remap_generation_ids(
    value: Option<Value>,
    maps: &HashMap<i64, i64>,
) -> Result<Option<Value>, ProjectBundleHistoryError> {
    let Some(mut value) = value else {
        return Ok(None);
    };
    if let Some(items) = value.as_array_mut() {
        for item in items {
            if let Some(old_id) = item.as_i64() {
                *item = Value::from(map_required(
                    maps,
                    old_id,
                    "workflow_checkpoint.generation_run_ids_json",
                )?);
            }
        }
    }
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        ProjectBundleEntityIdMaps, imported_status, remap_checkpoint_input, remap_workflow_scope,
    };

    #[test]
    fn checkpoint_remap_verifies_original_hash_without_inventing_one() {
        let maps = ProjectBundleEntityIdMaps {
            chapter_ids: HashMap::from([(7, 70)]),
            ..Default::default()
        };
        let input = serde_json::json!({"chapter_id": 7, "task": "kept"});
        let hash = crate::generation::generation_checkpoint_fingerprint_sha256(&input).unwrap();
        let (mapped, mapped_hash) =
            remap_checkpoint_input(Some(input.clone()), Some(&hash), &maps).unwrap();
        let expected = serde_json::json!({"chapter_id": 70, "task": "kept"});
        assert_eq!(mapped, Some(expected.clone()));
        assert_eq!(
            mapped_hash,
            Some(crate::generation::generation_checkpoint_fingerprint_sha256(&expected).unwrap())
        );
        let mut corrupt = input.clone();
        corrupt["task"] = serde_json::json!("changed without updating hash");
        assert!(
            remap_checkpoint_input(Some(corrupt), Some(&hash), &maps)
                .unwrap_err()
                .to_string()
                .contains("原始输入指纹不一致")
        );
        assert_eq!(
            remap_checkpoint_input(Some(input), None, &maps).unwrap(),
            (Some(expected), None)
        );
        assert!(remap_checkpoint_input(None, Some(&hash), &maps).is_err());
    }

    #[tokio::test]
    async fn history_project_scope_excludes_unselected_descendants() {
        use super::{selected_project_scope, workflow_run};
        use sea_orm::{
            ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
            QuerySelect, TransactionTrait,
        };
        let db = a4n_migration::test_support::connect_test_database().await;
        let tx = db.begin().await.unwrap();
        tx.execute_unprepared(r#"
            CREATE TEMP TABLE workflow_run (id bigint, project_id bigint, root_project_id bigint) ON COMMIT DROP;
            INSERT INTO workflow_run VALUES (1, 10, 10), (2, 20, 10), (3, NULL, 10), (4, 30, 30);
        "#).await.unwrap();
        for (projects, expected) in [(vec![10], vec![1, 3]), (vec![10, 20], vec![1, 2, 3])] {
            let rows = workflow_run::Entity::find()
                .select_only()
                .column(workflow_run::Column::Id)
                .filter(
                    Condition::any()
                        .add(workflow_run::Column::ProjectId.is_in(projects.clone()))
                        .add(workflow_run::Column::RootProjectId.is_in(projects.clone())),
                )
                .filter(selected_project_scope(
                    workflow_run::Column::ProjectId,
                    &projects,
                ))
                .order_by_asc(workflow_run::Column::Id)
                .into_tuple::<i64>()
                .all(&tx)
                .await
                .unwrap();
            assert_eq!(rows, expected);
        }
        tx.rollback().await.unwrap();
    }

    #[test]
    fn running_history_is_imported_as_resumable_interrupted() {
        let (status, can_resume, reason, interrupted) = imported_status("running", false, None);
        assert_eq!(status, "interrupted");
        assert!(can_resume);
        assert!(reason.is_some());
        assert!(interrupted);
        let blocked = Some("旧媒体依赖已经退役".to_string());
        let (status, can_resume, reason, _) =
            imported_status("interrupted", false, blocked.clone());
        assert_eq!(status, "interrupted");
        assert!(!can_resume);
        assert_eq!(reason, blocked);
    }

    #[test]
    fn workflow_scope_is_remapped_or_blocked_explicitly() {
        let maps = ProjectBundleEntityIdMaps {
            chapter_ids: HashMap::from([(7, 70)]),
            ..Default::default()
        };
        assert_eq!(
            remap_workflow_scope(Some("chapter"), Some(7), &maps),
            (Some("chapter".to_string()), Some(70), true)
        );
        assert_eq!(
            remap_workflow_scope(Some("canvas_graph"), Some(7), &maps),
            (None, None, false)
        );
    }
}
