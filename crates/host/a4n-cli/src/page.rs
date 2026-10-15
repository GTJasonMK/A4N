use a4n_api_contract::page::{PageCommandManifest, PageOperationStatus, PageOperationView};
use anyhow::{Context, Result, anyhow, bail};
use clap::Subcommand;
use futures::StreamExt;
use reqwest::Method;
use serde_json::{Value, json};
use std::io::Read;

use crate::client::ApiClient;
use crate::context::SessionContext;
use crate::output::{self, OutputOptions};
use crate::{CallFiles, CliRuntime, PageCallContext, invoke_call, read_json_source};

#[derive(Debug, Subcommand)]
pub(crate) enum PageCommand {
    /// 列出当前用户的 GUI Page Session
    Sessions,
    /// 使用 GUI 显示的一次性授权码绑定当前 Agent
    Attach {
        /// GUI 显示的一次性授权码
        #[arg(long, conflicts_with = "code_stdin")]
        code: Option<String>,
        /// 从 stdin 读取一次性授权码，避免进入 shell history
        #[arg(long, conflicts_with = "code")]
        code_stdin: bool,
    },
    /// 查看已绑定 Page Session 的状态
    Status,
    /// 查看当前页面上下文
    Context,
    /// 查看当前页面实时可用的 Host API 命令
    Commands,
    /// 在当前页面上下文中调用 Host API 命令
    Call {
        command: String,
        #[arg(long, default_value = "{}", value_name = "JSON|@FILE|-")]
        args: String,
        #[arg(long)]
        allow_destructive: bool,
    },
    /// 请求 GUI 导航到站内路由
    Navigate {
        route: String,
        #[arg(long)]
        page_kind: Option<String>,
        #[arg(long)]
        reason: Option<String>,
    },
    /// 暂停当前 Page Agent
    Pause,
    /// 恢复当前 Page Agent
    Resume,
    /// 取消当前操作，但保留 Agent 绑定
    Cancel,
    /// 用户接管当前页面并解除 Agent 绑定
    Takeover,
    /// 解除 Page Session 绑定
    Detach,
    /// 监听当前用户的 Page Session SSE 事件
    Events,
}

impl PageCommand {
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Sessions => "sessions".into(),
            Self::Attach { .. } => "attach".into(),
            Self::Status => "status".into(),
            Self::Context => "context".into(),
            Self::Commands => "commands".into(),
            Self::Call { command, .. } => format!("call {command}"),
            Self::Navigate { .. } => "navigate".into(),
            Self::Pause => "pause".into(),
            Self::Resume => "resume".into(),
            Self::Cancel => "cancel".into(),
            Self::Takeover => "takeover".into(),
            Self::Detach => "detach".into(),
            Self::Events => "events".into(),
        }
    }
}

pub(crate) async fn run(
    runtime: &mut CliRuntime,
    command: PageCommand,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        PageCommand::Sessions => {
            let value = runtime
                .client
                .send_page_json(Method::GET, "/ui-sessions", None, false, false)
                .await?;
            output::print_value(&value, output)
        }
        PageCommand::Attach { code, code_stdin } => {
            let code = read_authorization_code(code.as_deref(), code_stdin)?;
            attach(runtime, &code, output).await
        }
        PageCommand::Status => {
            let value = get_session(&mut runtime.client).await?;
            output::print_value(&value, output)
        }
        PageCommand::Context => {
            let value = get_session(&mut runtime.client).await?;
            output::print_value(
                &json!({
                    "ui_session_id": value["ui_session_id"],
                    "page_kind": value["page_kind"],
                    "route": value["route"],
                    "project_id": value["project_id"],
                    "chapter_id": value["chapter_id"],
                    "active_tab": value["active_tab"],
                    "selected_ids": value["selected_ids"],
                    "dirty_state": value["dirty_state"],
                    "local_state": value["local_state"],
                    "context_revision": value["context_revision"],
                }),
                output,
            )
        }
        PageCommand::Commands => {
            let value = get_commands(&mut runtime.client).await?;
            output::print_value(&value, output)
        }
        PageCommand::Call {
            command,
            args,
            allow_destructive,
        } => call(runtime, &command, &args, allow_destructive, output).await,
        PageCommand::Navigate {
            route,
            page_kind,
            reason,
        } => {
            let session_id = require_session_id(&runtime.client)?;
            let value = runtime
                .client
                .send_page_json(
                    Method::POST,
                    &path(session_id, "/navigate"),
                    Some(json!({
                        "route": route,
                        "page_kind": page_kind,
                        "reason": reason,
                    })),
                    true,
                    true,
                )
                .await?;
            output::print_value(&value, output)
        }
        PageCommand::Pause => control(runtime, "pause", output).await,
        PageCommand::Resume => control(runtime, "resume", output).await,
        PageCommand::Cancel => control(runtime, "cancel", output).await,
        PageCommand::Takeover => control(runtime, "takeover", output).await,
        PageCommand::Detach => control(runtime, "detach", output).await,
        PageCommand::Events => events(runtime, output).await,
    }
}

async fn attach(runtime: &mut CliRuntime, code: &str, output: &OutputOptions) -> Result<()> {
    let agent_id = runtime
        .client
        .agent_id()
        .ok_or_else(|| anyhow!("Page attach 需要已保存的 Agent；请先执行 `a4n agent register`"))?;
    let value = runtime
        .client
        .send_page_json(
            Method::POST,
            "/ui-sessions/attach",
            Some(json!({ "authorization_code": code, "agent_id": agent_id })),
            false,
            true,
        )
        .await
        .map_err(|error| crate::client::redact_error_values(error, &[code.to_string()]))?;
    let session_id = value
        .pointer("/session/ui_session_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Page attach 响应缺少 session.ui_session_id"))?;
    let token = value
        .get("ui_session_token")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Page attach 响应缺少 ui_session_token"))?;
    runtime
        .client
        .save_page_credentials(session_id.into(), token.into())?;
    output::print_value(
        &json!({
            "attached": true,
            "session": value["session"],
        }),
        output,
    )
}

fn read_authorization_code(code: Option<&str>, code_stdin: bool) -> Result<String> {
    let value = if code_stdin {
        let mut input = String::new();
        std::io::stdin()
            .read_to_string(&mut input)
            .context("读取 Page 授权码失败")?;
        input
    } else {
        code.unwrap_or_default().to_string()
    };
    let value = value.trim().to_string();
    if value.is_empty() || value.chars().any(char::is_whitespace) {
        bail!("Page 授权码无效；请使用 `--code CODE` 或 `--code-stdin`");
    }
    Ok(value)
}

async fn call(
    runtime: &mut CliRuntime,
    command: &str,
    args_source: &str,
    allow_destructive: bool,
    output: &OutputOptions,
) -> Result<()> {
    let mut args = read_json_source(args_source)?;
    let session_value = get_session(&mut runtime.client).await?;
    let manifest_value = get_commands(&mut runtime.client).await?;
    let manifest: PageCommandManifest =
        serde_json::from_value(manifest_value).context("Page command manifest 合同无效")?;
    let availability = manifest
        .commands
        .iter()
        .find(|item| item.descriptor.name == command)
        .ok_or_else(|| anyhow!("当前页面未暴露 Host API 命令: {command}"))?;
    if !availability.available {
        return Err(output::CliError::new(
            "page_command_unavailable",
            "conflict",
            availability
                .reason
                .clone()
                .unwrap_or_else(|| format!("当前页面暂不可用: {command}")),
            6,
        )
        .with_details(json!({ "command": command, "route": session_value["route"] }))
        .into());
    }
    if availability.descriptor.effect == "destructive" && !allow_destructive {
        return Err(output::CliError::new(
            "destructive_confirmation_required",
            "confirmation",
            format!("命令 `{command}` 属于危险操作，需要 --allow-destructive 显式授权"),
            10,
        )
        .into());
    }
    if !args.is_object() {
        bail!("page call --args 必须是 JSON object");
    }
    let session_id = require_session_id(&runtime.client)?.to_string();
    if runtime.client.agent_id().is_none() {
        bail!("Page call 需要已保存的 Agent；请先执行 `a4n agent register`");
    }
    let mut operation_target = args.clone();
    let operation_secrets = crate::request_secret_values(&availability.descriptor, &args);
    crate::client::redact_json_values(&mut operation_target, &operation_secrets);
    let operation_value = runtime
        .client
        .send_page_json(
            Method::POST,
            &path(&session_id, "/operations"),
            Some(json!({
                "command": command,
                "allow_destructive": allow_destructive,
                "phase": format!("Page CLI 调用 {command}"),
                "target": operation_target,
                "context_revision": session_value["context_revision"],
            })),
            true,
            true,
        )
        .await?;
    let operation: PageOperationView =
        serde_json::from_value(operation_value).context("Page operation 创建响应合同无效")?;

    let mut page_context = SessionContext::load(None, runtime.client.api_base())?;
    page_context.set(
        session_value["project_id"].as_i64(),
        session_value["chapter_id"].as_i64(),
        None,
    )?;
    let result = invoke_call(
        &mut runtime.client,
        &page_context,
        command,
        std::mem::take(&mut args),
        CallFiles {
            set_base64: &[],
            set_text: &[],
            save_base64: &[],
            save_text: &[],
            force: false,
        },
        Some(PageCallContext {
            session_id: session_id.clone(),
            operation_id: operation.operation_id.clone(),
        }),
    )
    .await;
    match result {
        Ok(result) => {
            let completed = runtime
                .client
                .send_page_json(
                    Method::PATCH,
                    &path(
                        &session_id,
                        &format!("/operations/{}", operation.operation_id),
                    ),
                    Some(json!({
                        "status": PageOperationStatus::Succeeded,
                        "progress": 100,
                        "summary": "Page CLI 调用已完成",
                        "result": result,
                    })),
                    true,
                    true,
                )
                .await;
            let completed = match completed {
                Ok(value) => value,
                Err(error) if page_operation_cancelled(&error) => return Err(error),
                Err(error) => return Err(error),
            };
            let result_value = completed["result"].clone();
            output::print_value(
                &json!({ "operation": completed, "result": result_value }),
                output,
            )
        }
        Err(error) if page_operation_cancelled(&error) => Err(error),
        Err(error) => {
            let status_update = runtime
                .client
                .send_page_json(
                    Method::PATCH,
                    &path(
                        &session_id,
                        &format!("/operations/{}", operation.operation_id),
                    ),
                    Some(json!({
                        "status": PageOperationStatus::Failed,
                        "summary": "Page CLI 调用失败",
                        "error": "Page CLI Host API 调用失败",
                    })),
                    true,
                    true,
                )
                .await;
            match status_update {
                Ok(_) => Err(error),
                Err(update_error) => Err(update_error.context(format!(
                    "Page Operation 失败状态回写失败；原始 Page CLI 调用错误: {error:#}"
                ))),
            }
        }
    }
}

fn page_operation_cancelled(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<crate::client::ApiRequestError>()
        .is_some_and(|error| error.code == "page_operation_cancelled")
}

async fn control(runtime: &mut CliRuntime, action: &str, output: &OutputOptions) -> Result<()> {
    let session_id = require_session_id(&runtime.client)?;
    let value = runtime
        .client
        .send_page_json(
            Method::POST,
            &path(session_id, &format!("/control/{action}")),
            None,
            true,
            false,
        )
        .await?;
    if matches!(action, "takeover" | "detach") {
        runtime.client.clear_page_credentials()?;
    }
    output::print_value(&value, output)
}

async fn get_session(client: &mut ApiClient) -> Result<Value> {
    let session_id = require_session_id(client)?;
    client
        .send_page_json(Method::GET, &path(session_id, ""), None, true, false)
        .await
}

async fn get_commands(client: &mut ApiClient) -> Result<Value> {
    let session_id = require_session_id(client)?;
    client
        .send_page_json(
            Method::GET,
            &path(session_id, "/commands"),
            None,
            true,
            false,
        )
        .await
}

fn require_session_id(client: &ApiClient) -> Result<&str> {
    client
        .ui_session_id()
        .ok_or_else(|| anyhow!("没有 Page Session；请先执行 `a4n page attach --code CODE`"))
}

fn path(session_id: &str, suffix: &str) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(session_id.as_bytes()).collect();
    format!("/ui-sessions/{encoded}{suffix}")
}

async fn events(runtime: &mut CliRuntime, output: &OutputOptions) -> Result<()> {
    let response = runtime.client.page_stream("/ui-sessions/events").await?;
    let mut stream = response.bytes_stream();
    let mut buffer = Vec::new();
    while let Some(chunk) = stream.next().await {
        buffer.extend_from_slice(&chunk.context("读取 Page SSE 失败")?);
        while let Some((frame_end, delimiter_len)) = frame_boundary(&buffer) {
            let frame = buffer.drain(..frame_end).collect::<Vec<_>>();
            buffer.drain(..delimiter_len);
            let data = std::str::from_utf8(&frame)
                .context("Page SSE frame 不是有效 UTF-8")?
                .lines()
                .filter_map(|line| line.strip_prefix("data:"))
                .map(|value| value.strip_prefix(' ').unwrap_or(value))
                .collect::<Vec<_>>()
                .join("\n");
            if data.is_empty() {
                continue;
            }
            let value: Value =
                serde_json::from_str(&data).context("Page SSE data 不是有效 JSON")?;
            output::print_value(&value, output)?;
        }
    }
    Ok(())
}

fn frame_boundary(buffer: &[u8]) -> Option<(usize, usize)> {
    let lf = buffer.windows(2).position(|window| window == b"\n\n");
    let crlf = buffer.windows(4).position(|window| window == b"\r\n\r\n");
    match (lf, crlf) {
        (Some(lf), Some(crlf)) if lf <= crlf => Some((lf, 2)),
        (Some(_), Some(crlf)) => Some((crlf, 4)),
        (Some(lf), None) => Some((lf, 2)),
        (None, Some(crlf)) => Some((crlf, 4)),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{PageCommand, frame_boundary, path, read_authorization_code};

    #[test]
    fn authorization_codes_are_trimmed_but_internal_whitespace_is_rejected() {
        assert_eq!(
            read_authorization_code(Some("  CODE-123  "), false).unwrap(),
            "CODE-123"
        );
        assert!(read_authorization_code(Some("CODE 123"), false).is_err());
        assert!(read_authorization_code(Some("   "), false).is_err());
        assert!(read_authorization_code(None, false).is_err());
    }

    #[test]
    fn page_paths_escape_session_id_delimiters() {
        assert_eq!(
            path("session/with?delimiters", "/context"),
            "/ui-sessions/session%2Fwith%3Fdelimiters/context"
        );
    }

    #[test]
    fn page_command_labels_include_the_host_command() {
        assert_eq!(
            PageCommand::Call {
                command: "list_projects".into(),
                args: "{}".into(),
                allow_destructive: false,
            }
            .label(),
            "call list_projects"
        );
        assert_eq!(PageCommand::Events.label(), "events");
    }

    #[test]
    fn page_sse_frames_support_lf_and_crlf_delimiters_across_buffer_boundaries() {
        assert_eq!(frame_boundary(b"data: {}\n\nrest"), Some((8, 2)));
        assert_eq!(frame_boundary(b"data: {}\r\n\r\nrest"), Some((8, 4)));
        assert_eq!(frame_boundary(b"data: {}\r\n"), None);
    }
}
