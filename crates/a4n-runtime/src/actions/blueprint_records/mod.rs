mod diffing;
mod restore_snapshot;

use crate::AppState;
use crate::actions::access;
use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use crate::actions::template_variables::parse_template_variables_app_error as parse_template_variables;
use a4n_core::error::AppError;
use a4n_core::event::bridge::{EventBus, InternalEvent};
use a4n_core::models::project::{blueprint_version, novel_blueprint};
use a4n_services::blueprint::{BLUEPRINT_GENERATE_PROGRESS_TOTAL, BlueprintGenerationPreviewStep};
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED,
    WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use diffing::json_diff;
use restore_snapshot::{
    RestoredBlueprintFields, restore_blueprint_character_snapshot,
    restore_blueprint_relationship_snapshot, sync_project_after_blueprint_restore,
};

pub type BlueprintProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintSummary {
    pub id: i64,
    pub project_id: i64,
    pub title: String,
    pub genre: String,
    pub style: Option<String>,
    pub tone: Option<String>,
    pub target_audience: Option<String>,
    pub one_sentence_summary: Option<String>,
    pub full_synopsis: Option<String>,
    pub world_setting: Option<Value>,
    pub current_version: String,
    pub needs_part_outlines: bool,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub avatar_svg: Option<String>,
    pub avatar_animal: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintVersionSummary {
    pub id: i64,
    pub project_id: i64,
    pub version: String,
    pub change_description: Option<String>,
    pub blueprint_data: Value,
    pub world_setting_snapshot: Option<Value>,
    pub characters_snapshot: Option<Value>,
    pub relationships_snapshot: Option<Value>,
    pub synopsis_snapshot: Option<String>,
    pub created_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintDiffSummary {
    pub version_a: String,
    pub version_b: String,
    pub diff: Value,
}

#[derive(Debug, Clone, Copy)]
pub struct BlueprintTaskScope {
    pub project_id: i64,
    pub total: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateBlueprintInput {
    pub project_id: i64,
    pub title: String,
    pub genre: String,
    pub target_length: i32,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefineBlueprintInput {
    pub project_id: i64,
    pub focus: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffBlueprintVersionsInput {
    pub project_id: i64,
    pub version_a: String,
    pub version_b: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreBlueprintVersionInput {
    pub project_id: i64,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateWorldSettingInput {
    pub project_id: i64,
    pub world_setting: Value,
}

fn blueprint_summary(bp: &novel_blueprint::Model) -> BlueprintSummary {
    BlueprintSummary {
        id: bp.id,
        project_id: bp.project_id,
        title: bp.title.clone(),
        genre: bp.genre.clone(),
        style: bp.style.clone(),
        tone: bp.tone.clone(),
        target_audience: bp.target_audience.clone(),
        one_sentence_summary: bp.one_sentence_summary.clone(),
        full_synopsis: bp.full_synopsis.clone(),
        world_setting: bp.world_setting.clone(),
        current_version: bp.current_version.clone(),
        needs_part_outlines: bp.needs_part_outlines,
        total_chapters: bp.total_chapters,
        chapters_per_part: bp.chapters_per_part,
        avatar_svg: bp.avatar_svg.clone(),
        avatar_animal: bp.avatar_animal.clone(),
        created_at: Some(bp.created_at.to_string()),
        updated_at: Some(bp.updated_at.to_string()),
    }
}

fn blueprint_version_summary(v: &blueprint_version::Model) -> BlueprintVersionSummary {
    BlueprintVersionSummary {
        id: v.id,
        project_id: v.project_id,
        version: v.version.clone(),
        change_description: v.change_description.clone(),
        blueprint_data: v.blueprint_data.clone(),
        world_setting_snapshot: v.world_setting_snapshot.clone(),
        characters_snapshot: v.characters_snapshot.clone(),
        relationships_snapshot: v.relationships_snapshot.clone(),
        synopsis_snapshot: v.synopsis_snapshot.clone(),
        created_at: Some(v.created_at.to_string()),
        created_by: v.created_by.clone(),
    }
}

pub async fn generate_blueprint_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GenerateBlueprintInput,
) -> Result<BlueprintTaskScope, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    Ok(BlueprintTaskScope {
        project_id: input.project_id,
        total: BLUEPRINT_GENERATE_PROGRESS_TOTAL,
    })
}

pub async fn get_blueprint(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Option<BlueprintSummary>, AppError> {
    access::verify_project_access(state, user_id, project_id).await?;
    Ok(state
        .blueprint
        .get(project_id)
        .await?
        .map(|blueprint| blueprint_summary(&blueprint)))
}

pub async fn generate_blueprint(
    state: &AppState,
    user_id: i64,
    input: GenerateBlueprintInput,
    cancel_token: Option<&CancellationToken>,
    progress: Option<BlueprintProgressReporter<'_>>,
) -> Result<BlueprintSummary, AppError> {
    generate_blueprint_task_scope(state, user_id, &input).await?;
    report(
        progress,
        0,
        BLUEPRINT_GENERATE_PROGRESS_TOTAL,
        "启动蓝图生成",
    )?;
    let variables = parse_template_variables(&input.variables)?;
    let progress_adapter = |current: u32, total: u32, message: &str| {
        let _ = report(progress, current, total, message);
    };
    let service_progress =
        progress.map(|_| &progress_adapter as &(dyn Fn(u32, u32, &str) + Send + Sync));
    let workflow_fingerprint = json!({
        "schema_version": 1,
        "workflow": "blueprint_generation",
        "project_id": input.project_id,
        "title": &input.title,
        "genre": &input.genre,
        "target_length": input.target_length,
        "variables": &variables,
    });
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!("blueprint_generation:project:{}", input.project_id),
            workflow_kind: "blueprint_generation".to_string(),
            project_id: Some(input.project_id),
            root_project_id: Some(input.project_id),
            source_snapshot_id: None,
            scope_table: Some("project".to_string()),
            scope_id: Some(input.project_id),
            chapter_id: None,
            mode: Some("generate".to_string()),
            input_fingerprint_json: Some(workflow_fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("blueprint_generation.story_core".to_string()),
            active_task_id: input.task_id.clone(),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    let workflow_run_id = workflow_run.id;
    let blueprint_result = state
        .blueprint
        .generate(a4n_services::blueprint::BlueprintGenerateInput {
            project_id: input.project_id,
            novel_title: &input.title,
            genre: &input.genre,
            target_length: input.target_length,
            llm: state.llm.as_ref(),
            prompt_engine: state.engine.as_ref(),
            variables: &variables,
            cancel_token,
            progress: service_progress,
            workflow_run_id: Some(workflow_run_id),
        })
        .await;
    match blueprint_result {
        Ok(blueprint) => {
            state
                .workflow_run
                .mark_status(workflow_run_id, WORKFLOW_STATUS_COMPLETED, false, None)
                .await?;
            Ok(blueprint_summary(&blueprint))
        }
        Err(err) => {
            let status = if matches!(err, AppError::Cancelled) {
                WORKFLOW_STATUS_CANCELLED
            } else {
                WORKFLOW_STATUS_FAILED
            };
            let _ = state
                .workflow_run
                .mark_status(workflow_run_id, status, true, Some(&err.to_string()))
                .await;
            Err(err)
        }
    }
}

pub async fn preview_generate_blueprint_request(
    state: &AppState,
    user_id: i64,
    input: GenerateBlueprintInput,
) -> Result<ModelRequestPreview, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let stages = state
        .blueprint
        .preview_generate_requests(
            input.project_id,
            &input.title,
            &input.genre,
            input.target_length,
            state.engine.as_ref(),
            &variables,
        )
        .await?;
    chat_status_preview_for_state(
        state,
        "blueprint_generation",
        "蓝图生成请求预览",
        stages.into_iter().map(blueprint_preview_step),
        "llm_chat",
    )
    .await
    .map_err(AppError::InvalidArgument)
}

pub async fn refine_blueprint(
    state: &AppState,
    user_id: i64,
    input: RefineBlueprintInput,
) -> Result<BlueprintSummary, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let blueprint = state
        .blueprint
        .refine(
            input.project_id,
            input.focus.as_deref(),
            state.llm.as_ref(),
            state.engine.as_ref(),
            &variables,
        )
        .await?;
    Ok(blueprint_summary(&blueprint))
}

pub async fn preview_refine_blueprint_request(
    state: &AppState,
    user_id: i64,
    input: RefineBlueprintInput,
) -> Result<ModelRequestPreview, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let request = state
        .blueprint
        .preview_refine_request(
            input.project_id,
            input.focus.as_deref(),
            state.engine.as_ref(),
            &variables,
        )
        .await?;
    chat_status_preview_for_state(
        state,
        "blueprint_refine",
        "蓝图优化请求预览",
        [ChatStatusPreviewStep::llm(
            "blueprint_refine",
            "优化蓝图",
            Some("blueprint_optimization".to_string()),
            "ready",
            None,
            Some(request),
        )],
        "llm_chat",
    )
    .await
    .map_err(AppError::InvalidArgument)
}

pub async fn list_blueprint_versions(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<BlueprintVersionSummary>, AppError> {
    access::verify_project_access(state, user_id, project_id).await?;
    let versions = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project_id))
        .order_by_desc(blueprint_version::Column::CreatedAt)
        .all(state.db.as_ref())
        .await?;
    Ok(versions.iter().map(blueprint_version_summary).collect())
}

pub async fn diff_blueprint_versions(
    state: &AppState,
    user_id: i64,
    input: DiffBlueprintVersionsInput,
) -> Result<BlueprintDiffSummary, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let version_a = load_blueprint_version(state, input.project_id, &input.version_a).await?;
    let version_b = load_blueprint_version(state, input.project_id, &input.version_b).await?;
    Ok(BlueprintDiffSummary {
        version_a: input.version_a,
        version_b: input.version_b,
        diff: json_diff(&version_a.blueprint_data, &version_b.blueprint_data),
    })
}

pub async fn restore_blueprint_version(
    state: &AppState,
    user_id: i64,
    input: RestoreBlueprintVersionInput,
) -> Result<BlueprintSummary, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let version_name = input.version.trim();
    if version_name.is_empty() {
        return Err(AppError::InvalidArgument("版本号不能为空".to_string()));
    }

    let txn = state.db.begin().await?;
    let version = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(input.project_id))
        .filter(blueprint_version::Column::Version.eq(version_name))
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("版本 {version_name} 不存在")))?;

    let now = Utc::now().naive_utc();
    let data = &version.blueprint_data;
    let fields = RestoredBlueprintFields::from_snapshot(data, &version.version)
        .map_err(AppError::InvalidArgument)?;
    let existing = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(input.project_id))
        .one(&txn)
        .await?;

    let restored = if let Some(blueprint) = existing {
        let mut active: novel_blueprint::ActiveModel = blueprint.into();
        fields.apply_to_existing(&mut active, now);
        active.update(&txn).await?
    } else {
        fields
            .into_active_model(input.project_id, now)
            .insert(&txn)
            .await?
    };

    restore_blueprint_character_snapshot(
        &txn,
        input.project_id,
        version
            .characters_snapshot
            .as_ref()
            .or_else(|| data.get("characters")),
        now,
    )
    .await
    .map_err(AppError::InvalidArgument)?;
    restore_blueprint_relationship_snapshot(
        &txn,
        input.project_id,
        version
            .relationships_snapshot
            .as_ref()
            .or_else(|| data.get("relationships")),
        now,
    )
    .await
    .map_err(AppError::InvalidArgument)?;
    let status_change =
        sync_project_after_blueprint_restore(&txn, input.project_id, &restored, data, now)
            .await
            .map_err(AppError::InvalidArgument)?;
    txn.commit().await?;

    if let Some((from, to)) = status_change {
        state
            .event_bus
            .publish(InternalEvent::ProjectStatusChanged {
                project_id: input.project_id,
                from,
                to,
                timestamp: now,
            })
            .await?;
    }
    state
        .event_bus
        .publish(InternalEvent::BlueprintUpdated {
            project_id: input.project_id,
            version: version.version,
        })
        .await?;
    Ok(blueprint_summary(&restored))
}

pub async fn update_world_setting(
    state: &AppState,
    user_id: i64,
    input: UpdateWorldSettingInput,
) -> Result<Value, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let world_setting = required_world_setting_value(&input.world_setting)?;
    let blueprint = state
        .blueprint
        .get(input.project_id)
        .await?
        .ok_or_else(|| AppError::NotFound("蓝图不存在".to_string()))?;

    let mut active: novel_blueprint::ActiveModel = blueprint.into();
    active.world_setting = Set(Some(world_setting.clone()));
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(state.db.as_ref()).await?;
    Ok(world_setting)
}

fn blueprint_preview_step(stage: BlueprintGenerationPreviewStep) -> ChatStatusPreviewStep {
    ChatStatusPreviewStep::llm(
        stage.key,
        stage.title,
        Some(stage.prompt_name),
        stage.state,
        stage.note,
        stage.request,
    )
}

async fn load_blueprint_version(
    state: &AppState,
    project_id: i64,
    version: &str,
) -> Result<blueprint_version::Model, AppError> {
    blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project_id))
        .filter(blueprint_version::Column::Version.eq(version))
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound(format!("版本 {version} 不存在")))
}

fn required_world_setting_value(value: &Value) -> Result<Value, AppError> {
    if value.is_null() {
        return Err(AppError::InvalidArgument(
            "world_setting 不能为 null".to_string(),
        ));
    }
    Ok(value.clone())
}

fn report(
    progress: Option<BlueprintProgressReporter<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(progress) = progress {
        progress(current, total.max(1), message)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn update_world_setting_rejects_null() {
        let err = required_world_setting_value(&Value::Null)
            .expect_err("null world setting should be rejected");

        assert!(err.to_string().contains("world_setting 不能为 null"));
    }

    #[test]
    fn generate_blueprint_input_rejects_unknown_fields() {
        let err = serde_json::from_value::<GenerateBlueprintInput>(json!({
            "project_id": 7,
            "title": "标题",
            "genre": "类型",
            "target_length": 30,
            "projectId": 7
        }))
        .expect_err("unknown fields must fail instead of being ignored");

        assert!(err.to_string().contains("unknown field"));
    }
}
