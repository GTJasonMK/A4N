use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use a4n_api_contract::command_manifest as catalog;

use crate::client::is_sensitive_field_name;
use crate::config::write_private_json_unlocked;
use crate::context::SessionContext;

const FLOW_SCHEMA_VERSION: u32 = 1;
const JOURNAL_SCHEMA_VERSION: u32 = 1;
const INLINE_RESULT_LIMIT: usize = 256 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FlowSpec {
    pub(crate) version: u32,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) steps: Vec<FlowStep>,
}

#[derive(Deserialize)]
pub(crate) struct FlowStep {
    pub(crate) id: String,
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) capture: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) resume: ResumePolicy,
    #[serde(flatten)]
    pub(crate) action: FlowAction,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub(crate) enum FlowAction {
    Call {
        command: String,
        #[serde(default = "empty_object")]
        args: Value,
        #[serde(default)]
        set_base64: Vec<FileBinding>,
        #[serde(default)]
        set_text: Vec<FileBinding>,
        #[serde(default)]
        save_base64: Vec<FileBinding>,
        #[serde(default)]
        save_text: Vec<FileBinding>,
        #[serde(default)]
        force: bool,
        #[serde(default)]
        allow_destructive: bool,
    },
    TaskWatch {
        task_id: Value,
        #[serde(default = "default_attempt")]
        attempt: u32,
        after_sequence: Option<u64>,
    },
    WorkflowWatch {
        workflow_run_id: Value,
        #[serde(default = "default_interval_ms")]
        interval_ms: u64,
        timeout_seconds: Option<u64>,
    },
    MediaDownload {
        media_path: String,
        output: String,
        #[serde(default)]
        force: bool,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileBinding {
    pub(crate) pointer: String,
    pub(crate) path: String,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResumePolicy {
    #[default]
    Auto,
    Rerun,
    Manual,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FlowJournal {
    pub(crate) schema_version: u32,
    pub(crate) run_id: String,
    pub(crate) flow_name: String,
    pub(crate) flow_path: PathBuf,
    pub(crate) flow_sha256: String,
    pub(crate) status: String,
    pub(crate) created_at_epoch_ms: u128,
    pub(crate) updated_at_epoch_ms: u128,
    pub(crate) steps: Vec<JournalStep>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JournalStep {
    pub(crate) id: String,
    pub(crate) status: String,
    pub(crate) started_at_epoch_ms: Option<u128>,
    pub(crate) completed_at_epoch_ms: Option<u128>,
    pub(crate) result: Value,
    pub(crate) error: Option<String>,
}

pub(crate) struct LoadedFlow {
    pub(crate) spec: FlowSpec,
    pub(crate) path: PathBuf,
    pub(crate) sha256: String,
}

pub(crate) fn load_flow(path: &Path) -> Result<LoadedFlow> {
    let bytes = fs::read(path).with_context(|| format!("读取 flow 失败: {}", path.display()))?;
    let spec: FlowSpec = serde_json::from_slice(&bytes)
        .with_context(|| format!("flow JSON 无效: {}", path.display()))?;
    validate(&spec)?;
    Ok(LoadedFlow {
        spec,
        path: fs::canonicalize(path)
            .with_context(|| format!("解析 flow 路径失败: {}", path.display()))?,
        sha256: sha256(&bytes),
    })
}

pub(crate) fn validate(spec: &FlowSpec) -> Result<()> {
    if spec.version != FLOW_SCHEMA_VERSION {
        bail!(
            "flow version 不兼容: expected={} actual={}",
            FLOW_SCHEMA_VERSION,
            spec.version
        );
    }
    if spec.name.trim().is_empty() || spec.steps.is_empty() {
        bail!("flow name 不能为空且至少需要一个 step");
    }
    let mut ids = HashSet::new();
    for step in &spec.steps {
        if step.id.is_empty()
            || !step
                .id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            bail!(
                "flow step id 只能包含 ASCII 字母、数字、_ 和 -: {}",
                step.id
            );
        }
        if !ids.insert(step.id.as_str()) {
            bail!("flow step id 重复: {}", step.id);
        }
        match &step.action {
            FlowAction::Call {
                command,
                args,
                set_base64,
                set_text,
                save_base64,
                save_text,
                force,
                allow_destructive,
            } => {
                if catalog::API_COMMANDS
                    .binary_search(&command.as_str())
                    .is_err()
                {
                    bail!("flow step {} 使用未知命令: {command}", step.id);
                }
                if (catalog::is_destructive(command) || *force) && !allow_destructive {
                    bail!(
                        "flow step {} 是危险命令，spec 必须设置 allow_destructive=true",
                        step.id
                    );
                }
                validate_secret_args(command, args, &step.id)?;
                for binding in set_base64
                    .iter()
                    .chain(set_text)
                    .chain(save_base64)
                    .chain(save_text)
                {
                    if binding.path.trim().is_empty() {
                        bail!("flow step {} 文件绑定 path 不能为空", step.id);
                    }
                }
            }
            FlowAction::TaskWatch {
                attempt, task_id, ..
            } => {
                if *attempt == 0 || task_id.is_null() {
                    bail!("flow step {} 的 task_id/attempt 无效", step.id);
                }
            }
            FlowAction::WorkflowWatch {
                workflow_run_id,
                interval_ms,
                ..
            } => {
                if workflow_run_id.is_null() || !(200..=60_000).contains(interval_ms) {
                    bail!("flow step {} 的 workflow_run_id/interval_ms 无效", step.id);
                }
            }
            FlowAction::MediaDownload {
                media_path, output, ..
            } => {
                if media_path.trim().is_empty() || output.trim().is_empty() {
                    bail!("flow step {} 的媒体路径和输出路径不能为空", step.id);
                }
            }
        }
        for pointer in step.capture.values() {
            if !pointer.is_empty() && !pointer.starts_with('/') {
                bail!("flow step {} capture 必须是 JSON Pointer", step.id);
            }
        }
    }
    Ok(())
}

pub(crate) fn list_flows(directory: &Path) -> Result<Value> {
    if !directory.exists() {
        return Ok(json!({ "directory": directory, "flows": [] }));
    }
    let mut flows = Vec::new();
    for entry in fs::read_dir(directory)
        .with_context(|| format!("读取 flow 目录失败: {}", directory.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json")
            || path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.ends_with(".journal.json"))
        {
            continue;
        }
        match load_flow(&path) {
            Ok(flow) => flows.push(json!({
                "name": flow.spec.name,
                "description": flow.spec.description,
                "steps": flow.spec.steps.len(),
                "path": path,
                "valid": true,
            })),
            Err(error) => flows.push(json!({
                "path": path,
                "valid": false,
                "error": error.to_string(),
            })),
        }
    }
    flows.sort_by(|left, right| left["path"].to_string().cmp(&right["path"].to_string()));
    Ok(json!({ "directory": directory, "flows": flows }))
}

impl FlowJournal {
    pub(crate) fn new(flow: &LoadedFlow) -> Result<Self> {
        let now = now_epoch_ms()?;
        Ok(Self {
            schema_version: JOURNAL_SCHEMA_VERSION,
            run_id: Uuid::new_v4().to_string(),
            flow_name: flow.spec.name.clone(),
            flow_path: flow.path.clone(),
            flow_sha256: flow.sha256.clone(),
            status: "pending".into(),
            created_at_epoch_ms: now,
            updated_at_epoch_ms: now,
            steps: flow
                .spec
                .steps
                .iter()
                .map(|step| JournalStep {
                    id: step.id.clone(),
                    status: "pending".into(),
                    started_at_epoch_ms: None,
                    completed_at_epoch_ms: None,
                    result: Value::Null,
                    error: None,
                })
                .collect(),
        })
    }

    pub(crate) fn save_unlocked(&mut self, path: &Path) -> Result<()> {
        let previous = self.updated_at_epoch_ms;
        self.updated_at_epoch_ms = now_epoch_ms()?;
        if let Err(error) = write_private_json_unlocked(path, self) {
            self.updated_at_epoch_ms = previous;
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn results(&self) -> HashMap<String, Value> {
        self.steps
            .iter()
            .filter(|step| step.status == "completed")
            .map(|step| (step.id.clone(), step.result.clone()))
            .collect()
    }
}

pub(crate) fn load_journal_unlocked(path: &Path) -> Result<FlowJournal> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("读取 flow journal 失败: {}", path.display()))?;
    let journal: FlowJournal = serde_json::from_str(&contents)
        .with_context(|| format!("flow journal 无效: {}", path.display()))?;
    validate_journal(&journal)?;
    Ok(journal)
}

pub(crate) fn default_journal_path(flow_path: &Path, run_id: &str) -> PathBuf {
    let name = flow_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("flow");
    flow_path.with_file_name(format!("{name}.{run_id}.journal.json"))
}

pub(crate) fn verify_journal(flow: &LoadedFlow, journal: &FlowJournal) -> Result<()> {
    validate_journal(journal)?;
    if journal.flow_path != flow.path || journal.flow_sha256 != flow.sha256 {
        bail!("flow 文件在 journal 创建后已变化；拒绝自动 resume");
    }
    if journal.steps.len() != flow.spec.steps.len()
        || journal
            .steps
            .iter()
            .zip(&flow.spec.steps)
            .any(|(journal, spec)| journal.id != spec.id)
    {
        bail!("flow journal 与 step 清单不兼容");
    }
    Ok(())
}

fn validate_journal(journal: &FlowJournal) -> Result<()> {
    if journal.schema_version != JOURNAL_SCHEMA_VERSION {
        bail!("flow journal schema 不兼容");
    }
    if Uuid::parse_str(&journal.run_id).is_err()
        || journal.flow_name.trim().is_empty()
        || journal.flow_sha256.len() != 64
        || !journal
            .flow_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || journal.created_at_epoch_ms > journal.updated_at_epoch_ms
    {
        bail!("flow journal 元数据无效");
    }
    if !matches!(
        journal.status.as_str(),
        "pending" | "running" | "blocked" | "failed" | "completed"
    ) {
        bail!("flow journal 状态无效: {}", journal.status);
    }
    let mut ids = HashSet::new();
    for step in &journal.steps {
        if step.id.trim().is_empty() || !ids.insert(step.id.as_str()) {
            bail!("flow journal step id 无效或重复");
        }
        match step.status.as_str() {
            "pending"
                if step.started_at_epoch_ms.is_none()
                    && step.completed_at_epoch_ms.is_none()
                    && step.result.is_null()
                    && step.error.is_none() => {}
            "running"
                if step.started_at_epoch_ms.is_some()
                    && step.completed_at_epoch_ms.is_none()
                    && step.result.is_null()
                    && step.error.is_none() => {}
            "failed"
                if step.started_at_epoch_ms.is_some()
                    && step.completed_at_epoch_ms.is_none()
                    && step.result.is_null()
                    && step.error.as_deref().is_some_and(|error| !error.is_empty()) => {}
            "completed"
                if step.started_at_epoch_ms.is_some()
                    && step.completed_at_epoch_ms.is_some()
                    && step.error.is_none() => {}
            _ => bail!("flow journal step `{}` 状态内容不一致", step.id),
        }
        if step
            .started_at_epoch_ms
            .zip(step.completed_at_epoch_ms)
            .is_some_and(|(started, completed)| started > completed)
        {
            bail!("flow journal step `{}` 时间顺序无效", step.id);
        }
    }
    if journal.status == "completed" && journal.steps.iter().any(|step| step.status != "completed")
    {
        bail!("flow journal 不能在仍有未完成 step 时标记 completed");
    }
    Ok(())
}

pub(crate) fn render(
    value: &Value,
    context: &SessionContext,
    results: &HashMap<String, Value>,
    run_id: &str,
) -> Result<Value> {
    match value {
        Value::Array(items) => items
            .iter()
            .map(|item| render(item, context, results, run_id))
            .collect::<Result<Vec<_>>>()
            .map(Value::Array),
        Value::Object(object) => object
            .iter()
            .map(|(key, value)| Ok((key.clone(), render(value, context, results, run_id)?)))
            .collect::<Result<Map<_, _>>>()
            .map(Value::Object),
        Value::String(value) => render_string(value, context, results, run_id),
        _ => Ok(value.clone()),
    }
}

pub(crate) fn capture_result(result: &Value, capture: &BTreeMap<String, String>) -> Result<Value> {
    let captured = if capture.is_empty() {
        result.clone()
    } else {
        let mut object = Map::new();
        for (name, pointer) in capture {
            let value = if pointer.is_empty() {
                result
            } else {
                result.pointer(pointer).ok_or_else(|| {
                    anyhow::anyhow!("capture JSON Pointer 不存在: {name}={pointer}")
                })?
            };
            object.insert(name.clone(), value.clone());
        }
        Value::Object(object)
    };
    let redacted = redact_result(&captured);
    let bytes = serde_json::to_vec(&redacted)?;
    if bytes.len() <= INLINE_RESULT_LIMIT {
        return Ok(redacted);
    }
    Ok(json!({
        "_truncated": true,
        "byte_size": bytes.len(),
        "hint": "为该 step 配置 capture JSON Pointers 以保存后续步骤所需字段",
        "references": shallow_references(&redacted),
    }))
}

pub(crate) fn action_summary(step: &FlowStep) -> Value {
    let mut summary = match &step.action {
        FlowAction::Call {
            command,
            allow_destructive,
            ..
        } => json!({
            "id": step.id,
            "type": "call",
            "command": command,
            "effect": if catalog::is_destructive(command) { "destructive" } else { "normal" },
            "allow_destructive": allow_destructive,
            "resume": step.resume,
        }),
        FlowAction::TaskWatch { .. } => {
            json!({ "id": step.id, "type": "task-watch", "resume": step.resume })
        }
        FlowAction::WorkflowWatch { .. } => {
            json!({ "id": step.id, "type": "workflow-watch", "resume": step.resume })
        }
        FlowAction::MediaDownload { output, .. } => json!({
            "id": step.id,
            "type": "media-download",
            "output": output,
            "resume": step.resume,
        }),
    };
    if let Some(description) = &step.description {
        summary
            .as_object_mut()
            .expect("flow action summary must be an object")
            .insert("description".into(), Value::String(description.clone()));
    }
    summary
}

pub(crate) fn effective_resume_policy(step: &FlowStep) -> ResumePolicy {
    if step.resume != ResumePolicy::Auto {
        return step.resume;
    }
    match &step.action {
        FlowAction::Call { command, .. } => {
            let idempotency = catalog::command_idempotency(command)
                .expect("validated flow command must have idempotency metadata");
            if idempotency == "none" {
                ResumePolicy::Manual
            } else {
                ResumePolicy::Rerun
            }
        }
        FlowAction::MediaDownload { force, .. } if !force => ResumePolicy::Manual,
        _ => ResumePolicy::Rerun,
    }
}

fn render_string(
    input: &str,
    context: &SessionContext,
    results: &HashMap<String, Value>,
    run_id: &str,
) -> Result<Value> {
    if let Some(expression) = input
        .strip_prefix("${")
        .and_then(|value| value.strip_suffix('}'))
        && input.matches("${").count() == 1
    {
        return resolve(expression, context, results, run_id);
    }
    let mut output = String::new();
    let mut rest = input;
    while let Some(start) = rest.find("${") {
        output.push_str(&rest[..start]);
        let expression_start = start + 2;
        let Some(end) = rest[expression_start..].find('}') else {
            bail!("flow 模板占位符未闭合: {input}");
        };
        let expression = &rest[expression_start..expression_start + end];
        let value = resolve(expression, context, results, run_id)?;
        output.push_str(&scalar_text(&value)?);
        rest = &rest[expression_start + end + 1..];
    }
    output.push_str(rest);
    Ok(Value::String(output))
}

fn resolve(
    expression: &str,
    context: &SessionContext,
    results: &HashMap<String, Value>,
    run_id: &str,
) -> Result<Value> {
    let context_value = match expression {
        "context.user_id" => context.user_id,
        "context.project_id" => context.project_id,
        "context.chapter_id" => context.chapter_id,
        "context.snapshot_id" => context.snapshot_id,
        _ => None,
    };
    if expression.starts_with("context.") {
        return context_value
            .map(Value::from)
            .ok_or_else(|| anyhow::anyhow!("flow 所需上下文未设置: {expression}"));
    }
    if expression == "run.id" {
        return Ok(Value::String(run_id.to_string()));
    }
    if let Some(name) = expression.strip_prefix("env.") {
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            bail!("flow 环境变量名无效: {name}");
        }
        return std::env::var(name)
            .map(Value::String)
            .with_context(|| format!("flow 缺少环境变量: {name}"));
    }
    if let Some(reference) = expression.strip_prefix("steps.") {
        let (step_id, pointer) = reference
            .split_once('#')
            .ok_or_else(|| anyhow::anyhow!("step 引用必须使用 steps.<id>#<json-pointer>"))?;
        let result = results
            .get(step_id)
            .ok_or_else(|| anyhow::anyhow!("flow step 结果尚不可用: {step_id}"))?;
        return if pointer.is_empty() {
            Ok(result.clone())
        } else {
            result
                .pointer(pointer)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("flow step JSON Pointer 不存在: {expression}"))
        };
    }
    bail!("未知 flow 占位符: {expression}")
}

fn validate_secret_args(command: &str, args: &Value, step_id: &str) -> Result<()> {
    let descriptor = catalog::command_descriptor(command)
        .expect("validated flow command must have a descriptor");
    for pointer in &descriptor.secret_pointers {
        let Some(value) = args.pointer(pointer) else {
            continue;
        };
        if value.is_null() {
            continue;
        }
        let valid_env_reference = value
            .as_str()
            .and_then(|value| value.strip_prefix("${env."))
            .and_then(|value| value.strip_suffix('}'))
            .is_some_and(|name| {
                !name.is_empty()
                    && name
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            });
        if !valid_env_reference {
            bail!("flow step {step_id} 的敏感参数 {pointer} 不得内联；请使用 ${{env.NAME}} 占位符");
        }
    }
    Ok(())
}

pub(crate) fn redact_result(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(redact_result).collect()),
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    if is_sensitive_field_name(key) {
                        (key.clone(), Value::String("[REDACTED]".into()))
                    } else {
                        (key.clone(), redact_result(value))
                    }
                })
                .collect(),
        ),
        value => value.clone(),
    }
}

fn shallow_references(value: &Value) -> Value {
    let mut references = Map::new();
    if let Some(object) = value.as_object() {
        for (key, value) in object {
            if (key == "id" || key.ends_with("_id") || key.ends_with("_path"))
                && (value.is_string() || value.is_number())
            {
                references.insert(key.clone(), value.clone());
            }
        }
    }
    Value::Object(references)
}

fn scalar_text(value: &Value) -> Result<String> {
    match value {
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        Value::String(value) => Ok(value.clone()),
        _ => bail!("嵌入字符串的 flow 占位符必须解析为非 null scalar"),
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn now_epoch_ms() -> Result<u128> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("系统时间早于 UNIX epoch")?
        .as_millis())
}

fn empty_object() -> Value {
    json!({})
}

const fn default_attempt() -> u32 {
    1
}

const fn default_interval_ms() -> u64 {
    1_000
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::collections::HashMap;

    use serde_json::{Value, json};

    use super::{FlowJournal, FlowSpec, capture_result, render, validate, validate_journal};
    use crate::context::SessionContext;

    #[test]
    fn flow_validates_and_preserves_typed_references() {
        let spec: FlowSpec = serde_json::from_value(json!({
            "version": 1,
            "name": "smoke",
            "steps": [{
                "id": "list",
                "type": "call",
                "command": "list_projects",
                "args": {}
            }]
        }))
        .unwrap();
        validate(&spec).unwrap();

        let mut context = SessionContext::default();
        context.project_id = Some(7);
        let results = HashMap::from([("create".into(), json!({"id": 9}))]);
        let rendered = render(
            &json!({
                "projectId": "${context.project_id}",
                "createdId": "${steps.create#/id}",
                "label": "project-${steps.create#/id}",
            }),
            &context,
            &results,
            "run-1",
        )
        .unwrap();
        assert_eq!(rendered["projectId"], 7);
        assert_eq!(rendered["createdId"], 9);
        assert_eq!(rendered["label"], "project-9");

        let null_results = HashMap::from([("empty".into(), Value::Null)]);
        assert!(
            render(
                &json!("value-${steps.empty#}"),
                &context,
                &null_results,
                "run-1"
            )
            .is_err()
        );
    }

    #[test]
    fn flow_rejects_inline_secrets() {
        let value = json!({
            "version": 1,
            "name": "unsafe",
            "steps": [{
                "id": "config",
                "type": "call",
                "command": "create_llm_config",
                "args": {"input": {"api_key": "plain-text"}}
            }]
        });
        let spec: FlowSpec = serde_json::from_value(value.clone()).unwrap();
        assert!(validate(&spec).is_err());

        let mut numeric_secret = value.clone();
        numeric_secret["steps"][0]["args"]["input"]["api_key"] = json!(123);
        assert!(validate(&serde_json::from_value(numeric_secret).unwrap()).is_err());

        let mut unknown = value;
        unknown["legacy_mode"] = json!(true);
        assert!(serde_json::from_value::<FlowSpec>(unknown).is_err());

        let mut unknown_step = json!({
            "version": 1,
            "name": "unsafe",
            "steps": [{
                "id": "config",
                "type": "call",
                "command": "list_projects",
                "args": {},
                "legacy_mode": true
            }]
        });
        assert!(serde_json::from_value::<FlowSpec>(unknown_step.clone()).is_err());
        unknown_step["steps"][0]
            .as_object_mut()
            .unwrap()
            .remove("legacy_mode");
        unknown_step["steps"][0]["unexpected_action_field"] = json!(true);
        assert!(serde_json::from_value::<FlowSpec>(unknown_step).is_err());

        let non_secret_marker: FlowSpec = serde_json::from_value(json!({
            "version": 1,
            "name": "token-count",
            "steps": [{
                "id": "list",
                "type": "call",
                "command": "list_projects",
                "args": {"token_count": 10}
            }]
        }))
        .unwrap();
        validate(&non_secret_marker).unwrap();
    }

    #[test]
    fn flow_capture_limits_persisted_results() {
        let result = json!({
            "id": 42,
            "token_count": 128,
            "payload": "x".repeat(256 * 1024),
        });
        let truncated = capture_result(&result, &BTreeMap::new()).unwrap();
        assert_eq!(truncated["_truncated"], true);
        assert!(truncated["byte_size"].as_u64().unwrap() > 256 * 1024);
        assert_eq!(truncated["references"]["id"], 42);

        let captured = capture_result(
            &result,
            &BTreeMap::from([
                ("project_id".into(), "/id".into()),
                ("token_count".into(), "/token_count".into()),
            ]),
        )
        .unwrap();
        assert_eq!(captured, json!({"project_id": 42, "token_count": 128}));

        let redacted = capture_result(
            &json!({"access_token": "secret", "token_count": 128}),
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(redacted["access_token"], "[REDACTED]");
        assert_eq!(redacted["token_count"], 128);
    }

    #[test]
    fn journal_rejects_false_completed_state() {
        let journal = serde_json::from_value(json!({
            "schema_version": 1,
            "run_id": "4f9f1bd4-7b70-4a55-9f2c-5968d41e0808",
            "flow_name": "fake",
            "flow_path": "/tmp/fake.json",
            "flow_sha256": "a".repeat(64),
            "status": "completed",
            "created_at_epoch_ms": 1,
            "updated_at_epoch_ms": 2,
            "steps": [{
                "id": "pending",
                "status": "pending",
                "started_at_epoch_ms": null,
                "completed_at_epoch_ms": null,
                "result": null,
                "error": null
            }]
        }))
        .unwrap();
        assert!(validate_journal(&journal).is_err());
    }

    #[test]
    fn journal_preserves_completed_null_results() {
        let value = json!({
            "schema_version": 1,
            "run_id": "4f9f1bd4-7b70-4a55-9f2c-5968d41e0808",
            "flow_name": "null-result",
            "flow_path": "/tmp/null-result.json",
            "flow_sha256": "a".repeat(64),
            "status": "completed",
            "created_at_epoch_ms": 1,
            "updated_at_epoch_ms": 2,
            "steps": [{
                "id": "null",
                "status": "completed",
                "started_at_epoch_ms": 1,
                "completed_at_epoch_ms": 2,
                "result": null,
                "error": null
            }]
        });
        let journal: FlowJournal = serde_json::from_value(value).unwrap();
        validate_journal(&journal).unwrap();
        assert_eq!(
            journal.results().get("null"),
            Some(&serde_json::Value::Null)
        );

        let journal: FlowJournal =
            serde_json::from_value(serde_json::to_value(journal).unwrap()).unwrap();
        assert_eq!(
            journal.results().get("null"),
            Some(&serde_json::Value::Null)
        );
    }
}
