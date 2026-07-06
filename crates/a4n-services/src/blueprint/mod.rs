//! 故事蓝图服务：蓝图生成/优化/编辑 + 角色提取
//!
//! 对应文档 03-故事蓝图系统

mod generation;
mod generation_contract;
mod json;
mod sync;

use chrono::Utc;
use sea_orm::*;
use serde_json::{Value, json};
use std::sync::Arc;

use crate::generation::context_aware_chat_request_trace_json;
use crate::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    save_completed_llm_call_checkpoint,
};
use crate::llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, LlmContextManifest, context_aware_system_user_request,
};
use crate::source::source_content_sha256;
use crate::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING, WorkflowRunService,
};
use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_core::event::bridge::{EventBus, InternalEvent};
use a4n_core::models::project::{
    blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint, novel_project,
    project_status_history,
};
use a4n_core::state::{status::ProjectStatus, validator::StateValidator};
use a4n_engine::llm::provider::{
    ChatRequest, LlmProvider, LlmTraceContext, is_truncated_finish_reason,
};
use a4n_engine::prompt::engine::PromptEngine;
pub use generation::{BlueprintGenerateInput, BlueprintGenerationPreviewStep};
use generation_contract::blueprint_refine_manifest_user_message;
pub(crate) use json::{CHAPTERS_PER_PART, PART_OUTLINE_THRESHOLD};
use json::{increment_version, parse_blueprint_optimization_json};
pub use sync::{
    BlueprintCharacterDraft, BlueprintIncrementalSyncStats, BlueprintRelationshipDraft,
    sync_blueprint_characters, sync_blueprint_relationships,
    sync_character_profiles_from_blueprint,
};

pub const BLUEPRINT_GENERATE_PROGRESS_TOTAL: u32 = 12;

pub fn needs_part_outlines_for_chapters(total_chapters: i32) -> bool {
    total_chapters > PART_OUTLINE_THRESHOLD
}

pub fn blueprint_model_llm_context(blueprint: &novel_blueprint::Model) -> Value {
    json!({
        "title": blueprint.title,
        "genre": blueprint.genre,
        "style": blueprint.style,
        "tone": blueprint.tone,
        "target_audience": blueprint.target_audience,
        "one_sentence_summary": blueprint.one_sentence_summary,
        "full_synopsis": blueprint.full_synopsis,
        "world_setting": blueprint.world_setting,
        "needs_part_outlines": blueprint.needs_part_outlines,
        "total_chapters": blueprint.total_chapters,
        "chapters_per_part": blueprint.chapters_per_part,
    })
}

pub fn blueprint_character_llm_context(character: &blueprint_character::Model) -> Value {
    json!({
        "name": character.name,
        "character_type": character.character_type,
        "gender": character.gender,
        "age": character.age,
        "appearance": character.appearance,
        "personality": character.personality,
        "background": character.background,
        "motivation": character.motivation,
        "arc_summary": character.arc_summary,
        "relationship_to_protagonist": character.relationship_to_protagonist,
        "arc_stages": character.arc_stages,
    })
}

pub fn blueprint_relationship_llm_context(relationship: &blueprint_relationship::Model) -> Value {
    json!({
        "relationship_type": relationship.relationship_type,
        "character_a": relationship.character_a,
        "character_b": relationship.character_b,
        "description": relationship.description,
        "strength": relationship.strength,
        "evolution": relationship.evolution,
    })
}

pub fn blueprint_llm_context(
    blueprint: &novel_blueprint::Model,
    characters: &[blueprint_character::Model],
    relationships: &[blueprint_relationship::Model],
) -> Value {
    json!({
        "blueprint": blueprint_model_llm_context(blueprint),
        "characters": characters
            .iter()
            .map(blueprint_character_llm_context)
            .collect::<Vec<_>>(),
        "relationships": relationships
            .iter()
            .map(blueprint_relationship_llm_context)
            .collect::<Vec<_>>(),
    })
}

pub fn validate_blueprint_chapter_count(value: i32, context: &str) -> Result<(), AppError> {
    json::validate_total_chapter_count(value, context)
}

pub(crate) fn validate_blueprint_world_setting_payload(value: &Value) -> Result<(), AppError> {
    json::validate_world_setting_payload(value)
}

pub(crate) fn validate_blueprint_characters_payload(value: &Value) -> Result<(), AppError> {
    json::validate_characters_payload(value)
}

pub(crate) fn validate_blueprint_relationships_payload(value: &Value) -> Result<(), AppError> {
    json::validate_relationships_payload(value)
}

fn project_status_after_blueprint_generation(previous: ProjectStatus) -> ProjectStatus {
    if previous.ordinal() < ProjectStatus::BlueprintReady.ordinal() {
        ProjectStatus::BlueprintReady
    } else {
        previous
    }
}

fn build_blueprint_refine_request(
    project_id: i64,
    system_prompt: String,
    user_prompt: String,
    blueprint_context: &Value,
) -> Result<ChatRequest, AppError> {
    let dynamic_user_content_sha256 = source_content_sha256(&user_prompt);
    let manifest = build_blueprint_refine_context_manifest(blueprint_context, user_prompt)?;
    Ok(context_aware_system_user_request(
        system_prompt,
        manifest,
        Some(0.6),
        None,
        Some(llm_token_profiles::BLUEPRINT.into()),
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: None,
        workflow: "blueprint_refine".to_string(),
        step_key: "blueprint_refine".to_string(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some("blueprint_refine".to_string()),
        input_fingerprint_sha256: Some(source_content_sha256(&serde_json::to_string(&json!({
            "blueprint_context": blueprint_context,
            "dynamic_user_content_sha256": dynamic_user_content_sha256,
        }))?)),
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}

fn build_blueprint_refine_context_manifest(
    blueprint_context: &Value,
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    let mut ledger = LlmContextLedger::new();
    let blueprint_hash = source_content_sha256(&serde_json::to_string(blueprint_context)?);
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: ("project").into(),
        pack_kind: ("existing_blueprint_context").into(),
        source_refs_json: Some(json!({
            "source": "novel_blueprint",
            "content_sha256": blueprint_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "blueprint_refine_context_v1",
            "kind": "existing_blueprint_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "existing_blueprint_context",
                    "blueprint_context": blueprint_context,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "existing_blueprint_context",
                    "blueprint_context": blueprint_context,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "existing_blueprint_context",
                    "blueprint_context": minimal_blueprint_refine_context(blueprint_context),
                }),
            ),
        ],
    })?;
    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(
            "蓝图优化必须返回严格 JSON 对象；允许字段为 title, genre, style, tone, target_audience, one_sentence_summary, full_synopsis, world_setting, characters, relationships。"
                .to_string(),
        ),
    ))
}

fn minimal_blueprint_refine_context(blueprint_context: &Value) -> Value {
    let Some(object) = blueprint_context.as_object() else {
        return blueprint_context.clone();
    };
    let mut minimal = serde_json::Map::new();
    if let Some(blueprint) = object.get("blueprint") {
        minimal.insert(
            "blueprint".to_string(),
            minimal_blueprint_model_context(blueprint),
        );
    }
    if let Some(characters) = object.get("characters") {
        minimal.insert(
            "character_count".to_string(),
            json!(characters.as_array().map_or(0, Vec::len)),
        );
    }
    if let Some(relationships) = object.get("relationships") {
        minimal.insert(
            "relationship_count".to_string(),
            json!(relationships.as_array().map_or(0, Vec::len)),
        );
    }
    Value::Object(minimal)
}

fn minimal_blueprint_model_context(blueprint: &Value) -> Value {
    let Some(object) = blueprint.as_object() else {
        return blueprint.clone();
    };
    let mut minimal = serde_json::Map::new();
    for key in [
        "title",
        "genre",
        "style",
        "tone",
        "target_audience",
        "one_sentence_summary",
        "full_synopsis",
        "total_chapters",
        "chapters_per_part",
    ] {
        if let Some(value) = object.get(key) {
            minimal.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(minimal)
}

fn ensure_blueprint_response_not_truncated(
    finish_reason: Option<&str>,
    label: &str,
) -> Result<(), AppError> {
    if is_truncated_finish_reason(finish_reason) {
        return Err(AppError::InvalidArgument(format!(
            "{label} 输出达到当前蓝图 max_tokens 上限，返回 JSON 被截断；请在设置页调大“蓝图”输出档位，或减少本次蓝图输入/输出规模后重试"
        )));
    }
    Ok(())
}

fn blueprint_refine_checkpoint_response(
    checkpoint_json: &Value,
    call_key: &str,
) -> Result<(String, Option<Value>, Option<String>), AppError> {
    let response_text = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("{call_key}.response_content 缺失或不是字符串"),
            operation: "恢复蓝图优化 LLM 调用断点".to_string(),
        })?;
    let parsed = parse_blueprint_optimization_json(&response_text)?;
    let parsed_output =
        checkpoint_json
            .get("parsed_output")
            .ok_or_else(|| AppError::InvalidState {
                current: format!("{call_key}.parsed_output 缺失"),
                operation: "恢复蓝图优化 LLM 调用断点".to_string(),
            })?;
    if parsed_output != &parsed.raw {
        return Err(AppError::InvalidState {
            current: format!(
                "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
            ),
            operation: "恢复蓝图优化 LLM 调用断点".to_string(),
        });
    }
    let usage = checkpoint_json
        .get("usage")
        .filter(|value| !value.is_null())
        .cloned();
    let finish_reason =
        match checkpoint_json.get("finish_reason") {
            Some(Value::Null) | None => None,
            Some(value) => Some(value.as_str().map(ToString::to_string).ok_or_else(|| {
                AppError::InvalidState {
                    current: format!("{call_key}.finish_reason 必须是字符串或 null"),
                    operation: "恢复蓝图优化 LLM 调用断点".to_string(),
                }
            })?),
        };
    ensure_blueprint_response_not_truncated(finish_reason.as_deref(), "蓝图优化 checkpoint")?;
    Ok((response_text, usage, finish_reason))
}

async fn finalize_blueprint_refine_workflow(
    workflow: &WorkflowRunService,
    workflow_run_id: i64,
    result: &Result<novel_blueprint::Model, AppError>,
) -> Result<(), AppError> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(err) => (WORKFLOW_STATUS_FAILED, true, Some(err.to_string())),
    };
    workflow
        .mark_status(
            workflow_run_id,
            status,
            can_resume,
            error_message.as_deref(),
        )
        .await
        .map(|_| ())
}

#[derive(Clone)]
pub struct BlueprintService {
    db: Arc<DatabaseConnection>,
    event_bus: Arc<dyn EventBus>,
}

impl BlueprintService {
    pub fn new(db: Arc<DatabaseConnection>, event_bus: Arc<dyn EventBus>) -> Self {
        Self { db, event_bus }
    }

    pub async fn get(&self, project_id: i64) -> Result<Option<novel_blueprint::Model>, AppError> {
        novel_blueprint::Entity::find()
            .filter(novel_blueprint::Column::ProjectId.eq(project_id))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn refine(
        &self,
        project_id: i64,
        focus: Option<&str>,
        llm: &dyn LlmProvider,
        prompt_engine: &dyn PromptEngine,
        variables: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<novel_blueprint::Model, AppError> {
        let bp = self
            .get(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("蓝图不存在".into()))?;
        let characters = blueprint_character::Entity::find()
            .filter(blueprint_character::Column::ProjectId.eq(project_id))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let relationships = blueprint_relationship::Entity::find()
            .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let blueprint_context = blueprint_llm_context(&bp, &characters, &relationships);
        let next_version = self
            .next_version(project_id, Some(bp.current_version.as_str()))
            .await?;

        let prompt_content =
            prompt_engine.render("blueprint_optimization", variables, Some(project_id))?;
        let user_message =
            blueprint_refine_manifest_user_message(prompt_engine, Some(project_id), focus)?;
        let mut request = build_blueprint_refine_request(
            project_id,
            prompt_content,
            user_message,
            &blueprint_context,
        )?;
        let request_json = context_aware_chat_request_trace_json(&request)?;
        let input_fingerprint_json = json!({
            "prompt_names": ["blueprint_optimization", "blueprint_optimization_user"],
            "request": request_json,
            "input_variables": variables,
            "focus": focus,
            "source_refs": {
                "blueprint_id": bp.id,
                "current_version": &bp.current_version,
                "blueprint_context": &blueprint_context,
            },
        });
        let call_key = format!("blueprint_refine:project:{project_id}");
        let workflow = WorkflowRunService::new(self.db.clone());
        let workflow_run = workflow
            .upsert_run(UpsertWorkflowRunInput {
                business_key: format!("blueprint_refine:{project_id}"),
                workflow_kind: "blueprint_refine".to_string(),
                project_id: Some(project_id),
                root_project_id: None,
                source_snapshot_id: None,
                scope_table: Some("novel_blueprint".to_string()),
                scope_id: Some(bp.id),
                chapter_id: None,
                mode: focus.map(str::to_string),
                input_fingerprint_json: Some(input_fingerprint_json.clone()),
                status: WORKFLOW_STATUS_RUNNING.to_string(),
                current_step_key: Some(call_key.clone()),
                active_task_id: None,
                can_resume: true,
                resume_block_reason: None,
                error_message: None,
            })
            .await?;
        let workflow_run_id = workflow_run.id;
        let input_fingerprint_sha256 =
            llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)?;
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }
        workflow
            .mark_current_step(workflow_run_id, &call_key)
            .await?;

        let result = async {
            let (response_text, response_usage_json, response_finish_reason, restored) =
                if let Some(checkpoint) = load_completed_llm_call_checkpoint(
                    self.db.clone(),
                    workflow_run_id,
                    &call_key,
                    &input_fingerprint_json,
                )
                .await?
                {
                    let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)?;
                    let (response_text, usage, finish_reason) =
                        blueprint_refine_checkpoint_response(&checkpoint_json, &call_key)?;
                    (response_text, usage, finish_reason, true)
                } else {
                    let response = llm.chat(request).await?;
                    ensure_blueprint_response_not_truncated(
                        response.finish_reason.as_deref(),
                        "蓝图优化",
                    )?;
                    let response_text = response.message.content.clone();
                    let response_usage_json = response
                        .usage
                        .as_ref()
                        .map(serde_json::to_value)
                        .transpose()
                        .map_err(AppError::Serialization)?;
                    let response_finish_reason = response.finish_reason.clone();
                    (
                        response_text,
                        response_usage_json,
                        response_finish_reason,
                        false,
                    )
                };

            let json = parse_blueprint_optimization_json(&response_text)?;
            let optimization_patch = json.raw.clone();
            if !restored {
                save_completed_llm_call_checkpoint(
                    self.db.clone(),
                    CompletedLlmCallCheckpointInput {
                        workflow_run_id,
                        call_key: call_key.clone(),
                        call_order: 1,
                        input_fingerprint_json: input_fingerprint_json.clone(),
                        checkpoint_json: json!({
                            "output": optimization_patch.clone(),
                            "parsed_output": optimization_patch.clone(),
                            "response_content": response_text,
                            "usage": response_usage_json,
                            "finish_reason": response_finish_reason,
                        }),
                        artifacts_json: None,
                    },
                )
                .await?;
            }

            let mut a: novel_blueprint::ActiveModel = bp.into();
            let now = Utc::now().naive_utc();
            if let Some(style) = json.style {
                a.style = Set(Some(style));
            }
            if let Some(full_synopsis) = json.full_synopsis {
                a.full_synopsis = Set(Some(full_synopsis));
            }
            if let Some(world_setting) = json.world_setting {
                a.world_setting = Set(Some(world_setting));
            }
            a.current_version = Set(next_version.clone());
            a.updated_at = Set(now);
            let updated = a
                .update(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;

            if let Some(characters) = json.characters {
                self.replace_characters(project_id, &characters).await?;
            }
            if let Some(relationships) = json.relationships {
                self.replace_relationships(project_id, &relationships)
                    .await?;
            }
            sync_character_profiles_from_blueprint(self.db.as_ref(), project_id, now).await?;

            let updated_characters = blueprint_character::Entity::find()
                .filter(blueprint_character::Column::ProjectId.eq(project_id))
                .all(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
            let updated_relationships = blueprint_relationship::Entity::find()
                .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
                .all(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
            let snapshot = serde_json::json!({
                "title": &updated.title,
                "genre": &updated.genre,
                "style": &updated.style,
                "tone": &updated.tone,
                "target_audience": &updated.target_audience,
                "one_sentence_summary": &updated.one_sentence_summary,
                "full_synopsis": &updated.full_synopsis,
                "world_setting": &updated.world_setting,
                "needs_part_outlines": updated.needs_part_outlines,
                "total_chapters": updated.total_chapters,
                "chapters_per_part": updated.chapters_per_part,
                "characters": updated_characters,
                "relationships": updated_relationships,
                "optimization_patch": optimization_patch,
            });
            self.save_version(
                project_id,
                next_version,
                Some("优化蓝图".to_string()),
                &snapshot,
            )
            .await?;

            Ok(updated)
        }
        .await;

        finalize_blueprint_refine_workflow(&workflow, workflow_run_id, &result).await?;
        result
    }

    pub async fn preview_refine_request(
        &self,
        project_id: i64,
        focus: Option<&str>,
        prompt_engine: &dyn PromptEngine,
        variables: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<ChatRequest, AppError> {
        let bp = self
            .get(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("蓝图不存在".into()))?;
        let characters = blueprint_character::Entity::find()
            .filter(blueprint_character::Column::ProjectId.eq(project_id))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let relationships = blueprint_relationship::Entity::find()
            .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let blueprint_context = blueprint_llm_context(&bp, &characters, &relationships);
        let prompt_content =
            prompt_engine.render("blueprint_optimization", variables, Some(project_id))?;
        let user_message =
            blueprint_refine_manifest_user_message(prompt_engine, Some(project_id), focus)?;
        build_blueprint_refine_request(project_id, prompt_content, user_message, &blueprint_context)
    }

    async fn save_version(
        &self,
        project_id: i64,
        version: String,
        description: Option<String>,
        data: &Value,
    ) -> Result<(), AppError> {
        let now = Utc::now().naive_utc();
        blueprint_version::ActiveModel {
            project_id: Set(project_id),
            version: Set(version.clone()),
            change_description: Set(description),
            blueprint_data: Set(data.clone()),
            world_setting_snapshot: Set(data.get("world_setting").cloned()),
            characters_snapshot: Set(data.get("characters").cloned()),
            relationships_snapshot: Set(data.get("relationships").cloned()),
            synopsis_snapshot: Set(data["full_synopsis"].as_str().map(|s| s.to_string())),
            created_at: Set(now),
            created_by: Set(Some("system".to_string())),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)?;

        self.event_bus
            .publish(InternalEvent::BlueprintUpdated {
                project_id,
                version,
            })
            .await?;

        Ok(())
    }

    async fn replace_characters(
        &self,
        project_id: i64,
        chars: &[json::ParsedBlueprintCharacter],
    ) -> Result<(), AppError> {
        let now = Utc::now().naive_utc();
        let drafts = chars
            .iter()
            .map(|c| BlueprintCharacterDraft {
                name: c.name.clone(),
                character_type: c.character_type.clone(),
                gender: c.gender.clone(),
                age: c.age.clone(),
                appearance: Some(c.appearance.clone()),
                personality: Some(c.personality.clone()),
                background: Some(c.background.clone()),
                motivation: Some(c.motivation.clone()),
                arc_summary: Some(c.arc_summary.clone()),
                relationship_to_protagonist: Some(c.relationship_to_protagonist.clone()),
                arc_stages: c.arc_stages.clone(),
            })
            .collect();
        sync_blueprint_characters(self.db.as_ref(), project_id, drafts, now).await?;
        Ok(())
    }

    async fn replace_relationships(
        &self,
        project_id: i64,
        rels: &[json::ParsedBlueprintRelationship],
    ) -> Result<(), AppError> {
        let now = Utc::now().naive_utc();
        let drafts = rels
            .iter()
            .map(|r| BlueprintRelationshipDraft {
                relationship_type: r.relationship_type.clone(),
                character_a: r.character_a.clone(),
                character_b: r.character_b.clone(),
                description: Some(r.description.clone()),
                strength: r.strength,
                evolution: r.evolution.clone(),
            })
            .collect();
        sync_blueprint_relationships(self.db.as_ref(), project_id, drafts, now).await?;
        Ok(())
    }

    async fn sync_project_after_generation(
        &self,
        project_id: i64,
        blueprint: &novel_blueprint::Model,
        data: &Value,
    ) -> Result<(), AppError> {
        let project = novel_project::Entity::find_by_id(project_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;

        let previous_status = StateValidator::parse_status(&project.status)?;
        let next_status = project_status_after_blueprint_generation(previous_status);
        let now = Utc::now().naive_utc();
        let mut active: novel_project::ActiveModel = project.into();
        active.title = Set(blueprint.title.clone());
        active.status = Set(next_status.as_str().to_string());
        active.target_chapters = Set(blueprint.total_chapters.ok_or_else(|| {
            AppError::InvalidArgument("蓝图缺少 total_chapters，无法同步项目".to_string())
        })?);
        active.total_chapters = Set(blueprint.total_chapters);
        active.chapters_per_part = Set(blueprint.chapters_per_part);
        active.raw_blueprint = Set(Some(serde_json::to_string_pretty(data)?));
        active.updated_at = Set(now);
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        if previous_status != next_status {
            project_status_history::ActiveModel {
                project_id: Set(project_id),
                from_status: Set(previous_status.as_str().to_string()),
                to_status: Set(next_status.as_str().to_string()),
                reason: Set(Some("蓝图生成完成".to_string())),
                changed_by: Set(Some("system".to_string())),
                changed_at: Set(now),
                ..Default::default()
            }
            .insert(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

            self.event_bus
                .publish(InternalEvent::ProjectStatusChanged {
                    project_id,
                    from: previous_status.as_str().to_string(),
                    to: next_status.as_str().to_string(),
                    timestamp: now,
                })
                .await?;
        }

        Ok(())
    }

    async fn next_version(
        &self,
        project_id: i64,
        current_version: Option<&str>,
    ) -> Result<String, AppError> {
        let mut candidate = match current_version {
            Some(current_version) => increment_version(current_version)?,
            None => "1.0".to_string(),
        };

        loop {
            let exists = blueprint_version::Entity::find()
                .filter(blueprint_version::Column::ProjectId.eq(project_id))
                .filter(blueprint_version::Column::Version.eq(&candidate))
                .one(self.db.as_ref())
                .await
                .map_err(AppError::Database)?
                .is_some();
            if !exists {
                return Ok(candidate);
            }
            candidate = increment_version(&candidate)?;
        }
    }
}

#[cfg(test)]
mod tests;
