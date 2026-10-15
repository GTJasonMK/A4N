use a4n_db::auth::create_user_summary;
use a4n_db::canvas::{canvas_text_asset_versions_record, canvas_text_assets_by_project_record};
use a4n_db::project::bundle::{
    ProjectBundleEntityIdMaps, ProjectBundleHistoryImportSummary, ProjectBundleHistoryRows,
    StandaloneComicProjectBundle, StandaloneComicProjectBundleError,
    StandaloneComicTextSourceBundleRow, export_standalone_comic_project_bundle,
    import_project_bundle_history, validate_standalone_comic_project_bundle,
};
use a4n_db::project::{
    CreateProjectRecordInput, CreateStandaloneComicProjectParams,
    PrepareStandaloneComicSourceDbInput, create_project_record,
    create_standalone_comic_project_record, prepare_standalone_comic_source_chapter,
    project_by_id_record,
};
use a4n_db_models::models::chapter::chapter;
use a4n_db_models::models::generation::{
    generation_run, generation_run_target, llm_call_trace, model_usage_record, workflow_checkpoint,
    workflow_progress_event, workflow_run,
};
use a4n_db_models::models::project::{
    project_source_snapshot, source_snapshot_chapter, standalone_comic_text_source,
};
use a4n_db_models::models::user::{conversation, message};
use a4n_domain::canvas::CanvasTextAssetRecordInput;
use a4n_domain::project::ProjectSourceMode;
use chrono::{Timelike, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde_json::Value;
use std::time::Instant;

const ASSET_COUNT: usize = 32;
const SOURCE_COUNT: usize = 128;
const SECTIONS_PER_ASSET: usize = SOURCE_COUNT / ASSET_COUNT;
const SAMPLES: usize = 15;
const HISTORY_MESSAGE_COUNT: usize = 1_000;
const WORKFLOW_PROGRESS_EVENT_COUNT: usize = 200;
const MODEL_USAGE_RECORD_COUNT: usize = 1_000;
const LLM_CALL_TRACE_COUNT: usize = 1_000;
const GENERATION_TARGET_COUNT: usize = 1_000;
const WORKFLOW_CHECKPOINT_COUNT: usize = 128;
const CHECKPOINT_PAYLOAD_BYTES: usize = 16 * 1024;

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * percentile).round() as usize;
    sorted[index]
}

fn remap_checkpoint_fixture_value(
    value: &mut Value,
    source_project_id: i64,
    target_project_id: i64,
) {
    match value {
        Value::Object(object) => {
            for (key, value) in object.iter_mut() {
                if matches!(key.as_str(), "project_id" | "root_project_id")
                    && value.as_i64() == Some(source_project_id)
                {
                    *value = Value::from(target_project_id);
                } else {
                    remap_checkpoint_fixture_value(value, source_project_id, target_project_id);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                remap_checkpoint_fixture_value(value, source_project_id, target_project_id);
            }
        }
        _ => {}
    }
}

async fn import_checkpoints_with_previous_query_shape(
    db: &impl a4n_db::DbExecutor,
    source_workflow: &workflow_run::Model,
    checkpoints: &[workflow_checkpoint::Model],
    maps: &ProjectBundleEntityIdMaps,
) -> Result<i64, sea_orm::DbErr> {
    let txn = db.begin().await?;
    let source_project_id = source_workflow.project_id.unwrap();
    let project_id = maps.project_ids[&source_project_id];
    let root_project_id = maps.root_project_ids[&source_workflow.root_project_id.unwrap()];
    let workflow = workflow_run::ActiveModel {
        id: sea_orm::NotSet,
        business_key: Set(format!("previous-checkpoint:{}", uuid::Uuid::new_v4())),
        workflow_kind: Set(source_workflow.workflow_kind.clone()),
        project_id: Set(Some(project_id)),
        root_project_id: Set(Some(root_project_id)),
        source_snapshot_id: Set(None),
        scope_table: Set(source_workflow.scope_table.clone()),
        scope_id: Set(source_workflow.scope_id),
        chapter_id: Set(source_workflow.chapter_id),
        mode: Set(source_workflow.mode.clone()),
        input_fingerprint_json: Set(source_workflow.input_fingerprint_json.clone()),
        status: Set(source_workflow.status.clone()),
        current_step_key: Set(source_workflow.current_step_key.clone()),
        progress_json: Set(source_workflow.progress_json.clone()),
        active_task_id: Set(None),
        can_resume: Set(false),
        resume_block_reason: Set(source_workflow.resume_block_reason.clone()),
        error_message: Set(source_workflow.error_message.clone()),
        created_at: Set(source_workflow.created_at),
        updated_at: Set(source_workflow.updated_at),
        completed_at: Set(source_workflow.completed_at),
    }
    .insert(&txn)
    .await?;
    for source in checkpoints {
        let mut active: workflow_checkpoint::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.workflow_run_id = Set(workflow.id);

        if let (Some(input), Some(stored_hash)) = (
            source.input_fingerprint_json.as_ref(),
            source.input_fingerprint_sha256.as_deref(),
        ) {
            let actual_hash = a4n_db::generation::generation_checkpoint_fingerprint_sha256(input)
                .map_err(|error| sea_orm::DbErr::Custom(error.to_string()))?;
            if actual_hash != stored_hash {
                return Err(sea_orm::DbErr::Custom(
                    "checkpoint input fingerprint does not match source".into(),
                ));
            }
        }
        let mut fingerprint = source.input_fingerprint_json.clone();
        if let Some(value) = fingerprint.as_mut() {
            remap_checkpoint_fixture_value(value, source_project_id, project_id);
        }
        active.input_fingerprint_sha256 = Set(fingerprint
            .as_ref()
            .map(a4n_db::generation::generation_checkpoint_fingerprint_sha256)
            .transpose()
            .map_err(|error| sea_orm::DbErr::Custom(error.to_string()))?);
        active.input_fingerprint_json = Set(fingerprint);

        let mut checkpoint = source.checkpoint_json.clone();
        if let Some(value) = checkpoint.as_mut() {
            remap_checkpoint_fixture_value(value, source_project_id, project_id);
        }
        active.checkpoint_json = Set(checkpoint);
        let mut artifacts = source.artifacts_json.clone();
        if let Some(value) = artifacts.as_mut() {
            remap_checkpoint_fixture_value(value, source_project_id, project_id);
        }
        active.artifacts_json = Set(artifacts);
        active.status = Set(source.status.clone());
        active.completed_at = Set(source.completed_at);
        active.insert(&txn).await?;
    }
    txn.commit().await?;
    Ok(workflow.id)
}

async fn assert_checkpoints_match(
    db: &impl a4n_db::DbExecutor,
    workflow_run_id: i64,
    expected: &[workflow_checkpoint::Model],
    source_project_id: i64,
    target_project_id: i64,
) {
    let actual = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .order_by_asc(workflow_checkpoint::Column::StepOrder)
        .all(db)
        .await
        .expect("imported checkpoints must be readable");
    assert_eq!(actual.len(), expected.len());
    for (source, imported) in expected.iter().zip(actual) {
        let mut checkpoint = source.checkpoint_json.clone();
        if let Some(value) = checkpoint.as_mut() {
            remap_checkpoint_fixture_value(value, source_project_id, target_project_id);
        }
        let mut artifacts = source.artifacts_json.clone();
        if let Some(value) = artifacts.as_mut() {
            remap_checkpoint_fixture_value(value, source_project_id, target_project_id);
        }
        let mut fingerprint = source.input_fingerprint_json.clone();
        if let Some(value) = fingerprint.as_mut() {
            remap_checkpoint_fixture_value(value, source_project_id, target_project_id);
        }
        assert_eq!(imported.step_key, source.step_key);
        assert_eq!(imported.step_order, source.step_order);
        assert_eq!(imported.status, source.status);
        assert_eq!(imported.input_fingerprint_json, fingerprint);
        assert_eq!(
            imported.input_fingerprint_sha256,
            fingerprint
                .as_ref()
                .map(a4n_db::generation::generation_checkpoint_fingerprint_sha256)
                .transpose()
                .expect("fingerprint must be computable")
        );
        assert_eq!(imported.checkpoint_json, checkpoint);
        assert_eq!(imported.artifacts_json, artifacts);
        assert_eq!(
            imported.generation_run_ids_json,
            source.generation_run_ids_json
        );
        assert_eq!(imported.error_message, source.error_message);
        assert_eq!(imported.created_at, source.created_at);
        assert_eq!(imported.updated_at, source.updated_at);
        assert_eq!(imported.completed_at, source.completed_at);
    }
}

async fn import_history_with_previous_query_shape(
    db: &impl a4n_db::DbExecutor,
    rows: &ProjectBundleHistoryRows,
    maps: &ProjectBundleEntityIdMaps,
) -> Result<(ProjectBundleHistoryImportSummary, i64), sea_orm::DbErr> {
    let txn = db.begin().await?;
    let mut summary = ProjectBundleHistoryImportSummary::default();
    let source = rows.conversations.first().expect("one conversation source");
    let mut conversation: conversation::ActiveModel = source.clone().into();
    conversation.id = sea_orm::NotSet;
    conversation.project_id = sea_orm::Set(maps.project_ids[&source.project_id]);
    let inserted_conversation = conversation.insert(&txn).await?;
    summary.conversations_imported = 1;
    for source in &rows.messages {
        let mut active: message::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.conversation_id = sea_orm::Set(inserted_conversation.id);
        active.insert(&txn).await?;
        summary.messages_imported += 1;
    }
    txn.commit().await?;
    Ok((summary, inserted_conversation.id))
}

async fn assert_imported_history_matches(
    db: &impl a4n_db::DbExecutor,
    conversation_id: i64,
    sources: &[message::Model],
) {
    let imported = message::Entity::find()
        .filter(message::Column::ConversationId.eq(conversation_id))
        .order_by_asc(message::Column::Id)
        .all(db)
        .await
        .expect("imported message query must succeed");
    assert_eq!(imported.len(), sources.len());
    for (source, imported) in sources.iter().zip(imported) {
        assert_eq!(imported.role, source.role);
        assert_eq!(imported.content, source.content);
        assert_eq!(imported.tool_calls, source.tool_calls);
        assert_eq!(imported.prompt_tokens, source.prompt_tokens);
        assert_eq!(imported.completion_tokens, source.completion_tokens);
        assert_eq!(imported.chapter_number, source.chapter_number);
        assert_eq!(imported.is_important, source.is_important);
        assert_eq!(imported.created_at, source.created_at);
    }
}

async fn import_progress_events_with_previous_query_shape(
    db: &impl a4n_db::DbExecutor,
    source_workflow: &workflow_run::Model,
    events: &[workflow_progress_event::Model],
    maps: &ProjectBundleEntityIdMaps,
) -> Result<i64, sea_orm::DbErr> {
    let txn = db.begin().await?;
    let project_id = maps.project_ids[&source_workflow.project_id.unwrap()];
    let root_project_id = maps.root_project_ids[&source_workflow.root_project_id.unwrap()];
    let workflow = workflow_run::ActiveModel {
        id: sea_orm::NotSet,
        business_key: Set(format!(
            "baseline:{}:{}",
            uuid::Uuid::new_v4(),
            source_workflow.business_key
        )),
        workflow_kind: Set(source_workflow.workflow_kind.clone()),
        project_id: Set(Some(project_id)),
        root_project_id: Set(Some(root_project_id)),
        source_snapshot_id: Set(None),
        scope_table: Set(source_workflow.scope_table.clone()),
        scope_id: Set(source_workflow.scope_id),
        chapter_id: Set(source_workflow.chapter_id),
        mode: Set(source_workflow.mode.clone()),
        input_fingerprint_json: Set(None),
        status: Set(source_workflow.status.clone()),
        current_step_key: Set(source_workflow.current_step_key.clone()),
        progress_json: Set(source_workflow.progress_json.clone()),
        active_task_id: Set(None),
        can_resume: Set(false),
        resume_block_reason: Set(source_workflow.resume_block_reason.clone()),
        error_message: Set(source_workflow.error_message.clone()),
        created_at: Set(source_workflow.created_at),
        updated_at: Set(source_workflow.updated_at),
        completed_at: Set(source_workflow.completed_at),
    }
    .insert(&txn)
    .await?;
    for source in events {
        workflow_progress_event::ActiveModel {
            id: sea_orm::NotSet,
            workflow_run_id: Set(workflow.id),
            task_id: Set(None),
            event_index: Set(source.event_index),
            current: Set(source.current),
            total: Set(source.total),
            message: Set(source.message.clone()),
            percent: Set(source.percent),
            meta_json: Set(None),
            created_at: Set(source.created_at),
        }
        .insert(&txn)
        .await?;
    }
    txn.commit().await?;
    Ok(workflow.id)
}

async fn assert_progress_events_match(
    db: &impl a4n_db::DbExecutor,
    workflow_run_id: i64,
    expected: &[workflow_progress_event::Model],
) {
    let actual = workflow_progress_event::Entity::find()
        .filter(workflow_progress_event::Column::WorkflowRunId.eq(workflow_run_id))
        .order_by_asc(workflow_progress_event::Column::EventIndex)
        .all(db)
        .await
        .expect("imported progress events must be readable");
    assert_eq!(actual.len(), expected.len());
    for (source, imported) in expected.iter().zip(actual) {
        assert_eq!(imported.event_index, source.event_index);
        assert_eq!(imported.current, source.current);
        assert_eq!(imported.total, source.total);
        assert_eq!(imported.message, source.message);
        assert_eq!(imported.percent, source.percent);
        assert_eq!(imported.meta_json, source.meta_json);
        assert_eq!(imported.created_at, source.created_at);
        assert_eq!(imported.task_id, None);
    }
}

async fn import_usage_with_previous_query_shape(
    db: &impl a4n_db::DbExecutor,
    source_workflow: &workflow_run::Model,
    usage_sources: &[model_usage_record::Model],
    maps: &ProjectBundleEntityIdMaps,
    user_id: i64,
) -> Result<i64, sea_orm::DbErr> {
    let txn = db.begin().await?;
    let project_id = maps.project_ids[&source_workflow.project_id.unwrap()];
    let root_project_id = maps.root_project_ids[&source_workflow.root_project_id.unwrap()];
    let workflow = workflow_run::ActiveModel {
        id: sea_orm::NotSet,
        business_key: Set(format!("baseline:{}", uuid::Uuid::new_v4())),
        workflow_kind: Set(source_workflow.workflow_kind.clone()),
        project_id: Set(Some(project_id)),
        root_project_id: Set(Some(root_project_id)),
        source_snapshot_id: Set(None),
        scope_table: Set(source_workflow.scope_table.clone()),
        scope_id: Set(source_workflow.scope_id),
        chapter_id: Set(source_workflow.chapter_id),
        mode: Set(source_workflow.mode.clone()),
        input_fingerprint_json: Set(None),
        status: Set(source_workflow.status.clone()),
        current_step_key: Set(source_workflow.current_step_key.clone()),
        progress_json: Set(source_workflow.progress_json.clone()),
        active_task_id: Set(None),
        can_resume: Set(false),
        resume_block_reason: Set(source_workflow.resume_block_reason.clone()),
        error_message: Set(source_workflow.error_message.clone()),
        created_at: Set(source_workflow.created_at),
        updated_at: Set(source_workflow.updated_at),
        completed_at: Set(source_workflow.completed_at),
    }
    .insert(&txn)
    .await?;
    for source in usage_sources {
        let mut active: model_usage_record::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.request_id = Set(uuid::Uuid::new_v4().to_string());
        active.user_id = Set(Some(user_id));
        active.project_id = Set(source.project_id.map(|_| project_id));
        active.root_project_id = Set(source.root_project_id.map(|_| root_project_id));
        active.workflow_run_id = Set(source.workflow_run_id.map(|_| workflow.id));
        active.generation_run_id = Set(None);
        active.provider_config_id = Set(None);
        active.insert(&txn).await?;
    }
    txn.commit().await?;
    Ok(workflow.id)
}

async fn assert_usage_records_match(
    db: &impl a4n_db::DbExecutor,
    workflow_run_id: i64,
    user_id: i64,
    project_id: i64,
    root_project_id: i64,
    expected: &[model_usage_record::Model],
) {
    let actual = model_usage_record::Entity::find()
        .filter(model_usage_record::Column::WorkflowRunId.eq(Some(workflow_run_id)))
        .order_by_asc(model_usage_record::Column::Id)
        .all(db)
        .await
        .expect("imported usage records must be readable");
    assert_eq!(actual.len(), expected.len());
    for (source, imported) in expected.iter().zip(actual) {
        assert_ne!(imported.request_id, source.request_id);
        assert_eq!(imported.user_id, Some(user_id));
        assert_eq!(imported.project_id, Some(project_id));
        assert_eq!(imported.root_project_id, Some(root_project_id));
        assert_eq!(imported.workflow_run_id, Some(workflow_run_id));
        assert_eq!(imported.generation_run_id, None);
        assert_eq!(imported.workflow, source.workflow);
        assert_eq!(imported.step_key, source.step_key);
        assert_eq!(imported.call_key, source.call_key);
        assert_eq!(imported.source, source.source);
        assert_eq!(imported.capability, source.capability);
        assert_eq!(imported.provider_config_id, None);
        assert_eq!(imported.provider, source.provider);
        assert_eq!(imported.model, source.model);
        assert_eq!(imported.status, source.status);
        assert_eq!(imported.confidence, source.confidence);
        assert_eq!(imported.currency, source.currency);
        assert_eq!(imported.amount_micros, source.amount_micros);
        assert_eq!(imported.meter_json, source.meter_json);
        assert_eq!(imported.pricing_snapshot_json, source.pricing_snapshot_json);
        assert_eq!(imported.request_json, source.request_json);
        assert_eq!(imported.error_kind, source.error_kind);
        assert_eq!(imported.error_message, source.error_message);
        assert_eq!(imported.started_at, source.started_at);
        assert_eq!(imported.completed_at, source.completed_at);
        assert_eq!(imported.created_at, source.created_at);
    }
}

async fn import_traces_with_previous_query_shape(
    db: &impl a4n_db::DbExecutor,
    source_workflow: &workflow_run::Model,
    trace_sources: &[llm_call_trace::Model],
    maps: &ProjectBundleEntityIdMaps,
) -> Result<i64, sea_orm::DbErr> {
    let txn = db.begin().await?;
    let project_id = maps.project_ids[&source_workflow.project_id.unwrap()];
    let root_project_id = maps.root_project_ids[&source_workflow.root_project_id.unwrap()];
    let workflow = workflow_run::ActiveModel {
        id: sea_orm::NotSet,
        business_key: Set(format!("baseline:{}", uuid::Uuid::new_v4())),
        workflow_kind: Set(source_workflow.workflow_kind.clone()),
        project_id: Set(Some(project_id)),
        root_project_id: Set(Some(root_project_id)),
        source_snapshot_id: Set(None),
        scope_table: Set(source_workflow.scope_table.clone()),
        scope_id: Set(source_workflow.scope_id),
        chapter_id: Set(source_workflow.chapter_id),
        mode: Set(source_workflow.mode.clone()),
        input_fingerprint_json: Set(None),
        status: Set(source_workflow.status.clone()),
        current_step_key: Set(source_workflow.current_step_key.clone()),
        progress_json: Set(source_workflow.progress_json.clone()),
        active_task_id: Set(None),
        can_resume: Set(false),
        resume_block_reason: Set(source_workflow.resume_block_reason.clone()),
        error_message: Set(source_workflow.error_message.clone()),
        created_at: Set(source_workflow.created_at),
        updated_at: Set(source_workflow.updated_at),
        completed_at: Set(source_workflow.completed_at),
    }
    .insert(&txn)
    .await?;
    for source in trace_sources {
        let mut active: llm_call_trace::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.project_id = Set(source.project_id.map(|_| project_id));
        active.root_project_id = Set(source.root_project_id.map(|_| root_project_id));
        active.workflow_run_id = Set(source.workflow_run_id.map(|_| workflow.id));
        active.generation_run_id = Set(None);
        active.insert(&txn).await?;
    }
    txn.commit().await?;
    Ok(workflow.id)
}

async fn assert_traces_match(
    db: &impl a4n_db::DbExecutor,
    workflow_run_id: i64,
    project_id: i64,
    root_project_id: i64,
    expected: &[llm_call_trace::Model],
) {
    let actual = llm_call_trace::Entity::find()
        .filter(llm_call_trace::Column::WorkflowRunId.eq(Some(workflow_run_id)))
        .order_by_asc(llm_call_trace::Column::Id)
        .all(db)
        .await
        .expect("imported LLM traces must be readable");
    assert_eq!(actual.len(), expected.len());
    for (source, imported) in expected.iter().zip(actual) {
        assert_eq!(imported.project_id, Some(project_id));
        assert_eq!(imported.root_project_id, Some(root_project_id));
        assert_eq!(imported.workflow_run_id, Some(workflow_run_id));
        assert_eq!(imported.generation_run_id, None);
        assert_eq!(imported.call_key, source.call_key);
        assert_eq!(
            imported.input_fingerprint_sha256,
            source.input_fingerprint_sha256
        );
        assert_eq!(imported.workflow, source.workflow);
        assert_eq!(imported.step_key, source.step_key);
        assert_eq!(imported.provider, source.provider);
        assert_eq!(imported.model, source.model);
        assert_eq!(imported.context_plan_json, source.context_plan_json);
        assert_eq!(
            imported.context_pack_refs_json,
            source.context_pack_refs_json
        );
        assert_eq!(imported.prompt_tokens, source.prompt_tokens);
        assert_eq!(imported.completion_tokens, source.completion_tokens);
        assert_eq!(imported.total_tokens, source.total_tokens);
        assert_eq!(
            imported.prompt_cache_hit_tokens,
            source.prompt_cache_hit_tokens
        );
        assert_eq!(
            imported.prompt_cache_miss_tokens,
            source.prompt_cache_miss_tokens
        );
        assert_eq!(
            imported.prompt_cache_write_tokens,
            source.prompt_cache_write_tokens
        );
        assert_eq!(
            imported.prompt_cache_hit_ratio,
            source.prompt_cache_hit_ratio
        );
        assert_eq!(imported.duration_ms, source.duration_ms);
        assert_eq!(imported.streamed, source.streamed);
        assert_eq!(imported.status, source.status);
        assert_eq!(imported.error_kind, source.error_kind);
        assert_eq!(imported.error_message, source.error_message);
        assert_eq!(imported.finish_reason, source.finish_reason);
        assert_eq!(imported.created_at, source.created_at);
    }
}

async fn import_generation_targets_with_previous_query_shape(
    db: &impl a4n_db::DbExecutor,
    source_workflow: &workflow_run::Model,
    source_generation: &generation_run::Model,
    target_sources: &[generation_run_target::Model],
    maps: &ProjectBundleEntityIdMaps,
) -> Result<(i64, i64), sea_orm::DbErr> {
    let txn = db.begin().await?;
    let project_id = maps.project_ids[&source_workflow.project_id.unwrap()];
    let root_project_id = maps.root_project_ids[&source_workflow.root_project_id.unwrap()];
    let workflow = workflow_run::ActiveModel {
        id: sea_orm::NotSet,
        business_key: Set(format!("baseline:{}", uuid::Uuid::new_v4())),
        workflow_kind: Set(source_workflow.workflow_kind.clone()),
        project_id: Set(Some(project_id)),
        root_project_id: Set(Some(root_project_id)),
        source_snapshot_id: Set(None),
        scope_table: Set(source_workflow.scope_table.clone()),
        scope_id: Set(source_workflow.scope_id),
        chapter_id: Set(source_workflow.chapter_id),
        mode: Set(source_workflow.mode.clone()),
        input_fingerprint_json: Set(None),
        status: Set(source_workflow.status.clone()),
        current_step_key: Set(source_workflow.current_step_key.clone()),
        progress_json: Set(source_workflow.progress_json.clone()),
        active_task_id: Set(None),
        can_resume: Set(false),
        resume_block_reason: Set(source_workflow.resume_block_reason.clone()),
        error_message: Set(source_workflow.error_message.clone()),
        created_at: Set(source_workflow.created_at),
        updated_at: Set(source_workflow.updated_at),
        completed_at: Set(source_workflow.completed_at),
    }
    .insert(&txn)
    .await?;
    let generation = generation_run::ActiveModel {
        id: sea_orm::NotSet,
        project_id: Set(project_id),
        root_project_id: Set(root_project_id),
        workflow: Set(source_generation.workflow.clone()),
        step_key: Set(source_generation.step_key.clone()),
        title: Set(source_generation.title.clone()),
        model_kind: Set(source_generation.model_kind.clone()),
        provider: Set(source_generation.provider.clone()),
        model: Set(source_generation.model.clone()),
        status: Set(source_generation.status.clone()),
        workflow_run_id: Set(Some(workflow.id)),
        call_key: Set(source_generation.call_key.clone()),
        input_fingerprint_sha256: Set(source_generation.input_fingerprint_sha256.clone()),
        input_fingerprint_json: Set(source_generation.input_fingerprint_json.clone()),
        raw_response_json: Set(source_generation.raw_response_json.clone()),
        parsed_output_json: Set(source_generation.parsed_output_json.clone()),
        validation_json: Set(source_generation.validation_json.clone()),
        artifact_refs_json: Set(source_generation.artifact_refs_json.clone()),
        attempt: Set(source_generation.attempt),
        prompt_template_names_json: Set(source_generation.prompt_template_names_json.clone()),
        prompt_template_hashes_json: Set(source_generation.prompt_template_hashes_json.clone()),
        rendered_messages_json: Set(source_generation.rendered_messages_json.clone()),
        rendered_prompts_json: Set(source_generation.rendered_prompts_json.clone()),
        input_variables_json: Set(source_generation.input_variables_json.clone()),
        source_refs_json: Set(source_generation.source_refs_json.clone()),
        media_refs_json: Set(source_generation.media_refs_json.clone()),
        request_json: Set(source_generation.request_json.clone()),
        response_summary_json: Set(source_generation.response_summary_json.clone()),
        error_message: Set(source_generation.error_message.clone()),
        created_at: Set(source_generation.created_at),
        completed_at: Set(source_generation.completed_at),
    }
    .insert(&txn)
    .await?;

    for source in target_sources {
        let mut active: generation_run_target::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.generation_run_id = Set(generation.id);
        let mapped = if source.target_table == "chapter" {
            maps.chapter_ids.get(&source.target_id)
        } else {
            maps.entity_ids
                .get(&source.target_table)
                .and_then(|ids| ids.get(&source.target_id))
        };
        if let Some(mapped) = mapped {
            active.target_id = Set(*mapped);
        } else if !source.target_table.starts_with("source_archive:") {
            active.target_table = Set(format!("source_archive:{}", source.target_table));
        }
        active.insert(&txn).await?;
    }
    txn.commit().await?;
    Ok((workflow.id, generation.id))
}

async fn assert_generation_targets_match(
    db: &impl a4n_db::DbExecutor,
    generation_run_id: i64,
    target_sources: &[generation_run_target::Model],
    maps: &ProjectBundleEntityIdMaps,
) {
    let actual = generation_run_target::Entity::find()
        .filter(generation_run_target::Column::GenerationRunId.eq(generation_run_id))
        .order_by_asc(generation_run_target::Column::Id)
        .all(db)
        .await
        .expect("imported generation targets must be readable");
    assert_eq!(actual.len(), target_sources.len());
    for (source, imported) in target_sources.iter().zip(actual) {
        assert_eq!(imported.target_kind, source.target_kind);
        assert_eq!(imported.target_table, source.target_table);
        let target_ids = if source.target_table == "chapter" {
            &maps.chapter_ids
        } else {
            &maps.entity_ids[&source.target_table]
        };
        assert_eq!(imported.target_id, target_ids[&source.target_id]);
        assert_eq!(imported.role, source.role);
        assert_eq!(imported.created_at, source.created_at);
    }
}

async fn export_with_previous_query_shape(
    db: &impl a4n_db::DbExecutor,
    project_id: i64,
) -> Result<StandaloneComicProjectBundle, StandaloneComicProjectBundleError> {
    let project = project_by_id_record(db, project_id)
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

    let mut text_assets = canvas_text_assets_by_project_record(db, project_id)
        .await
        .map_err(|error| StandaloneComicProjectBundleError::Invalid(error.to_string()))?;
    for asset in &mut text_assets {
        asset.versions = canvas_text_asset_versions_record(db, project_id, asset.id)
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

#[tokio::test]
#[ignore = "manual PostgreSQL benchmark; needs isolated A4N_TEST_DATABASE_URL"]
async fn performance_standalone_comic_project_bundle_batches_source_and_version_reads() {
    let db = a4n_migration::test_support::connect_test_database().await;
    a4n_migration::migrations::canvas_project_split::ensure_complete(&db)
        .await
        .expect("comic project identity migration must be complete");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let user = create_user_summary(
        &db,
        &format!("comic-bundle-{suffix}"),
        &format!("comic-bundle-{suffix}@example.com"),
        "password",
        "user",
    )
    .await
    .expect("test user must be created");
    let project = create_standalone_comic_project_record(
        &db,
        user.id,
        CreateStandaloneComicProjectParams {
            title: format!("漫画包性能夹具 {suffix}"),
            description: String::new(),
        },
    )
    .await
    .expect("standalone comic project must be created");
    let now = Utc::now().naive_utc();

    let mut asset_ids = Vec::with_capacity(ASSET_COUNT);
    for asset_index in 0..ASSET_COUNT {
        let content = (0..SECTIONS_PER_ASSET)
            .map(|section_index| {
                format!(
                    "# Section {section_index}\nAsset {asset_index}, section {section_index}.\n"
                )
            })
            .collect::<String>();
        let asset = a4n_db::canvas::upsert_canvas_text_asset_record(
            &db,
            project.id,
            CanvasTextAssetRecordInput {
                id: None,
                expected_version: None,
                title: format!("Performance text {asset_index}"),
                text_kind: "script".to_string(),
                content: content.clone(),
            },
            now,
        )
        .await
        .expect("text asset version 1 must be created");
        a4n_db::canvas::upsert_canvas_text_asset_record(
            &db,
            project.id,
            CanvasTextAssetRecordInput {
                id: Some(asset.id),
                expected_version: Some(1),
                title: format!("Performance text {asset_index}"),
                text_kind: "script".to_string(),
                content: format!("{content}\nRevision 2."),
            },
            now,
        )
        .await
        .expect("text asset version 2 must be created");
        asset_ids.push(asset.id);
    }

    for source_index in 0..SOURCE_COUNT {
        let asset_index = source_index % ASSET_COUNT;
        let section_index = source_index / ASSET_COUNT;
        prepare_standalone_comic_source_chapter(
            &db,
            PrepareStandaloneComicSourceDbInput {
                project_id: project.id,
                asset_id: asset_ids[asset_index],
                version: 1,
                start_section: section_index as i32,
                end_section: section_index as i32,
            },
        )
        .await
        .expect("standalone comic source chapter must be created");
    }

    for _ in 0..2 {
        let previous = export_with_previous_query_shape(&db, project.id)
            .await
            .expect("previous query shape must export the fixture");
        let current = export_standalone_comic_project_bundle(&db, project.id)
            .await
            .expect("batched query shape must export the fixture");
        assert_eq!(
            serde_json::to_value(previous).unwrap(),
            serde_json::to_value(current).unwrap(),
            "complete project bundle must remain identical"
        );
    }

    let mut previous_samples = Vec::with_capacity(SAMPLES);
    let mut current_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        let (previous, current) = if sample % 2 == 0 {
            let started = Instant::now();
            let previous = export_with_previous_query_shape(&db, project.id)
                .await
                .expect("previous query shape must export the fixture");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);

            let started = Instant::now();
            let current = export_standalone_comic_project_bundle(&db, project.id)
                .await
                .expect("batched query shape must export the fixture");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            (previous, current)
        } else {
            let started = Instant::now();
            let current = export_standalone_comic_project_bundle(&db, project.id)
                .await
                .expect("batched query shape must export the fixture");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);

            let started = Instant::now();
            let previous = export_with_previous_query_shape(&db, project.id)
                .await
                .expect("previous query shape must export the fixture");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            (previous, current)
        };
        assert_eq!(
            serde_json::to_value(previous).unwrap(),
            serde_json::to_value(current).unwrap(),
            "complete project bundle must remain identical"
        );
    }

    let previous_p50 = percentile(&previous_samples, 0.50);
    let previous_p95 = percentile(&previous_samples, 0.95);
    let current_p50 = percentile(&current_samples, 0.50);
    let current_p95 = percentile(&current_samples, 0.95);
    let p50_improvement = 1.0 - current_p50 / previous_p50;
    let p95_improvement = 1.0 - current_p95 / previous_p95;
    println!(
        "standalone_comic_bundle sources={SOURCE_COUNT} assets={ASSET_COUNT} previous_sql={} batched_sql=10 p50_ms={previous_p50:.3}->{current_p50:.3} ({:.1}%) p95_ms={previous_p95:.3}->{current_p95:.3} ({:.1}%)",
        4 + 3 * ASSET_COUNT + 3 * SOURCE_COUNT,
        p50_improvement * 100.0,
        p95_improvement * 100.0,
    );
    assert!(
        p50_improvement >= 0.30,
        "P50 improvement must meet 30%: {p50_improvement:.1}%"
    );
    assert!(
        p95_improvement >= 0.30,
        "P95 improvement must meet 30%: {p95_improvement:.1}%"
    );

    let missing_asset = canvas_text_asset_versions_record(&db, project.id, i64::MAX)
        .await
        .expect_err("missing assets must remain an error");
    assert!(matches!(
        missing_asset,
        a4n_db::canvas::CanvasDbError::NotFound(_)
    ));

    let foreign_project = create_standalone_comic_project_record(
        &db,
        user.id,
        CreateStandaloneComicProjectParams {
            title: format!("漫画包权限夹具 {suffix}"),
            description: String::new(),
        },
    )
    .await
    .expect("foreign project must be created");
    let foreign_asset = a4n_db::canvas::upsert_canvas_text_asset_record(
        &db,
        foreign_project.id,
        CanvasTextAssetRecordInput {
            id: None,
            expected_version: None,
            title: "Foreign text".to_string(),
            text_kind: "script".to_string(),
            content: "foreign project data".to_string(),
        },
        now,
    )
    .await
    .expect("foreign text asset must be created");
    let foreign_asset_error = a4n_db::canvas::canvas_text_asset_versions_by_project_record(
        &db,
        project.id,
        &[foreign_asset.id],
    )
    .await
    .expect_err("foreign assets must remain forbidden");
    assert!(matches!(
        foreign_asset_error,
        a4n_db::canvas::CanvasDbError::Forbidden(_)
    ));
}

#[tokio::test]
#[ignore = "manual PostgreSQL benchmark; needs isolated A4N_TEST_DATABASE_URL"]
async fn performance_project_bundle_history_batches_message_imports() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let user = create_user_summary(
        &db,
        &format!("bundle-history-{suffix}"),
        &format!("bundle-history-{suffix}@example.com"),
        "password",
        "user",
    )
    .await
    .expect("test user must be created");
    let source_project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: format!("History source {suffix}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("source project must be created");
    let target_project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: format!("History target {suffix}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("target project must be created");
    let now = Utc::now().naive_utc();
    let source_conversation = conversation::ActiveModel {
        project_id: Set(source_project.id),
        title: Set(Some(format!("Performance conversation {suffix}"))),
        conversation_type: Set("general".to_string()),
        summary: Set(None),
        extracted_facts: Set(None),
        message_count: Set(HISTORY_MESSAGE_COUNT as i32),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source conversation must be created");
    let source_messages = (0..HISTORY_MESSAGE_COUNT)
        .map(|index| message::ActiveModel {
            conversation_id: Set(source_conversation.id),
            role: Set(if index % 2 == 0 { "user" } else { "assistant" }.to_string()),
            content: Set(format!("History item {index}: {}", "x".repeat(256))),
            tool_calls: Set(None),
            prompt_tokens: Set(Some(100)),
            completion_tokens: Set(Some(80)),
            chapter_number: Set(Some((index / 20) as i32 + 1)),
            is_important: Set(index % 100 == 0),
            created_at: Set(now + chrono::Duration::microseconds((index / 2) as i64)),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    message::Entity::insert_many(source_messages)
        .exec(&db)
        .await
        .expect("source messages must be created");
    let source_messages = message::Entity::find()
        .filter(message::Column::ConversationId.eq(source_conversation.id))
        .order_by_asc(message::Column::Id)
        .all(&db)
        .await
        .expect("source messages must be read");
    let rows = ProjectBundleHistoryRows {
        conversations: vec![source_conversation.clone()],
        messages: source_messages.clone(),
        ..Default::default()
    };
    let maps = ProjectBundleEntityIdMaps {
        project_ids: [(source_project.id, target_project.id)]
            .into_iter()
            .collect(),
        ..Default::default()
    };

    for _ in 0..2 {
        let (previous, previous_conversation_id) =
            import_history_with_previous_query_shape(&db, &rows, &maps)
                .await
                .expect("previous import shape must succeed");
        let current = import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
            .await
            .expect("batched import must succeed");
        assert_eq!(
            previous.conversations_imported,
            current.conversations_imported
        );
        assert_eq!(previous.messages_imported, current.messages_imported);
        assert_imported_history_matches(&db, previous_conversation_id, &source_messages).await;
        let current_conversation = conversation::Entity::find()
            .filter(conversation::Column::ProjectId.eq(target_project.id))
            .filter(conversation::Column::Title.eq(source_conversation.title.clone()))
            .order_by_desc(conversation::Column::Id)
            .one(&db)
            .await
            .expect("batched conversation query must succeed")
            .expect("batched conversation must be present");
        assert_imported_history_matches(&db, current_conversation.id, &source_messages).await;
    }

    let mut previous_samples = Vec::with_capacity(SAMPLES);
    let mut current_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let (previous, previous_conversation_id) =
                import_history_with_previous_query_shape(&db, &rows, &maps)
                    .await
                    .expect("previous import shape must succeed");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(previous.messages_imported, HISTORY_MESSAGE_COUNT);
            assert_imported_history_matches(&db, previous_conversation_id, &source_messages).await;

            let started = Instant::now();
            let current = import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
                .await
                .expect("batched import must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(current.messages_imported, HISTORY_MESSAGE_COUNT);
        } else {
            let started = Instant::now();
            let current = import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
                .await
                .expect("batched import must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(current.messages_imported, HISTORY_MESSAGE_COUNT);

            let started = Instant::now();
            let (previous, previous_conversation_id) =
                import_history_with_previous_query_shape(&db, &rows, &maps)
                    .await
                    .expect("previous import shape must succeed");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(previous.messages_imported, HISTORY_MESSAGE_COUNT);
            assert_imported_history_matches(&db, previous_conversation_id, &source_messages).await;
        }
    }

    let previous_p50 = percentile(&previous_samples, 0.50);
    let previous_p95 = percentile(&previous_samples, 0.95);
    let current_p50 = percentile(&current_samples, 0.50);
    let current_p95 = percentile(&current_samples, 0.95);
    let p50_improvement = 1.0 - current_p50 / previous_p50;
    let p95_improvement = 1.0 - current_p95 / previous_p95;
    println!(
        "bundle_history_import messages={HISTORY_MESSAGE_COUNT} previous_sql={} batched_sql=4 p50_ms={previous_p50:.3}->{current_p50:.3} ({:.1}%) p95_ms={previous_p95:.3}->{current_p95:.3} ({:.1}%)",
        HISTORY_MESSAGE_COUNT + 3,
        p50_improvement * 100.0,
        p95_improvement * 100.0,
    );
    assert!(
        p50_improvement >= 0.30,
        "P50 improvement must meet 30%: {p50_improvement:.1}%"
    );
    assert!(
        p95_improvement >= 0.30,
        "P95 improvement must meet 30%: {p95_improvement:.1}%"
    );
}

#[tokio::test]
#[ignore = "manual PostgreSQL benchmark; needs isolated A4N_TEST_DATABASE_URL"]
async fn performance_project_bundle_history_batches_progress_events() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let user = create_user_summary(
        &db,
        &format!("bundle-events-{suffix}"),
        &format!("bundle-events-{suffix}@example.com"),
        "password",
        "user",
    )
    .await
    .expect("test user must be created");
    let source_project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: format!("Event history source {suffix}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("source project must be created");
    let target_project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: format!("Event history target {suffix}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("target project must be created");
    let now = Utc::now().naive_utc();
    let source_workflow = workflow_run::ActiveModel {
        business_key: Set(format!("performance-events:{suffix}")),
        workflow_kind: Set("chapter_analysis".to_string()),
        project_id: Set(Some(source_project.id)),
        root_project_id: Set(Some(source_project.id)),
        status: Set("completed".to_string()),
        can_resume: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        completed_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source workflow must be created");
    let source_events = (0..WORKFLOW_PROGRESS_EVENT_COUNT)
        .map(|index| workflow_progress_event::ActiveModel {
            workflow_run_id: Set(source_workflow.id),
            task_id: Set(Some(format!("source-task-{index}"))),
            event_index: Set(index as i64 + 1),
            current: Set(index as i32 + 1),
            total: Set(WORKFLOW_PROGRESS_EVENT_COUNT as i32),
            message: Set(format!("Progress event {index}")),
            percent: Set((index + 1) as f32 / WORKFLOW_PROGRESS_EVENT_COUNT as f32 * 100.0),
            meta_json: Set(None),
            created_at: Set(now + chrono::Duration::microseconds((index / 2) as i64)),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    workflow_progress_event::Entity::insert_many(source_events)
        .exec(&db)
        .await
        .expect("source progress events must be created");
    let source_events = workflow_progress_event::Entity::find()
        .filter(workflow_progress_event::Column::WorkflowRunId.eq(source_workflow.id))
        .order_by_asc(workflow_progress_event::Column::EventIndex)
        .all(&db)
        .await
        .expect("source progress events must be read");
    assert_eq!(source_events.len(), WORKFLOW_PROGRESS_EVENT_COUNT);
    let rows = ProjectBundleHistoryRows {
        workflow_runs: vec![source_workflow.clone()],
        workflow_progress_events: source_events.clone(),
        ..Default::default()
    };
    let maps = ProjectBundleEntityIdMaps {
        project_ids: [(source_project.id, target_project.id)]
            .into_iter()
            .collect(),
        root_project_ids: [(source_project.id, target_project.id)]
            .into_iter()
            .collect(),
        ..Default::default()
    };

    for _ in 0..2 {
        let previous_workflow_id = import_progress_events_with_previous_query_shape(
            &db,
            &source_workflow,
            &source_events,
            &maps,
        )
        .await
        .expect("previous progress event import must succeed");
        import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
            .await
            .expect("batched progress event import must succeed");
        assert_progress_events_match(&db, previous_workflow_id, &source_events).await;
        let current_workflow = workflow_run::Entity::find()
            .filter(workflow_run::Column::ProjectId.eq(Some(target_project.id)))
            .filter(workflow_run::Column::WorkflowKind.eq("chapter_analysis"))
            .order_by_desc(workflow_run::Column::Id)
            .one(&db)
            .await
            .expect("batched workflow query must succeed")
            .expect("batched workflow must be present");
        assert_progress_events_match(&db, current_workflow.id, &source_events).await;
    }

    let mut previous_samples = Vec::with_capacity(SAMPLES);
    let mut current_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let previous_workflow_id = import_progress_events_with_previous_query_shape(
                &db,
                &source_workflow,
                &source_events,
                &maps,
            )
            .await
            .expect("previous progress event import must succeed");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_progress_events_match(&db, previous_workflow_id, &source_events).await;

            let started = Instant::now();
            import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
                .await
                .expect("batched progress event import must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            let current_workflow = workflow_run::Entity::find()
                .filter(workflow_run::Column::ProjectId.eq(Some(target_project.id)))
                .filter(workflow_run::Column::WorkflowKind.eq("chapter_analysis"))
                .order_by_desc(workflow_run::Column::Id)
                .one(&db)
                .await
                .expect("batched workflow query must succeed")
                .expect("batched workflow must be present");
            assert_progress_events_match(&db, current_workflow.id, &source_events).await;
        } else {
            let started = Instant::now();
            import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
                .await
                .expect("batched progress event import must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            let current_workflow = workflow_run::Entity::find()
                .filter(workflow_run::Column::ProjectId.eq(Some(target_project.id)))
                .filter(workflow_run::Column::WorkflowKind.eq("chapter_analysis"))
                .order_by_desc(workflow_run::Column::Id)
                .one(&db)
                .await
                .expect("batched workflow query must succeed")
                .expect("batched workflow must be present");
            assert_progress_events_match(&db, current_workflow.id, &source_events).await;

            let started = Instant::now();
            let previous_workflow_id = import_progress_events_with_previous_query_shape(
                &db,
                &source_workflow,
                &source_events,
                &maps,
            )
            .await
            .expect("previous progress event import must succeed");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_progress_events_match(&db, previous_workflow_id, &source_events).await;
        }
    }

    let previous_p50 = percentile(&previous_samples, 0.50);
    let previous_p95 = percentile(&previous_samples, 0.95);
    let current_p50 = percentile(&current_samples, 0.50);
    let current_p95 = percentile(&current_samples, 0.95);
    let p50_improvement = 1.0 - current_p50 / previous_p50;
    let p95_improvement = 1.0 - current_p95 / previous_p95;
    println!(
        "bundle_history_import_progress_events events={WORKFLOW_PROGRESS_EVENT_COUNT} previous_sql={} batched_sql=4 p50_ms={previous_p50:.3}->{current_p50:.3} ({:.1}%) p95_ms={previous_p95:.3}->{current_p95:.3} ({:.1}%)",
        WORKFLOW_PROGRESS_EVENT_COUNT + 3,
        p50_improvement * 100.0,
        p95_improvement * 100.0,
    );
    assert!(
        p50_improvement >= 0.30,
        "P50 improvement must meet 30%: {p50_improvement:.1}%"
    );
    assert!(
        p95_improvement >= 0.30,
        "P95 improvement must meet 30%: {p95_improvement:.1}%"
    );
}

#[tokio::test]
#[ignore = "manual PostgreSQL benchmark; needs isolated A4N_TEST_DATABASE_URL"]
async fn performance_project_bundle_history_batches_usage_records() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let user = create_user_summary(
        &db,
        &format!("bundle-usage-{suffix}"),
        &format!("bundle-usage-{suffix}@example.com"),
        "password",
        "user",
    )
    .await
    .expect("test user must be created");
    let source_project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: format!("Usage history source {suffix}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("source project must be created");
    let target_project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: format!("Usage history target {suffix}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("target project must be created");
    let now = Utc::now().naive_utc();
    let source_workflow = workflow_run::ActiveModel {
        business_key: Set(format!("performance-usage:{suffix}")),
        workflow_kind: Set("chapter_analysis".to_string()),
        project_id: Set(Some(source_project.id)),
        root_project_id: Set(Some(source_project.id)),
        status: Set("completed".to_string()),
        can_resume: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        completed_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source workflow must be created");
    let source_records = (0..MODEL_USAGE_RECORD_COUNT)
        .map(|index| model_usage_record::ActiveModel {
            request_id: Set(format!("usage-{suffix}-{index}")),
            user_id: Set(Some(user.id)),
            project_id: Set(Some(source_project.id)),
            root_project_id: Set(Some(source_project.id)),
            workflow_run_id: Set(Some(source_workflow.id)),
            generation_run_id: Set(None),
            workflow: Set("chapter_analysis".to_string()),
            step_key: Set(format!("analysis-{index}")),
            call_key: Set(Some(format!("call-{index}"))),
            source: Set(Some("agent".to_string())),
            capability: Set("text_generation".to_string()),
            provider_config_id: Set(Some(1)),
            provider: Set(Some("test-provider".to_string())),
            model: Set(Some("test-model".to_string())),
            status: Set("completed".to_string()),
            confidence: Set("exact".to_string()),
            currency: Set("USD".to_string()),
            amount_micros: Set(Some(125)),
            meter_json: Set(serde_json::json!({"prompt_tokens": 100, "completion_tokens": 80})),
            pricing_snapshot_json: Set(serde_json::json!({"currency": "USD", "unit_price": 1})),
            request_json: Set(Some(serde_json::json!({"call": index}))),
            error_kind: Set(None),
            error_message: Set(None),
            started_at: Set(now + chrono::Duration::microseconds((index / 2) as i64)),
            completed_at: Set(Some(
                now + chrono::Duration::microseconds((index / 2) as i64 + 1),
            )),
            created_at: Set(now + chrono::Duration::microseconds((index / 2) as i64)),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    model_usage_record::Entity::insert_many(source_records)
        .exec(&db)
        .await
        .expect("source usage records must be created");
    let source_records = model_usage_record::Entity::find()
        .filter(model_usage_record::Column::WorkflowRunId.eq(Some(source_workflow.id)))
        .order_by_asc(model_usage_record::Column::Id)
        .all(&db)
        .await
        .expect("source usage records must be read");
    assert_eq!(source_records.len(), MODEL_USAGE_RECORD_COUNT);
    let rows = ProjectBundleHistoryRows {
        workflow_runs: vec![source_workflow.clone()],
        model_usage_records: source_records.clone(),
        ..Default::default()
    };
    let maps = ProjectBundleEntityIdMaps {
        project_ids: [(source_project.id, target_project.id)]
            .into_iter()
            .collect(),
        root_project_ids: [(source_project.id, target_project.id)]
            .into_iter()
            .collect(),
        ..Default::default()
    };

    for _ in 0..2 {
        let previous_workflow_id = import_usage_with_previous_query_shape(
            &db,
            &source_workflow,
            &source_records,
            &maps,
            user.id,
        )
        .await
        .expect("previous usage import must succeed");
        import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
            .await
            .expect("batched usage import must succeed");
        assert_usage_records_match(
            &db,
            previous_workflow_id,
            user.id,
            target_project.id,
            target_project.id,
            &source_records,
        )
        .await;
        let current_workflow = workflow_run::Entity::find()
            .filter(workflow_run::Column::ProjectId.eq(Some(target_project.id)))
            .filter(workflow_run::Column::WorkflowKind.eq("chapter_analysis"))
            .order_by_desc(workflow_run::Column::Id)
            .one(&db)
            .await
            .expect("batched workflow query must succeed")
            .expect("batched workflow must be present");
        assert_usage_records_match(
            &db,
            current_workflow.id,
            user.id,
            target_project.id,
            target_project.id,
            &source_records,
        )
        .await;
    }

    let mut previous_samples = Vec::with_capacity(SAMPLES);
    let mut current_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let previous_workflow_id = import_usage_with_previous_query_shape(
                &db,
                &source_workflow,
                &source_records,
                &maps,
                user.id,
            )
            .await
            .expect("previous usage import must succeed");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_usage_records_match(
                &db,
                previous_workflow_id,
                user.id,
                target_project.id,
                target_project.id,
                &source_records,
            )
            .await;

            let started = Instant::now();
            import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
                .await
                .expect("batched usage import must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            let current_workflow = workflow_run::Entity::find()
                .filter(workflow_run::Column::ProjectId.eq(Some(target_project.id)))
                .filter(workflow_run::Column::WorkflowKind.eq("chapter_analysis"))
                .order_by_desc(workflow_run::Column::Id)
                .one(&db)
                .await
                .expect("batched workflow query must succeed")
                .expect("batched workflow must be present");
            assert_usage_records_match(
                &db,
                current_workflow.id,
                user.id,
                target_project.id,
                target_project.id,
                &source_records,
            )
            .await;
        } else {
            let started = Instant::now();
            import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
                .await
                .expect("batched usage import must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            let current_workflow = workflow_run::Entity::find()
                .filter(workflow_run::Column::ProjectId.eq(Some(target_project.id)))
                .filter(workflow_run::Column::WorkflowKind.eq("chapter_analysis"))
                .order_by_desc(workflow_run::Column::Id)
                .one(&db)
                .await
                .expect("batched workflow query must succeed")
                .expect("batched workflow must be present");
            assert_usage_records_match(
                &db,
                current_workflow.id,
                user.id,
                target_project.id,
                target_project.id,
                &source_records,
            )
            .await;

            let started = Instant::now();
            let previous_workflow_id = import_usage_with_previous_query_shape(
                &db,
                &source_workflow,
                &source_records,
                &maps,
                user.id,
            )
            .await
            .expect("previous usage import must succeed");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_usage_records_match(
                &db,
                previous_workflow_id,
                user.id,
                target_project.id,
                target_project.id,
                &source_records,
            )
            .await;
        }
    }

    let previous_p50 = percentile(&previous_samples, 0.50);
    let previous_p95 = percentile(&previous_samples, 0.95);
    let current_p50 = percentile(&current_samples, 0.50);
    let current_p95 = percentile(&current_samples, 0.95);
    let p50_improvement = 1.0 - current_p50 / previous_p50;
    let p95_improvement = 1.0 - current_p95 / previous_p95;
    println!(
        "bundle_history_import_usage records={MODEL_USAGE_RECORD_COUNT} previous_sql={} batched_sql=4 p50_ms={previous_p50:.3}->{current_p50:.3} ({:.1}%) p95_ms={previous_p95:.3}->{current_p95:.3} ({:.1}%)",
        MODEL_USAGE_RECORD_COUNT + 3,
        p50_improvement * 100.0,
        p95_improvement * 100.0,
    );
    assert!(
        p50_improvement >= 0.30,
        "P50 improvement must meet 30%: {p50_improvement:.1}%"
    );
    assert!(
        p95_improvement >= 0.30,
        "P95 improvement must meet 30%: {p95_improvement:.1}%"
    );
}

#[tokio::test]
#[ignore = "manual PostgreSQL benchmark; needs isolated A4N_TEST_DATABASE_URL"]
async fn performance_project_bundle_history_batches_llm_traces() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let user = create_user_summary(
        &db,
        &format!("bundle-traces-{suffix}"),
        &format!("bundle-traces-{suffix}@example.com"),
        "password",
        "user",
    )
    .await
    .expect("test user must be created");
    let source_project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: format!("Trace history source {suffix}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("source project must be created");
    let target_project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: format!("Trace history target {suffix}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("target project must be created");
    let now = Utc::now().naive_utc();
    let source_workflow = workflow_run::ActiveModel {
        business_key: Set(format!("performance-traces:{suffix}")),
        workflow_kind: Set("chapter_analysis".to_string()),
        project_id: Set(Some(source_project.id)),
        root_project_id: Set(Some(source_project.id)),
        status: Set("completed".to_string()),
        can_resume: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        completed_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source workflow must be created");
    let source_traces = (0..LLM_CALL_TRACE_COUNT)
        .map(|index| llm_call_trace::ActiveModel {
            workflow_run_id: Set(Some(source_workflow.id)),
            call_key: Set(Some(format!("call-{index}"))),
            input_fingerprint_sha256: Set(Some(format!("fingerprint-{index}"))),
            project_id: Set(Some(source_project.id)),
            root_project_id: Set(Some(source_project.id)),
            workflow: Set("chapter_analysis".to_string()),
            step_key: Set(format!("analysis-{index}")),
            provider: Set(Some("test-provider".to_string())),
            model: Set(Some("test-model".to_string())),
            context_plan_json: Set(Some(serde_json::json!({"call": index}))),
            context_pack_refs_json: Set(Some(serde_json::json!([]))),
            prompt_tokens: Set(Some(100)),
            completion_tokens: Set(Some(80)),
            total_tokens: Set(Some(180)),
            prompt_cache_hit_tokens: Set(Some(20)),
            prompt_cache_miss_tokens: Set(Some(80)),
            prompt_cache_write_tokens: Set(None),
            prompt_cache_hit_ratio: Set(Some(0.2)),
            duration_ms: Set(Some(300)),
            streamed: Set(true),
            status: Set("completed".to_string()),
            error_kind: Set(None),
            error_message: Set(None),
            finish_reason: Set(Some("stop".to_string())),
            created_at: Set(now + chrono::Duration::microseconds((index / 2) as i64)),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    llm_call_trace::Entity::insert_many(source_traces)
        .exec(&db)
        .await
        .expect("source LLM traces must be created");
    let source_traces = llm_call_trace::Entity::find()
        .filter(llm_call_trace::Column::WorkflowRunId.eq(Some(source_workflow.id)))
        .order_by_asc(llm_call_trace::Column::Id)
        .all(&db)
        .await
        .expect("source LLM traces must be read");
    assert_eq!(source_traces.len(), LLM_CALL_TRACE_COUNT);
    let rows = ProjectBundleHistoryRows {
        workflow_runs: vec![source_workflow.clone()],
        llm_call_traces: source_traces.clone(),
        ..Default::default()
    };
    let maps = ProjectBundleEntityIdMaps {
        project_ids: [(source_project.id, target_project.id)]
            .into_iter()
            .collect(),
        root_project_ids: [(source_project.id, target_project.id)]
            .into_iter()
            .collect(),
        ..Default::default()
    };

    for _ in 0..2 {
        let previous_workflow_id =
            import_traces_with_previous_query_shape(&db, &source_workflow, &source_traces, &maps)
                .await
                .expect("previous LLM trace import must succeed");
        import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
            .await
            .expect("batched LLM trace import must succeed");
        assert_traces_match(
            &db,
            previous_workflow_id,
            target_project.id,
            target_project.id,
            &source_traces,
        )
        .await;
        let current_workflow = workflow_run::Entity::find()
            .filter(workflow_run::Column::ProjectId.eq(Some(target_project.id)))
            .filter(workflow_run::Column::WorkflowKind.eq("chapter_analysis"))
            .order_by_desc(workflow_run::Column::Id)
            .one(&db)
            .await
            .expect("batched workflow query must succeed")
            .expect("batched workflow must be present");
        assert_traces_match(
            &db,
            current_workflow.id,
            target_project.id,
            target_project.id,
            &source_traces,
        )
        .await;
    }

    let mut previous_samples = Vec::with_capacity(SAMPLES);
    let mut current_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let previous_workflow_id = import_traces_with_previous_query_shape(
                &db,
                &source_workflow,
                &source_traces,
                &maps,
            )
            .await
            .expect("previous LLM trace import must succeed");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_traces_match(
                &db,
                previous_workflow_id,
                target_project.id,
                target_project.id,
                &source_traces,
            )
            .await;

            let started = Instant::now();
            import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
                .await
                .expect("batched LLM trace import must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            let current_workflow = workflow_run::Entity::find()
                .filter(workflow_run::Column::ProjectId.eq(Some(target_project.id)))
                .filter(workflow_run::Column::WorkflowKind.eq("chapter_analysis"))
                .order_by_desc(workflow_run::Column::Id)
                .one(&db)
                .await
                .expect("batched workflow query must succeed")
                .expect("batched workflow must be present");
            assert_traces_match(
                &db,
                current_workflow.id,
                target_project.id,
                target_project.id,
                &source_traces,
            )
            .await;
        } else {
            let started = Instant::now();
            import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
                .await
                .expect("batched LLM trace import must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            let current_workflow = workflow_run::Entity::find()
                .filter(workflow_run::Column::ProjectId.eq(Some(target_project.id)))
                .filter(workflow_run::Column::WorkflowKind.eq("chapter_analysis"))
                .order_by_desc(workflow_run::Column::Id)
                .one(&db)
                .await
                .expect("batched workflow query must succeed")
                .expect("batched workflow must be present");
            assert_traces_match(
                &db,
                current_workflow.id,
                target_project.id,
                target_project.id,
                &source_traces,
            )
            .await;

            let started = Instant::now();
            let previous_workflow_id = import_traces_with_previous_query_shape(
                &db,
                &source_workflow,
                &source_traces,
                &maps,
            )
            .await
            .expect("previous LLM trace import must succeed");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_traces_match(
                &db,
                previous_workflow_id,
                target_project.id,
                target_project.id,
                &source_traces,
            )
            .await;
        }
    }

    let previous_p50 = percentile(&previous_samples, 0.50);
    let previous_p95 = percentile(&previous_samples, 0.95);
    let current_p50 = percentile(&current_samples, 0.50);
    let current_p95 = percentile(&current_samples, 0.95);
    let p50_improvement = 1.0 - current_p50 / previous_p50;
    let p95_improvement = 1.0 - current_p95 / previous_p95;
    println!(
        "bundle_history_import_llm_traces traces={LLM_CALL_TRACE_COUNT} previous_sql={} batched_sql=4 p50_ms={previous_p50:.3}->{current_p50:.3} ({:.1}%) p95_ms={previous_p95:.3}->{current_p95:.3} ({:.1}%)",
        LLM_CALL_TRACE_COUNT + 3,
        p50_improvement * 100.0,
        p95_improvement * 100.0,
    );
    assert!(
        p50_improvement >= 0.30,
        "P50 improvement must meet 30%: {p50_improvement:.1}%"
    );
    assert!(
        p95_improvement >= 0.30,
        "P95 improvement must meet 30%: {p95_improvement:.1}%"
    );
}

#[tokio::test]
#[ignore = "manual PostgreSQL benchmark; needs isolated A4N_TEST_DATABASE_URL"]
async fn performance_project_bundle_history_batches_generation_targets() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let user = create_user_summary(
        &db,
        &format!("bundle-targets-{suffix}"),
        &format!("bundle-targets-{suffix}@example.com"),
        "password",
        "user",
    )
    .await
    .expect("test user must be created");
    let source_project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: format!("Target history source {suffix}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("source project must be created");
    let target_project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: format!("Target history target {suffix}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("target project must be created");
    let now = Utc::now().naive_utc();
    let source_workflow = workflow_run::ActiveModel {
        business_key: Set(format!("performance-targets:{suffix}")),
        workflow_kind: Set("chapter_analysis".to_string()),
        project_id: Set(Some(source_project.id)),
        root_project_id: Set(Some(source_project.id)),
        status: Set("completed".to_string()),
        can_resume: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        completed_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source workflow must be created");
    let source_generation = generation_run::ActiveModel {
        project_id: Set(source_project.id),
        root_project_id: Set(source_project.id),
        workflow: Set("chapter_analysis".to_string()),
        step_key: Set("analysis".to_string()),
        title: Set("Chapter analysis".to_string()),
        model_kind: Set("llm".to_string()),
        status: Set("completed".to_string()),
        workflow_run_id: Set(Some(source_workflow.id)),
        attempt: Set(1),
        created_at: Set(now),
        completed_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source generation run must be created");
    let source_targets = (0..GENERATION_TARGET_COUNT)
        .map(|index| generation_run_target::ActiveModel {
            generation_run_id: Set(source_generation.id),
            target_kind: Set("chapter".to_string()),
            target_table: Set("chapter".to_string()),
            target_id: Set(index as i64 + 1),
            role: Set(if index % 2 == 0 { "primary" } else { "related" }.to_string()),
            created_at: Set(now + chrono::Duration::microseconds((index / 2) as i64)),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    generation_run_target::Entity::insert_many(source_targets)
        .exec(&db)
        .await
        .expect("source generation targets must be created");
    let source_targets = generation_run_target::Entity::find()
        .filter(generation_run_target::Column::GenerationRunId.eq(source_generation.id))
        .order_by_asc(generation_run_target::Column::Id)
        .all(&db)
        .await
        .expect("source generation targets must be read");
    assert_eq!(source_targets.len(), GENERATION_TARGET_COUNT);
    let target_id_map = (1..=GENERATION_TARGET_COUNT as i64)
        .map(|id| (id, id + 100_000))
        .collect::<std::collections::HashMap<_, _>>();
    let rows = ProjectBundleHistoryRows {
        workflow_runs: vec![source_workflow.clone()],
        generation_runs: vec![source_generation.clone()],
        generation_run_targets: source_targets.clone(),
        ..Default::default()
    };
    let maps = ProjectBundleEntityIdMaps {
        project_ids: [(source_project.id, target_project.id)]
            .into_iter()
            .collect(),
        root_project_ids: [(source_project.id, target_project.id)]
            .into_iter()
            .collect(),
        chapter_ids: target_id_map,
        ..Default::default()
    };

    for _ in 0..2 {
        let (previous_workflow_id, previous_generation_id) =
            import_generation_targets_with_previous_query_shape(
                &db,
                &source_workflow,
                &source_generation,
                &source_targets,
                &maps,
            )
            .await
            .expect("previous generation target import must succeed");
        import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
            .await
            .expect("batched generation target import must succeed");
        assert_generation_targets_match(&db, previous_generation_id, &source_targets, &maps).await;
        let current_workflow = workflow_run::Entity::find()
            .filter(workflow_run::Column::ProjectId.eq(Some(target_project.id)))
            .filter(workflow_run::Column::WorkflowKind.eq("chapter_analysis"))
            .order_by_desc(workflow_run::Column::Id)
            .one(&db)
            .await
            .expect("batched workflow query must succeed")
            .expect("batched workflow must be present");
        let current_generation = generation_run::Entity::find()
            .filter(generation_run::Column::WorkflowRunId.eq(Some(current_workflow.id)))
            .order_by_desc(generation_run::Column::Id)
            .one(&db)
            .await
            .expect("batched generation query must succeed")
            .expect("batched generation must be present");
        assert_ne!(current_workflow.id, previous_workflow_id);
        assert_generation_targets_match(&db, current_generation.id, &source_targets, &maps).await;
    }

    let mut previous_samples = Vec::with_capacity(SAMPLES);
    let mut current_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let (_, previous_generation_id) = import_generation_targets_with_previous_query_shape(
                &db,
                &source_workflow,
                &source_generation,
                &source_targets,
                &maps,
            )
            .await
            .expect("previous generation target import must succeed");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_generation_targets_match(&db, previous_generation_id, &source_targets, &maps)
                .await;

            let started = Instant::now();
            import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
                .await
                .expect("batched generation target import must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            let current_workflow = workflow_run::Entity::find()
                .filter(workflow_run::Column::ProjectId.eq(Some(target_project.id)))
                .filter(workflow_run::Column::WorkflowKind.eq("chapter_analysis"))
                .order_by_desc(workflow_run::Column::Id)
                .one(&db)
                .await
                .expect("batched workflow query must succeed")
                .expect("batched workflow must be present");
            let current_generation = generation_run::Entity::find()
                .filter(generation_run::Column::WorkflowRunId.eq(Some(current_workflow.id)))
                .order_by_desc(generation_run::Column::Id)
                .one(&db)
                .await
                .expect("batched generation query must succeed")
                .expect("batched generation must be present");
            assert_generation_targets_match(&db, current_generation.id, &source_targets, &maps)
                .await;
        } else {
            let started = Instant::now();
            import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
                .await
                .expect("batched generation target import must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            let current_workflow = workflow_run::Entity::find()
                .filter(workflow_run::Column::ProjectId.eq(Some(target_project.id)))
                .filter(workflow_run::Column::WorkflowKind.eq("chapter_analysis"))
                .order_by_desc(workflow_run::Column::Id)
                .one(&db)
                .await
                .expect("batched workflow query must succeed")
                .expect("batched workflow must be present");
            let current_generation = generation_run::Entity::find()
                .filter(generation_run::Column::WorkflowRunId.eq(Some(current_workflow.id)))
                .order_by_desc(generation_run::Column::Id)
                .one(&db)
                .await
                .expect("batched generation query must succeed")
                .expect("batched generation must be present");
            assert_generation_targets_match(&db, current_generation.id, &source_targets, &maps)
                .await;

            let started = Instant::now();
            let (_, previous_generation_id) = import_generation_targets_with_previous_query_shape(
                &db,
                &source_workflow,
                &source_generation,
                &source_targets,
                &maps,
            )
            .await
            .expect("previous generation target import must succeed");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_generation_targets_match(&db, previous_generation_id, &source_targets, &maps)
                .await;
        }
    }

    let previous_p50 = percentile(&previous_samples, 0.50);
    let previous_p95 = percentile(&previous_samples, 0.95);
    let current_p50 = percentile(&current_samples, 0.50);
    let current_p95 = percentile(&current_samples, 0.95);
    let p50_improvement = 1.0 - current_p50 / previous_p50;
    let p95_improvement = 1.0 - current_p95 / previous_p95;
    println!(
        "bundle_history_import_generation_targets targets={GENERATION_TARGET_COUNT} previous_sql={} batched_sql=5 p50_ms={previous_p50:.3}->{current_p50:.3} ({:.1}%) p95_ms={previous_p95:.3}->{current_p95:.3} ({:.1}%)",
        GENERATION_TARGET_COUNT + 4,
        p50_improvement * 100.0,
        p95_improvement * 100.0,
    );
    assert!(
        p50_improvement >= 0.30,
        "P50 improvement must meet 30%: {p50_improvement:.1}%"
    );
    assert!(
        p95_improvement >= 0.30,
        "P95 improvement must meet 30%: {p95_improvement:.1}%"
    );
}

#[tokio::test]
#[ignore = "manual PostgreSQL benchmark; needs isolated A4N_TEST_DATABASE_URL"]
async fn performance_project_bundle_imports_workflow_checkpoints() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let user = create_user_summary(
        &db,
        &format!("bundle-checkpoints-{suffix}"),
        &format!("bundle-checkpoints-{suffix}@example.com"),
        "password",
        "user",
    )
    .await
    .expect("test user must be created");
    let source_project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: format!("Checkpoint history source {suffix}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("source project must be created");
    let target_project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: format!("Checkpoint history target {suffix}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("target project must be created");
    let now = Utc::now()
        .naive_utc()
        .with_nanosecond(0)
        .expect("whole second timestamp must be valid");
    let source_workflow = workflow_run::ActiveModel {
        business_key: Set(format!("performance-checkpoints:{suffix}")),
        workflow_kind: Set("chapter_analysis".to_string()),
        project_id: Set(Some(source_project.id)),
        root_project_id: Set(Some(source_project.id)),
        status: Set("completed".to_string()),
        can_resume: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        completed_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source workflow must be created");

    let checkpoint_payload = "x".repeat(CHECKPOINT_PAYLOAD_BYTES);
    let artifact_payload = "y".repeat(CHECKPOINT_PAYLOAD_BYTES / 4);
    let fingerprint_payload = "z".repeat(CHECKPOINT_PAYLOAD_BYTES / 4);
    let checkpoints = (0..WORKFLOW_CHECKPOINT_COUNT)
        .map(|index| {
            let fingerprint = serde_json::json!({
                "project_id": source_project.id,
                "root_project_id": source_project.id,
                "request": fingerprint_payload,
            });
            let fingerprint_sha256 =
                a4n_db::generation::generation_checkpoint_fingerprint_sha256(&fingerprint)
                    .expect("checkpoint fingerprint must be computable");
            workflow_checkpoint::Model {
                id: index as i64 + 1,
                workflow_run_id: source_workflow.id,
                step_key: format!("chapter_analysis.step_{index}"),
                step_order: index as i32,
                status: "completed".to_string(),
                input_fingerprint_sha256: Some(fingerprint_sha256),
                input_fingerprint_json: Some(fingerprint),
                checkpoint_json: Some(serde_json::json!({
                    "project_id": source_project.id,
                    "root_project_id": source_project.id,
                    "payload": checkpoint_payload,
                })),
                artifacts_json: Some(serde_json::json!({
                    "project_id": source_project.id,
                    "summary": artifact_payload,
                })),
                generation_run_ids_json: Some(serde_json::json!([])),
                error_message: None,
                created_at: now + chrono::Duration::milliseconds(index as i64),
                updated_at: now + chrono::Duration::milliseconds(index as i64),
                completed_at: Some(now + chrono::Duration::milliseconds(index as i64)),
            }
        })
        .collect::<Vec<_>>();
    let rows = ProjectBundleHistoryRows {
        workflow_runs: vec![source_workflow.clone()],
        workflow_checkpoints: checkpoints.clone(),
        ..Default::default()
    };
    let maps = ProjectBundleEntityIdMaps {
        project_ids: [(source_project.id, target_project.id)]
            .into_iter()
            .collect(),
        root_project_ids: [(source_project.id, target_project.id)]
            .into_iter()
            .collect(),
        ..Default::default()
    };
    let input_bytes = serde_json::to_vec(&checkpoints)
        .expect("checkpoint rows must serialize")
        .len();

    for _ in 0..2 {
        let previous_workflow_id = import_checkpoints_with_previous_query_shape(
            &db,
            &source_workflow,
            &checkpoints,
            &maps,
        )
        .await
        .expect("previous checkpoint import must succeed");
        import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
            .await
            .expect("project bundle checkpoint import must succeed");
        let current_workflow = workflow_run::Entity::find()
            .filter(workflow_run::Column::ProjectId.eq(Some(target_project.id)))
            .order_by_desc(workflow_run::Column::Id)
            .one(&db)
            .await
            .expect("target workflow query must succeed")
            .expect("target workflow must exist");
        assert_checkpoints_match(
            &db,
            previous_workflow_id,
            &checkpoints,
            source_project.id,
            target_project.id,
        )
        .await;
        assert_checkpoints_match(
            &db,
            current_workflow.id,
            &checkpoints,
            source_project.id,
            target_project.id,
        )
        .await;
    }

    let mut previous_samples = Vec::with_capacity(SAMPLES);
    let mut current_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let previous_workflow_id = import_checkpoints_with_previous_query_shape(
                &db,
                &source_workflow,
                &checkpoints,
                &maps,
            )
            .await
            .expect("previous checkpoint import must succeed");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_checkpoints_match(
                &db,
                previous_workflow_id,
                &checkpoints,
                source_project.id,
                target_project.id,
            )
            .await;

            let started = Instant::now();
            import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
                .await
                .expect("project bundle checkpoint import must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            let current_workflow = workflow_run::Entity::find()
                .filter(workflow_run::Column::ProjectId.eq(Some(target_project.id)))
                .order_by_desc(workflow_run::Column::Id)
                .one(&db)
                .await
                .expect("target workflow query must succeed")
                .expect("target workflow must exist");
            assert_checkpoints_match(
                &db,
                current_workflow.id,
                &checkpoints,
                source_project.id,
                target_project.id,
            )
            .await;
        } else {
            let started = Instant::now();
            import_project_bundle_history(&db, &rows, &maps, &suffix, user.id)
                .await
                .expect("project bundle checkpoint import must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            let current_workflow = workflow_run::Entity::find()
                .filter(workflow_run::Column::ProjectId.eq(Some(target_project.id)))
                .order_by_desc(workflow_run::Column::Id)
                .one(&db)
                .await
                .expect("target workflow query must succeed")
                .expect("target workflow must exist");
            assert_checkpoints_match(
                &db,
                current_workflow.id,
                &checkpoints,
                source_project.id,
                target_project.id,
            )
            .await;

            let started = Instant::now();
            let previous_workflow_id = import_checkpoints_with_previous_query_shape(
                &db,
                &source_workflow,
                &checkpoints,
                &maps,
            )
            .await
            .expect("previous checkpoint import must succeed");
            previous_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_checkpoints_match(
                &db,
                previous_workflow_id,
                &checkpoints,
                source_project.id,
                target_project.id,
            )
            .await;
        }
    }

    println!(
        "bundle_history_import_workflow_checkpoints checkpoints={WORKFLOW_CHECKPOINT_COUNT} input_json_bytes={input_bytes} p50_ms={:.3}/{:.3} p95_ms={:.3}/{:.3}",
        percentile(&previous_samples, 0.50),
        percentile(&current_samples, 0.50),
        percentile(&previous_samples, 0.95),
        percentile(&current_samples, 0.95),
    );
}
