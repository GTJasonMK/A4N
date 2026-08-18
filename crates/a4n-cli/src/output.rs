use std::io::{self, Write};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::ValueEnum;
use serde_json::{Map, Value, json};

use crate::client::ApiRequestError;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub(crate) enum OutputMode {
    Human,
    #[default]
    Json,
    Jsonl,
    AgentJson,
}

#[derive(Clone, Debug)]
pub(crate) struct OutputOptions {
    pub(crate) mode: OutputMode,
    pub(crate) compact: bool,
    pub(crate) command: String,
}

impl OutputOptions {
    pub(crate) fn new(mode: OutputMode, compact: bool) -> Self {
        Self {
            mode,
            compact,
            command: String::new(),
        }
    }

    pub(crate) fn for_command(&self, command: impl Into<String>) -> Self {
        let mut next = self.clone();
        next.command = command.into();
        next
    }
}

pub(crate) fn print_value(value: &Value, options: &OutputOptions) -> Result<()> {
    let value = match options.mode {
        OutputMode::AgentJson => json!({
            "ok": true,
            "command": options.command,
            "data": value,
            "meta": { "references": extract_references(value, &options.command) },
        }),
        _ => value.clone(),
    };
    let mut stdout = io::stdout().lock();
    match options.mode {
        OutputMode::Human => write_human(&mut stdout, &value)?,
        OutputMode::Jsonl => {
            if let Value::Array(items) = &value {
                for item in items {
                    serde_json::to_writer(&mut stdout, item)?;
                    stdout.write_all(b"\n")?;
                }
                stdout.flush().context("刷新 stdout 失败")?;
                return Ok(());
            }
            serde_json::to_writer(&mut stdout, &value)?;
        }
        OutputMode::AgentJson => serde_json::to_writer(&mut stdout, &value)?,
        OutputMode::Json if options.compact => serde_json::to_writer(&mut stdout, &value)?,
        OutputMode::Json => serde_json::to_writer_pretty(&mut stdout, &value)?,
    }
    if !matches!(options.mode, OutputMode::Human) {
        stdout.write_all(b"\n").context("写入 stdout 失败")?;
    }
    stdout.flush().context("刷新 stdout 失败")
}

#[derive(Debug)]
pub(crate) struct CliError {
    pub(crate) code: String,
    pub(crate) kind: &'static str,
    pub(crate) message: String,
    pub(crate) retryable: bool,
    pub(crate) details: Value,
    pub(crate) exit_code: u8,
}

impl CliError {
    pub(crate) fn new(
        code: impl Into<String>,
        kind: &'static str,
        message: impl Into<String>,
        exit_code: u8,
    ) -> Self {
        Self {
            code: code.into(),
            kind,
            message: message.into(),
            retryable: false,
            details: json!({}),
            exit_code,
        }
    }

    pub(crate) fn with_details(mut self, details: Value) -> Self {
        self.details = details;
        self
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CliError {}

pub(crate) fn print_error(error: &anyhow::Error, options: &OutputOptions) -> ExitCode {
    let info = ErrorInfo::from_error(error);
    if matches!(options.mode, OutputMode::AgentJson) {
        let body = json!({
            "ok": false,
            "command": options.command,
            "error": {
                "code": info.code,
                "kind": info.kind,
                "message": info.message,
                "retryable": info.retryable,
                "http_status": info.http_status,
                "trace_id": info.trace_id,
                "details": info.details,
            }
        });
        let mut stderr = io::stderr().lock();
        let _ = serde_json::to_writer(&mut stderr, &body);
        let _ = stderr.write_all(b"\n");
        let _ = stderr.flush();
    } else {
        eprintln!("error: {error:#}");
    }
    ExitCode::from(info.exit_code)
}

struct ErrorInfo {
    code: String,
    kind: &'static str,
    message: String,
    retryable: bool,
    http_status: Option<u16>,
    trace_id: Option<String>,
    details: Value,
    exit_code: u8,
}

impl ErrorInfo {
    fn from_error(error: &anyhow::Error) -> Self {
        if let Some(cli) = error.downcast_ref::<CliError>() {
            return Self {
                code: cli.code.clone(),
                kind: cli.kind,
                message: cli.message.clone(),
                retryable: cli.retryable,
                http_status: None,
                trace_id: None,
                details: cli.details.clone(),
                exit_code: cli.exit_code,
            };
        }
        if let Some(api) = error.downcast_ref::<ApiRequestError>() {
            let status = api.status.as_u16();
            let (kind, exit_code) = match status {
                401 => ("authentication", 3),
                403 => ("permission", 4),
                404 => ("not_found", 5),
                409 | 410 => ("conflict", 6),
                408 | 425 | 429 | 500..=599 => ("unavailable", 7),
                _ => ("server", 8),
            };
            return Self {
                code: api.code.clone(),
                kind,
                message: api.message.clone(),
                retryable: api.retryable,
                http_status: Some(status),
                trace_id: api.trace_id.clone(),
                details: api.details.clone(),
                exit_code,
            };
        }
        if let Some(request) = error.downcast_ref::<reqwest::Error>() {
            let retryable = request.is_timeout() || request.is_connect();
            return Self {
                code: if request.is_timeout() {
                    "network_timeout"
                } else if request.is_connect() {
                    "network_connect"
                } else {
                    "network_error"
                }
                .to_string(),
                kind: "unavailable",
                message: if request.is_timeout() {
                    "HTTP 请求超时".into()
                } else if request.is_connect() {
                    "无法连接 A4N Server".into()
                } else {
                    "HTTP 请求失败".into()
                },
                retryable,
                http_status: request.status().map(|status| status.as_u16()),
                trace_id: None,
                details: json!({}),
                exit_code: 7,
            };
        }

        let message = format!("{error:#}");
        Self {
            code: "invalid_input".into(),
            kind: "input",
            message,
            retryable: false,
            http_status: None,
            trace_id: None,
            details: json!({}),
            exit_code: 2,
        }
    }
}

fn write_human(writer: &mut impl Write, value: &Value) -> Result<()> {
    match value {
        Value::Array(items) if !items.is_empty() && items.iter().all(Value::is_object) => {
            write_object_table(writer, items)
        }
        Value::Object(object) => {
            if let Some((collection_key, items)) = ["items", "commands", "flows", "checks"]
                .into_iter()
                .find_map(|key| {
                    object
                        .get(key)
                        .and_then(Value::as_array)
                        .filter(|items| !items.is_empty() && items.iter().all(Value::is_object))
                        .map(|items| (key, items))
                })
            {
                for (key, value) in object {
                    if key != collection_key && !value.is_array() && !value.is_object() {
                        writeln!(writer, "{key}: {}", display_cell(value))?;
                    }
                }
                writeln!(writer)?;
                return write_object_table(writer, items);
            }
            let width = object.keys().map(String::len).max().unwrap_or_default();
            for (key, value) in object {
                writeln!(
                    writer,
                    "{key:width$}  {}",
                    display_cell(value),
                    width = width
                )?;
            }
            Ok(())
        }
        Value::String(value) => writeln!(writer, "{value}").map_err(Into::into),
        _ => {
            serde_json::to_writer_pretty(&mut *writer, value)?;
            Ok(())
        }
    }
}

fn write_object_table(writer: &mut impl Write, items: &[Value]) -> Result<()> {
    let mut columns = Vec::<String>::new();
    for item in items {
        if let Some(object) = item.as_object() {
            for key in object.keys() {
                let scalar_column = items.iter().all(|item| {
                    item.get(key).is_none_or(|value| {
                        value.is_null()
                            || value.is_boolean()
                            || value.is_number()
                            || value.is_string()
                    })
                });
                if scalar_column && !columns.contains(key) {
                    columns.push(key.clone());
                }
            }
        }
    }
    let rows = items
        .iter()
        .filter_map(Value::as_object)
        .map(|object| {
            columns
                .iter()
                .map(|key| display_cell(object.get(key).unwrap_or(&Value::Null)))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let widths = columns
        .iter()
        .enumerate()
        .map(|(index, key)| {
            rows.iter()
                .map(|row| row[index].chars().count())
                .max()
                .unwrap_or_default()
                .max(key.chars().count())
                .min(48)
        })
        .collect::<Vec<_>>();
    write_table_row(writer, &columns, &widths)?;
    writeln!(
        writer,
        "{}",
        widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<_>>()
            .join("-+-")
    )?;
    for row in rows {
        write_table_row(writer, &row, &widths)?;
    }
    Ok(())
}

fn write_table_row(writer: &mut impl Write, row: &[String], widths: &[usize]) -> Result<()> {
    let cells = row
        .iter()
        .zip(widths)
        .map(|(value, width)| format!("{:<width$}", truncate(value, *width), width = width))
        .collect::<Vec<_>>();
    writeln!(writer, "{}", cells.join(" | "))?;
    Ok(())
}

fn display_cell(value: &Value) -> String {
    match value {
        Value::Null => "-".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.replace(['\n', '\r'], " "),
        _ => serde_json::to_string(value).expect("serde_json::Value must serialize"),
    }
}

fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }
    if width <= 1 {
        return "…".into();
    }
    value.chars().take(width - 1).chain(['…']).collect()
}

fn extract_references(value: &Value, command: &str) -> Value {
    let mut references = Map::new();
    collect_references(value, &mut references);
    if let Some(id) = value.get("id").and_then(Value::as_i64) {
        let key = if command.contains("workflow") {
            Some("workflow_run_ids")
        } else if command.contains("chapter") {
            Some("chapter_ids")
        } else if command.contains("project") {
            Some("project_ids")
        } else {
            None
        };
        if let Some(key) = key {
            push_reference(&mut references, key, Value::from(id));
        }
    }
    Value::Object(references)
}

fn collect_references(value: &Value, references: &mut Map<String, Value>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_references(item, references);
            }
        }
        Value::Object(object) => {
            for (key, value) in object {
                let target = match key.as_str() {
                    "project_id" | "projectId" | "root_project_id" => Some("project_ids"),
                    "chapter_id" | "chapterId" => Some("chapter_ids"),
                    "snapshot_id" | "snapshotId" | "source_snapshot_id" => Some("snapshot_ids"),
                    "task_id" | "taskId" | "active_task_id" => Some("task_ids"),
                    "workflow_run_id" | "workflowRunId" => Some("workflow_run_ids"),
                    "media_path" | "file_path" | "output" | "download_url" => Some("artifacts"),
                    _ => None,
                };
                if let Some(target) = target
                    && (value.is_string() || value.is_number())
                {
                    push_reference(references, target, value.clone());
                }
                collect_references(value, references);
            }
        }
        _ => {}
    }
}

fn push_reference(references: &mut Map<String, Value>, key: &str, value: Value) {
    let values = references
        .entry(key.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let Some(values) = values.as_array_mut() else {
        return;
    };
    if values.len() < 100 && !values.contains(&value) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::extract_references;

    #[test]
    fn agent_metadata_extracts_stable_references() {
        let refs = extract_references(
            &json!({"project_id": 1, "task_id": "task-1", "nested": {"chapterId": 2}}),
            "call create_project",
        );
        assert_eq!(refs["project_ids"], json!([1]));
        assert_eq!(refs["chapter_ids"], json!([2]));
        assert_eq!(refs["task_ids"], json!(["task-1"]));
    }
}
