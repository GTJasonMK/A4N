use std::collections::HashMap;

use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_core::models::project::novel_blueprint;
use a4n_engine::llm::provider::{ChatRequest, ChatResponse, LlmProvider, LlmTraceContext};
use a4n_engine::prompt::engine::PromptEngine;
use chrono::Utc;
use sea_orm::*;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::generation::{
    GenerationRunTraceTarget, RecordCompletedGenerationRunInput, chat_request_messages_json,
    context_aware_chat_request_trace_json, record_completed_generation_run,
    text_response_summary_json,
};
use crate::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    required_llm_call_workflow_run_id, save_completed_llm_call_checkpoint,
};
use crate::llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, LlmContextManifest, context_aware_system_user_request,
};
use crate::workflow::WorkflowRunService;

use super::generation_contract::{
    CHARACTERS_STAGE_KEYS, RELATIONSHIPS_STAGE_KEYS, STORY_CORE_STAGE_KEYS, WORLD_STAGE_KEYS,
    assemble_staged_blueprint_json, blueprint_generation_source_hash,
    build_blueprint_stage_manifest_user_message, parse_stage_json_object,
};
use super::json::{
    build_generation_payload, parse_blueprint_json, validate_characters_stage,
    validate_relationships_stage, validate_story_core_stage, validate_world_stage,
};
use super::{
    BLUEPRINT_GENERATE_PROGRESS_TOTAL, BlueprintService, ensure_blueprint_response_not_truncated,
};

const BLUEPRINT_STAGE_SYSTEM_PROMPT: &str = "blueprint_stage_system";
const BLUEPRINT_STAGE_PREVIEW_PROMPT_NAME: &str =
    "blueprint + blueprint_stage_system + blueprint_stage_user";
const BLUEPRINT_CONTEXT_LEDGER_VERSION: &str = "blueprint_context_ledger_v1";
#[derive(Debug, Clone)]
pub struct BlueprintGenerationPreviewStep {
    pub key: String,
    pub title: String,
    pub prompt_name: String,
    pub state: String,
    pub note: Option<String>,
    pub request: Option<ChatRequest>,
}

pub struct BlueprintGenerateInput<'a> {
    pub project_id: i64,
    pub novel_title: &'a str,
    pub genre: &'a str,
    pub target_length: i32,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a HashMap<String, Value>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub progress: BlueprintProgressReporter<'a>,
    pub workflow_run_id: Option<i64>,
}

pub(crate) struct BlueprintStageRequestInput<'a> {
    pub(crate) prompt_engine: &'a dyn PromptEngine,
    pub(crate) project_id: i64,
    pub(crate) base_prompt: &'a str,
    pub(crate) user_payload_json: &'a str,
    pub(crate) stage_key: &'a str,
    pub(crate) stage_name: &'a str,
    pub(crate) output_schema: &'a str,
    pub(crate) previous_context: Option<&'a Value>,
}

struct BlueprintStageRunInput<'a> {
    project_id: i64,
    spec: &'a BlueprintStageSpec,
    progress: BlueprintProgressReporter<'a>,
    cancel_token: Option<&'a CancellationToken>,
    llm: &'a dyn LlmProvider,
    prompt_engine: &'a dyn PromptEngine,
    base_prompt: &'a str,
    user_payload_json: &'a str,
    previous_context: Option<&'a Value>,
    workflow_run_id: Option<i64>,
}

struct BlueprintStageSpec {
    key: &'static str,
    title: &'static str,
    validate: fn(&Value) -> Result<(), AppError>,
    progress_current: u32,
    progress_message: &'static str,
    restored_message: &'static str,
    output_schema: &'static str,
    expected_top_level_keys: &'static [&'static str],
    blocked_note: &'static str,
}

struct BlueprintStageLlmOutput {
    parsed_output: Value,
    response_content: String,
    usage: Option<Value>,
    finish_reason: Option<String>,
}

type BlueprintProgressCallback<'a> = dyn Fn(u32, u32, &str) + Send + Sync + 'a;
type BlueprintProgressReporter<'a> = Option<&'a BlueprintProgressCallback<'a>>;

const BLUEPRINT_GENERATION_STAGES: [BlueprintStageSpec; 4] = [
    BlueprintStageSpec {
        key: "story_core",
        title: "阶段 1/4：故事核心",
        validate: validate_story_core_stage,
        progress_current: 2,
        progress_message: "生成故事核心与完整梗概",
        restored_message: "已恢复故事核心断点",
        output_schema: r#"{"title":"string","target_audience":"string","genre":"string","style":"string","tone":"string","one_sentence_summary":"string","full_synopsis":"string"}"#,
        expected_top_level_keys: STORY_CORE_STAGE_KEYS,
        blocked_note: "",
    },
    BlueprintStageSpec {
        key: "world",
        title: "阶段 2/4：世界观",
        validate: validate_world_stage,
        progress_current: 3,
        progress_message: "生成结构化世界观",
        restored_message: "已恢复世界观断点",
        output_schema: r#"{"world_setting":{"premise":"string","core_rules":"string","technology_or_magic":{"source":"string","rules":["string"],"limits":["string"],"costs":["string"]},"key_locations":[{"name":"string","description":"string","story_function":"string"}],"factions":[{"name":"string","description":"string","agenda":"string","representative_characters":["string"]}],"history_timeline":[{"period":"string","event":"string","impact":"string"}],"culture_and_society":{"values":"string","class_structure":"string","daily_life":"string"},"conflict_sources":[{"name":"string","description":"string","stakes":"string"}],"mysteries_and_foreshadowing":[{"mystery":"string","early_clues":["string"],"payoff_direction":"string"}],"genre_promises":["string"]}}"#,
        expected_top_level_keys: WORLD_STAGE_KEYS,
        blocked_note: "需要先获得故事核心阶段输出。",
    },
    BlueprintStageSpec {
        key: "characters",
        title: "阶段 3/4：角色",
        validate: validate_characters_stage,
        progress_current: 4,
        progress_message: "生成主要角色档案",
        restored_message: "已恢复角色档案断点",
        output_schema: r#"{"characters":[{"name":"string","character_type":"protagonist|antagonist|supporting|minor","gender":"string","age":"string","appearance":"string","personality":"string","background":"string","motivation":"string","arc_summary":"string","relationship_to_protagonist":"string","arc_stages":{"start":"string","core_wound":"string","desire":"string","need":"string","secret":"string","voice_style":"string","turning_point":"string","end":"string"}}]}"#,
        expected_top_level_keys: CHARACTERS_STAGE_KEYS,
        blocked_note: "需要先获得故事核心和世界观阶段输出。",
    },
    BlueprintStageSpec {
        key: "relationships",
        title: "阶段 4/4：关系",
        validate: validate_relationships_stage,
        progress_current: 5,
        progress_message: "生成人物关系网络",
        restored_message: "已恢复人物关系断点",
        output_schema: r#"{"relationships":[{"relationship_type":"family|friend|enemy|lover|rival|mentor|other","character_a":"string","character_b":"string","description":"string","strength":1,"evolution":{"start":"string","tension":"string","turning_points":["string"],"hidden_truth":"string","story_function":"string","end":"string"}}]}"#,
        expected_top_level_keys: RELATIONSHIPS_STAGE_KEYS,
        blocked_note: "需要先获得故事核心、世界观和角色阶段输出。",
    },
];

fn ensure_blueprint_generation_not_cancelled(
    cancel_token: Option<&CancellationToken>,
) -> Result<(), AppError> {
    if let Some(token) = cancel_token
        && token.is_cancelled()
    {
        return Err(AppError::Cancelled);
    }
    Ok(())
}

async fn chat_with_blueprint_cancel(
    llm: &dyn LlmProvider,
    request: ChatRequest,
    cancel_token: Option<&CancellationToken>,
) -> Result<ChatResponse, AppError> {
    ensure_blueprint_generation_not_cancelled(cancel_token)?;
    match cancel_token {
        Some(token) => {
            tokio::select! {
                _ = token.cancelled() => Err(AppError::Cancelled),
                result = llm.chat(request) => result,
            }
        }
        None => llm.chat(request).await,
    }
}

pub(super) fn build_blueprint_stage_request(
    input: BlueprintStageRequestInput<'_>,
) -> Result<ChatRequest, AppError> {
    let BlueprintStageRequestInput {
        prompt_engine,
        project_id,
        base_prompt,
        user_payload_json,
        stage_key,
        stage_name,
        output_schema,
        previous_context,
    } = input;
    let user_content = build_blueprint_stage_manifest_user_message(
        prompt_engine,
        Some(project_id),
        stage_key,
        stage_name,
        output_schema,
    )?;
    let system_content = prompt_engine.render(
        BLUEPRINT_STAGE_SYSTEM_PROMPT,
        &HashMap::from([
            ("base_prompt".to_string(), json!(base_prompt)),
            ("stage_name".to_string(), json!(stage_name)),
        ]),
        Some(project_id),
    )?;
    let manifest = build_blueprint_stage_context_manifest(
        user_payload_json,
        previous_context,
        user_content,
        stage_key,
        stage_name,
        output_schema,
    )?;
    Ok(context_aware_system_user_request(
        system_content,
        manifest,
        Some(0.7),
        None,
        Some(llm_token_profiles::BLUEPRINT.into()),
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: None,
        workflow: "blueprint_generation".to_string(),
        step_key: stage_key.to_string(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some(stage_key.to_string()),
        input_fingerprint_sha256: None,
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}

fn build_blueprint_stage_context_manifest(
    user_payload_json: &str,
    previous_context: Option<&Value>,
    dynamic_user_content: String,
    stage_key: &str,
    stage_name: &str,
    output_schema: &str,
) -> Result<LlmContextManifest, AppError> {
    let source_input: Value = serde_json::from_str(user_payload_json)?;
    let output_schema_json: Value = serde_json::from_str(output_schema).map_err(|error| {
        AppError::InvalidArgument(format!("{stage_name} 输出 schema 不是合法 JSON: {error}"))
    })?;
    let mut ledger = LlmContextLedger::new();
    let source_hash = blueprint_generation_source_hash(&source_input)?;
    let source_content = json!({
        "kind": "project_seed_context",
        "source_input": source_input,
    });
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: ("project").into(),
        pack_kind: ("blueprint_project_seed_context").into(),
        source_refs_json: Some(json!({
            "source": "blueprint_generation_payload",
            "source_input_sha256": source_hash,
        })),
        config_identity_json: json!({
            "ledger_version": BLUEPRINT_CONTEXT_LEDGER_VERSION,
            "kind": "project_seed_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, source_content.clone()),
            LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, source_content.clone()),
            LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, source_content),
        ],
    })?;
    if let Some(previous_context) = previous_context {
        let previous_hash = blueprint_generation_source_hash(previous_context)?;
        let previous_content = json!({
            "kind": "previous_stage_outputs",
            "previous_context": previous_context,
        });
        ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
            scope_kind: ("project").into(),
            pack_kind: ("blueprint_previous_stage_outputs").into(),
            source_refs_json: Some(json!({
                "source": "blueprint_generation_previous_stage_outputs",
                "previous_context_sha256": previous_hash,
                "stage_key": stage_key,
            })),
            config_identity_json: json!({
                "ledger_version": BLUEPRINT_CONTEXT_LEDGER_VERSION,
                "kind": "previous_stage_outputs",
                "stage_key": stage_key,
            }),
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, previous_content.clone()),
                LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, previous_content.clone()),
                LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, previous_content),
            ],
        })?;
    }
    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(format!(
            "当前阶段 {stage_name} 必须只返回严格 JSON 对象；顶层字段必须匹配 output_schema：{}",
            serde_json::to_string(&output_schema_json)?
        )),
    ))
}

fn blueprint_stage_previous_context(
    stage_index: usize,
    outputs: &[Value],
) -> Result<Option<Value>, AppError> {
    let required = |index: usize, message: &str| {
        outputs
            .get(index)
            .ok_or_else(|| AppError::InvalidArgument(message.to_string()))
    };
    Ok(match stage_index {
        0 => None,
        1 => Some(json!({
            "story_core": required(0, "世界观阶段缺少故事核心上下文")?,
        })),
        2 => Some(json!({
            "story_core": required(0, "角色阶段缺少故事核心上下文")?,
            "world": required(1, "角色阶段缺少世界观上下文")?,
        })),
        3 => Some(json!({
            "story_core": required(0, "关系阶段缺少故事核心上下文")?,
            "world": required(1, "关系阶段缺少世界观上下文")?,
            "characters": required(2, "关系阶段缺少角色上下文")?,
        })),
        _ => {
            return Err(AppError::InvalidArgument(format!(
                "未知蓝图生成阶段索引: {stage_index}"
            )));
        }
    })
}

fn blueprint_stage_call_key(spec: &BlueprintStageSpec) -> String {
    format!("blueprint_generation.{}", spec.key)
}

fn blueprint_stage_call_fingerprint_json(
    spec: &BlueprintStageSpec,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": 1,
        "workflow": "blueprint_generation",
        "call_key": blueprint_stage_call_key(spec),
        "stage_key": spec.key,
        "prompt_name": BLUEPRINT_STAGE_PREVIEW_PROMPT_NAME,
        "messages": request.messages,
        "context_manifest": request.context_manifest_json,
        "max_tokens_profile": request.max_tokens_profile,
        "response_format": request.response_format,
        "output_schema": spec.output_schema,
        "expected_top_level_keys": spec.expected_top_level_keys,
    }))
}

fn restored_blueprint_stage_output(
    checkpoint_json: Value,
    stage_name: &str,
    expected_top_level_keys: &[&str],
) -> Result<Value, AppError> {
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_response_content".to_string(),
            operation: format!("读取蓝图生成阶段 {stage_name} 调用级断点"),
        })?;
    let parsed_output =
        checkpoint_json
            .get("parsed_output")
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_checkpoint_parsed_output".to_string(),
                operation: format!("读取蓝图生成阶段 {stage_name} 调用级断点"),
            })?;
    let reparsed = parse_stage_json_object(response_content, stage_name, expected_top_level_keys)?;
    if &reparsed != parsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("校验蓝图生成阶段 {stage_name} 调用级断点"),
        });
    }
    Ok(reparsed)
}

fn ready_blueprint_preview_step(
    key: &str,
    title: &str,
    request: ChatRequest,
) -> BlueprintGenerationPreviewStep {
    BlueprintGenerationPreviewStep {
        key: key.to_string(),
        title: title.to_string(),
        prompt_name: BLUEPRINT_STAGE_PREVIEW_PROMPT_NAME.to_string(),
        state: "ready".to_string(),
        note: None,
        request: Some(request),
    }
}

fn blocked_blueprint_preview_step(
    key: &str,
    title: &str,
    note: &str,
) -> BlueprintGenerationPreviewStep {
    BlueprintGenerationPreviewStep {
        key: key.to_string(),
        title: title.to_string(),
        prompt_name: BLUEPRINT_STAGE_PREVIEW_PROMPT_NAME.to_string(),
        state: "blocked".to_string(),
        note: Some(note.to_string()),
        request: None,
    }
}

impl BlueprintService {
    pub async fn preview_generate_requests(
        &self,
        project_id: i64,
        novel_title: &str,
        genre: &str,
        target_length: i32,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, Value>,
    ) -> Result<Vec<BlueprintGenerationPreviewStep>, AppError> {
        let prompt_content = prompt_engine.render("blueprint", variables, Some(project_id))?;
        let user_payload = build_generation_payload(novel_title, genre, target_length, variables)?;
        let user_payload_json = serde_json::to_string_pretty(&user_payload)?;

        let mut steps = Vec::new();
        let mut next_missing_seen = false;

        for spec in BLUEPRINT_GENERATION_STAGES.iter() {
            if next_missing_seen {
                steps.push(blocked_blueprint_preview_step(
                    spec.key,
                    spec.title,
                    spec.blocked_note,
                ));
                continue;
            }

            let request = build_blueprint_stage_request(BlueprintStageRequestInput {
                prompt_engine,
                project_id,
                base_prompt: &prompt_content,
                user_payload_json: &user_payload_json,
                stage_key: spec.key,
                stage_name: spec.title,
                output_schema: spec.output_schema,
                previous_context: None,
            })?;
            steps.push(ready_blueprint_preview_step(spec.key, spec.title, request));
            next_missing_seen = true;
        }

        Ok(steps)
    }

    pub async fn generate(
        &self,
        input: BlueprintGenerateInput<'_>,
    ) -> Result<novel_blueprint::Model, AppError> {
        let BlueprintGenerateInput {
            project_id,
            novel_title,
            genre,
            target_length,
            llm,
            prompt_engine,
            variables,
            cancel_token,
            progress,
            workflow_run_id,
        } = input;
        if let Some(report) = progress {
            report(
                1,
                BLUEPRINT_GENERATE_PROGRESS_TOTAL,
                "整理灵感对话与蓝图提示词",
            );
        }
        ensure_blueprint_generation_not_cancelled(cancel_token)?;
        let prompt_content = prompt_engine.render("blueprint", variables, Some(project_id))?;
        let user_payload = build_generation_payload(novel_title, genre, target_length, variables)?;
        let user_payload_json = serde_json::to_string_pretty(&user_payload)?;

        let mut stage_outputs = Vec::new();
        for (stage_index, spec) in BLUEPRINT_GENERATION_STAGES.iter().enumerate() {
            if let Some(report) = progress {
                report(
                    spec.progress_current,
                    BLUEPRINT_GENERATE_PROGRESS_TOTAL,
                    spec.progress_message,
                );
            }
            let previous_context = blueprint_stage_previous_context(stage_index, &stage_outputs)?;
            let value = self
                .generate_or_resume_blueprint_stage(BlueprintStageRunInput {
                    project_id,
                    spec,
                    progress,
                    cancel_token,
                    llm,
                    prompt_engine,
                    base_prompt: &prompt_content,
                    user_payload_json: &user_payload_json,
                    previous_context: previous_context.as_ref(),
                    workflow_run_id,
                })
                .await?;
            stage_outputs.push(value);
        }
        let [
            core_output,
            world_output,
            characters_output,
            relationships_output,
        ]: [Value; 4] = stage_outputs
            .try_into()
            .map_err(|_| AppError::InvalidArgument("蓝图生成阶段输出数量不完整".to_string()))?;
        let trace_requests = blueprint_stage_trace_requests(
            prompt_engine,
            project_id,
            &prompt_content,
            &user_payload_json,
            &[
                core_output.clone(),
                world_output.clone(),
                characters_output.clone(),
                relationships_output.clone(),
            ],
        )?;

        if let Some(report) = progress {
            report(
                6,
                BLUEPRINT_GENERATE_PROGRESS_TOTAL,
                "组合并校验完整蓝图结构",
            );
        }
        ensure_blueprint_generation_not_cancelled(cancel_token)?;
        let assembled = assemble_staged_blueprint_json(
            novel_title,
            genre,
            target_length,
            &core_output,
            &world_output,
            &characters_output,
            &relationships_output,
        );
        let assembled_content = serde_json::to_string(&assembled)?;
        let parsed = parse_blueprint_json(&assembled_content, target_length)?;

        if let Some(report) = progress {
            report(
                7,
                BLUEPRINT_GENERATE_PROGRESS_TOTAL,
                "校验章节规划与工作流边界",
            );
        }

        if let Some(report) = progress {
            report(
                8,
                BLUEPRINT_GENERATE_PROGRESS_TOTAL,
                "保留已有大纲与章节产物",
            );
        }
        ensure_blueprint_generation_not_cancelled(cancel_token)?;

        if let Some(report) = progress {
            report(9, BLUEPRINT_GENERATE_PROGRESS_TOTAL, "保存蓝图主体");
        }
        let now = Utc::now().naive_utc();
        let existing = self.get(project_id).await?;
        let next_version = self
            .next_version(
                project_id,
                existing.as_ref().map(|bp| bp.current_version.as_str()),
            )
            .await?;

        let chapters_per_part = if parsed.needs_part_outlines {
            Some(parsed.chapters_per_part)
        } else {
            None
        };

        let blueprint = if let Some(bp) = existing {
            let mut a: novel_blueprint::ActiveModel = bp.into();
            a.title = Set(parsed.title.clone());
            a.genre = Set(parsed.genre.clone());
            a.style = Set(Some(parsed.style.clone()));
            a.tone = Set(Some(parsed.tone.clone()));
            a.target_audience = Set(Some(parsed.target_audience.clone()));
            a.one_sentence_summary = Set(Some(parsed.one_sentence_summary.clone()));
            a.full_synopsis = Set(Some(parsed.full_synopsis.clone()));
            a.world_setting = Set(Some(parsed.world_setting.clone()));
            a.needs_part_outlines = Set(parsed.needs_part_outlines);
            a.total_chapters = Set(Some(parsed.total_chapters));
            a.chapters_per_part = Set(chapters_per_part);
            a.current_version = Set(next_version.clone());
            a.updated_at = Set(now);
            a.update(self.db.as_ref())
                .await
                .map_err(AppError::Database)?
        } else {
            novel_blueprint::ActiveModel {
                project_id: Set(project_id),
                title: Set(parsed.title.clone()),
                genre: Set(parsed.genre.clone()),
                style: Set(Some(parsed.style.clone())),
                tone: Set(Some(parsed.tone.clone())),
                target_audience: Set(Some(parsed.target_audience.clone())),
                one_sentence_summary: Set(Some(parsed.one_sentence_summary.clone())),
                full_synopsis: Set(Some(parsed.full_synopsis.clone())),
                world_setting: Set(Some(parsed.world_setting.clone())),
                current_version: Set(next_version.clone()),
                needs_part_outlines: Set(parsed.needs_part_outlines),
                total_chapters: Set(Some(parsed.total_chapters)),
                chapters_per_part: Set(chapters_per_part),
                avatar_svg: Set(None),
                avatar_animal: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
        };

        if let Some(report) = progress {
            report(10, BLUEPRINT_GENERATE_PROGRESS_TOTAL, "保存蓝图版本快照");
        }
        self.save_version(
            project_id,
            next_version.clone(),
            Some(if next_version == "1.0" {
                "自动生成首版蓝图".to_string()
            } else {
                "重新生成蓝图".to_string()
            }),
            &parsed.raw,
        )
        .await?;

        if let Some(report) = progress {
            report(11, BLUEPRINT_GENERATE_PROGRESS_TOTAL, "同步主要角色");
        }
        self.replace_characters(project_id, &parsed.characters)
            .await?;

        if let Some(report) = progress {
            report(
                12,
                BLUEPRINT_GENERATE_PROGRESS_TOTAL,
                "同步人物关系、角色档案与项目状态",
            );
        }
        self.replace_relationships(project_id, &parsed.relationships)
            .await?;
        super::sync_character_profiles_from_blueprint(self.db.as_ref(), project_id, now).await?;
        self.sync_project_after_generation(project_id, &blueprint, &parsed.raw)
            .await?;
        record_completed_generation_run(
            self.db.as_ref(),
            RecordCompletedGenerationRunInput {
                project_id,
                workflow: "blueprint_generation".to_string(),
                step_key: "blueprint".to_string(),
                title: "生成故事蓝图".to_string(),
                model_kind: "llm".to_string(),
                provider: None,
                model: Some(llm.model_name().to_string()),
                prompt_template_names_json: Some(json!([
                    "blueprint",
                    BLUEPRINT_STAGE_SYSTEM_PROMPT,
                    "blueprint_stage_user"
                ])),
                prompt_template_hashes_json: None,
                rendered_messages_json: Some(Value::Array(
                    trace_requests
                        .iter()
                        .map(|(stage, request)| {
                            Ok(json!({
                                "stage": stage,
                                "messages": chat_request_messages_json(request)?,
                            }))
                        })
                        .collect::<Result<Vec<_>, AppError>>()?,
                )),
                rendered_prompts_json: None,
                input_variables_json: Some(json!({
                    "novel_title": novel_title,
                    "genre": genre,
                    "target_length": target_length,
                    "generation_payload": user_payload,
                })),
                source_refs_json: Some(json!({
                    "source_node": "inspiration",
                    "source_kind": "inspiration_conversation",
                })),
                media_refs_json: None,
                request_json: Some(Value::Array(
                    trace_requests
                        .iter()
                        .map(|(stage, request)| {
                            Ok(json!({
                                "stage": stage,
                                "request": context_aware_chat_request_trace_json(request)?,
                            }))
                        })
                        .collect::<Result<Vec<_>, AppError>>()?,
                )),
                response_summary_json: Some(text_response_summary_json(
                    &assembled_content,
                    None,
                    json!({
                        "title": parsed.title,
                        "current_version": blueprint.current_version,
                        "stage_count": trace_requests.len(),
                    }),
                )),
                targets: vec![GenerationRunTraceTarget {
                    target_kind: "novel_blueprint".to_string(),
                    target_table: "novel_blueprint".to_string(),
                    target_id: blueprint.id,
                }],
            },
        )
        .await?;

        Ok(blueprint)
    }

    async fn generate_or_resume_blueprint_stage(
        &self,
        input: BlueprintStageRunInput<'_>,
    ) -> Result<Value, AppError> {
        let BlueprintStageRunInput {
            project_id,
            spec,
            progress,
            cancel_token,
            llm,
            prompt_engine,
            base_prompt,
            user_payload_json,
            previous_context,
            workflow_run_id,
        } = input;
        ensure_blueprint_generation_not_cancelled(cancel_token)?;
        let mut request = build_blueprint_stage_request(BlueprintStageRequestInput {
            prompt_engine,
            project_id,
            base_prompt,
            user_payload_json,
            stage_key: spec.key,
            stage_name: spec.title,
            output_schema: spec.output_schema,
            previous_context,
        })?;
        let call_key = blueprint_stage_call_key(spec);
        let workflow_run_id = required_llm_call_workflow_run_id(workflow_run_id, &call_key)?;
        let fingerprint_json = blueprint_stage_call_fingerprint_json(spec, &request)?;
        let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)?;
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }
        WorkflowRunService::new(self.db.clone())
            .mark_current_step(workflow_run_id, &call_key)
            .await?;
        if let Some(checkpoint) = load_completed_llm_call_checkpoint(
            self.db.clone(),
            workflow_run_id,
            &call_key,
            &fingerprint_json,
        )
        .await?
        {
            let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)?;
            let value = restored_blueprint_stage_output(
                checkpoint_json,
                spec.title,
                spec.expected_top_level_keys,
            )?;
            (spec.validate)(&value)?;
            if let Some(report) = progress {
                report(
                    spec.progress_current,
                    BLUEPRINT_GENERATE_PROGRESS_TOTAL,
                    spec.restored_message,
                );
            }
            return Ok(value);
        }

        let output = self
            .generate_blueprint_stage(llm, request, spec, cancel_token)
            .await?;
        ensure_blueprint_generation_not_cancelled(cancel_token)?;
        (spec.validate)(&output.parsed_output)?;
        save_completed_llm_call_checkpoint(
            self.db.clone(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: spec.progress_current as i32,
                input_fingerprint_json: fingerprint_json,
                checkpoint_json: json!({
                    "output": output.parsed_output.clone(),
                    "parsed_output": output.parsed_output.clone(),
                    "response_content": output.response_content,
                    "usage": output.usage,
                    "finish_reason": output.finish_reason,
                }),
                artifacts_json: None,
            },
        )
        .await?;
        Ok(output.parsed_output)
    }

    async fn generate_blueprint_stage(
        &self,
        llm: &dyn LlmProvider,
        request: ChatRequest,
        spec: &BlueprintStageSpec,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<BlueprintStageLlmOutput, AppError> {
        let response = chat_with_blueprint_cancel(llm, request, cancel_token).await?;
        ensure_blueprint_generation_not_cancelled(cancel_token)?;
        ensure_blueprint_response_not_truncated(response.finish_reason.as_deref(), spec.title)?;

        let parsed_output = parse_stage_json_object(
            &response.message.content,
            spec.title,
            spec.expected_top_level_keys,
        )?;
        let usage = response
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;
        Ok(BlueprintStageLlmOutput {
            parsed_output,
            response_content: response.message.content,
            usage,
            finish_reason: response.finish_reason,
        })
    }
}

fn blueprint_stage_trace_requests(
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
    base_prompt: &str,
    user_payload_json: &str,
    outputs: &[Value],
) -> Result<Vec<(&'static str, ChatRequest)>, AppError> {
    BLUEPRINT_GENERATION_STAGES
        .iter()
        .enumerate()
        .map(|(stage_index, spec)| {
            let previous_context = blueprint_stage_previous_context(stage_index, outputs)?;
            let request = build_blueprint_stage_request(BlueprintStageRequestInput {
                prompt_engine,
                project_id,
                base_prompt,
                user_payload_json,
                stage_key: spec.key,
                stage_name: spec.title,
                output_schema: spec.output_schema,
                previous_context: previous_context.as_ref(),
            })?;
            Ok((spec.key, request))
        })
        .collect()
}
