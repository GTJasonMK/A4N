mod client;
mod config;
mod context;
mod local;
mod output;
mod routing;
mod shell;
mod sse;

use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use a4n_api_contract::auth::{ChangePasswordInput, RevokeSessionInput};
use a4n_api_contract::command_manifest as catalog;
use a4n_api_contract::project::CreateProjectInput;
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;
use anyhow::{Context, Result, bail};
use base64::Engine as _;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use reqwest::{Method, Response};
use serde_json::{Map, Value, json};
use tokio::io::AsyncWriteExt;

use client::{ApiBody, ApiClient, JsonRequest};
use config::{StoredConfig, resolve_config_path};
use context::SessionContext;
use output::{OutputMode, OutputOptions};

#[derive(Debug, Parser)]
#[command(name = "a4n", version, about = "A4N headless Server CLI")]
struct Cli {
    /// A4N Server 地址；可省略 /api
    #[arg(long, global = true, value_name = "URL")]
    server: Option<String>,

    /// CLI 配置文件路径
    #[arg(long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,

    /// 显式持久化会话上下文；默认只在当前进程内保存
    #[arg(long, global = true, value_name = "PATH")]
    context: Option<PathBuf>,

    /// JSON 使用单行输出
    #[arg(long, global = true)]
    compact: bool,

    /// 输出格式；one-shot 默认保持原始 JSON。与文件型 --output 同时使用时放在子命令前
    #[arg(long, value_enum)]
    output: Option<OutputMode>,

    /// Agent 模式：结构化 envelope、禁止交互、关闭颜色
    #[arg(long, global = true)]
    agent: bool,

    /// 禁止读取交互输入；缺少参数时直接失败
    #[arg(long, global = true)]
    no_input: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// 检查 CLI、Server、数据库和本地能力依赖
    Doctor {
        /// 将 warning 也视为失败
        #[arg(long)]
        strict: bool,
    },
    /// 管理无前端本地 Server；不带子命令时启动后进入 shell
    Local {
        #[command(subcommand)]
        command: Option<LocalCommand>,
    },
    /// 启动适合人工连续操作的交互式 shell
    Shell {
        /// 不保存命令历史
        #[arg(long)]
        no_history: bool,
        /// 自定义 history 文件路径
        #[arg(long, value_name = "PATH", conflicts_with = "no_history")]
        history: Option<PathBuf>,
    },
    /// 检查 Server 健康状态
    Health,
    /// 登录、注册和会话管理
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// 按前端命令名调用完整 Host API
    Call {
        /// 命令名，例如 list_projects
        command: String,
        /// JSON、@文件路径或 -（stdin）；结构与前端 apiInvoke args 相同
        #[arg(long, default_value = "{}", value_name = "JSON|@FILE|-")]
        args: String,
        /// 将本地文件按 Base64 写入参数 JSON Pointer，例如 /input/data_base64=book.epub
        #[arg(long, value_name = "POINTER=PATH")]
        set_base64: Vec<String>,
        /// 将 UTF-8 文件内容写入参数 JSON Pointer，例如 /input/settings_json=config.json
        #[arg(long, value_name = "POINTER=PATH")]
        set_text: Vec<String>,
        /// 将响应中的 Base64 JSON Pointer 解码到文件，例如 /data_base64=book.epub
        #[arg(long, value_name = "POINTER=PATH")]
        save_base64: Vec<String>,
        /// 将响应中的字符串 JSON Pointer 写入文件；根 Pointer 写作 =config.json
        #[arg(long, value_name = "POINTER=PATH")]
        save_text: Vec<String>,
        /// 允许覆盖 --save-base64/--save-text 的输出文件
        #[arg(long)]
        force: bool,
        /// 允许执行命令目录中标记为 destructive 的操作
        #[arg(long)]
        allow_destructive: bool,
    },
    /// 列出 CLI 可调用的全部业务命令和桌面运行时等价入口
    Commands {
        /// 按命令名或等价入口模糊过滤
        filter: Option<String>,
    },
    /// 查看 Agent 可使用的完整 Host API 能力和机器合同
    Capabilities {
        #[command(subcommand)]
        command: CapabilitiesCommand,
    },
    /// 调用原始 /api 相对路径
    Request {
        method: HttpMethod,
        /// 例如 /projects 或 /commands/list_chapters
        path: String,
        /// JSON、@文件路径或 -（stdin）
        #[arg(long, value_name = "JSON|@FILE|-")]
        body: Option<String>,
        /// 不发送登录 token
        #[arg(long)]
        no_auth: bool,
        /// 将原始响应写入文件
        #[arg(long, value_name = "PATH")]
        output: Option<PathBuf>,
        /// 允许覆盖 --output 文件
        #[arg(long, requires = "output")]
        force: bool,
        /// 允许原始非 GET 请求；语义命令不需要此参数
        #[arg(long)]
        allow_unsafe_request: bool,
    },
    /// 查询、监听和取消 generation task
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    /// 高频项目操作；其他项目能力仍可通过 call 使用
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// 高频章节操作；其他章节能力仍可通过 call 使用
    Chapter {
        #[command(subcommand)]
        command: ChapterCommand,
    },
    /// 查询、观察、恢复和取消持久化工作流
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommand,
    },
    /// 下载 Server 管理的媒体文件
    Media {
        #[command(subcommand)]
        command: MediaCommand,
    },
    /// 查看或修改 CLI 本地配置
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// 查看或修改项目/章节/快照上下文
    Context {
        #[command(subcommand)]
        command: ContextCommand,
    },
}

#[derive(Debug, Subcommand)]
enum AuthCommand {
    /// 登录并保存 access/refresh token
    Login {
        #[arg(long)]
        username: String,
        /// 从 stdin 读取密码，避免进入 shell history
        #[arg(long)]
        password_stdin: bool,
    },
    /// 注册并保存登录会话
    Register {
        #[arg(long)]
        username: String,
        #[arg(long)]
        email: String,
        #[arg(long)]
        invite_code: String,
        /// 从 stdin 读取密码，避免进入 shell history
        #[arg(long)]
        password_stdin: bool,
    },
    /// 显示当前登录用户
    Status,
    /// 主动刷新并保存 access/refresh token
    Refresh,
    /// 修改密码；stdin 第一行旧密码、第二行新密码
    ChangePassword {
        #[arg(long)]
        passwords_stdin: bool,
    },
    /// 修改当前用户资料
    UpdateProfile {
        #[arg(long)]
        email: Option<String>,
        #[arg(long, conflicts_with = "clear_avatar_url")]
        avatar_url: Option<String>,
        #[arg(long)]
        clear_avatar_url: bool,
    },
    /// 列出当前用户的登录会话
    Sessions,
    /// 撤销一个登录会话
    RevokeSession {
        session_id: String,
        #[arg(long)]
        yes: bool,
    },
    /// 获取 OAuth2 授权地址
    OauthAuthorize { provider: String },
    /// 用 stdin 中的授权 code 完成 OAuth2 登录并保存 token
    OauthCallback {
        provider: String,
        #[arg(long)]
        code_stdin: bool,
    },
    /// 撤销当前 session 并清除本地 token
    Logout,
}

#[derive(Debug, Subcommand)]
enum TaskCommand {
    /// 查询任务摘要
    Get {
        task_id: String,
        #[arg(long, default_value_t = 1)]
        attempt: u32,
    },
    /// 查询任务 snapshot
    Snapshot {
        task_id: String,
        #[arg(long, default_value_t = 1)]
        attempt: u32,
    },
    /// 列出活跃任务
    Active {
        #[arg(long)]
        project_id: Option<i64>,
    },
    /// 查询历史任务
    History {
        #[arg(long)]
        project_id: Option<i64>,
        #[arg(long, default_value_t = 50)]
        limit: u32,
        #[arg(long)]
        cursor: Option<String>,
    },
    /// 取消任务
    Cancel {
        task_id: String,
        #[arg(long, default_value_t = 1)]
        attempt: u32,
        #[arg(long)]
        yes: bool,
    },
    /// 通过 SSE 输出事件，直到任务进入终态
    Watch {
        task_id: String,
        #[arg(long, default_value_t = 1)]
        attempt: u32,
        #[arg(long)]
        after_sequence: Option<u64>,
    },
}

#[derive(Debug, Subcommand)]
enum ProjectCommand {
    /// 列出当前用户可访问的项目
    List {
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long, default_value_t = 50)]
        page_size: u32,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        root_project_id: Option<i64>,
        #[arg(long)]
        access_scope: Option<String>,
    },
    /// 获取项目；省略 ID 时使用上下文
    Get { project_id: Option<i64> },
    /// 创建文本项目
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        project_type: String,
        /// 创建后写入当前上下文
        #[arg(long)]
        use_context: bool,
    },
    /// 验证并切换当前项目上下文
    Use { project_id: i64 },
    /// 删除项目
    Delete {
        project_id: Option<i64>,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ChapterCommand {
    /// 列出项目章节；省略项目 ID 时使用上下文
    List {
        #[arg(long)]
        project_id: Option<i64>,
    },
    /// 获取章节摘要；省略章节 ID 时使用上下文
    Get {
        chapter_id: Option<i64>,
        #[arg(long)]
        project_id: Option<i64>,
    },
    /// 验证并切换当前章节上下文
    Use {
        chapter_id: i64,
        #[arg(long)]
        project_id: Option<i64>,
    },
    /// 从文本或 @文件/- 导入章节
    Import {
        #[arg(long)]
        project_id: Option<i64>,
        #[arg(long)]
        number: i32,
        #[arg(long, value_name = "TEXT|@FILE|-")]
        content: String,
        #[arg(long)]
        title: Option<String>,
    },
    /// 更新章节正文
    Update {
        #[arg(long)]
        chapter_id: Option<i64>,
        #[arg(long, value_name = "TEXT|@FILE|-")]
        content: String,
        #[arg(long)]
        title: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum WorkflowCommand {
    /// 列出工作流运行记录
    List {
        #[arg(long)]
        project_id: Option<i64>,
        #[arg(long)]
        chapter_id: Option<i64>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        resumable_only: bool,
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// 获取工作流、检查点和 generation run
    Show {
        workflow_run_id: i64,
        #[arg(long)]
        no_generation_runs: bool,
    },
    /// 获取最近进度事件
    Events {
        workflow_run_id: i64,
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// 恢复可恢复的工作流
    Resume { workflow_run_id: i64 },
    /// 取消工作流或其任务
    Cancel {
        #[arg(long, conflicts_with = "task_id")]
        workflow_run_id: Option<i64>,
        #[arg(long, conflicts_with = "workflow_run_id")]
        task_id: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    /// 轮询进度事件直到工作流终态
    Watch {
        workflow_run_id: i64,
        #[arg(long, default_value_t = 1_000)]
        interval_ms: u64,
        #[arg(long)]
        timeout_seconds: Option<u64>,
    },
}

#[derive(Debug, Subcommand)]
enum LocalCommand {
    /// 迁移数据库并在后台启动本地 Server
    Start {
        #[arg(long)]
        no_sidecars: bool,
        #[arg(long)]
        no_qdrant: bool,
        /// 从源码 workspace 构建并使用 embedded-capabilities host
        #[arg(long, conflicts_with = "no_sidecars")]
        full: bool,
    },
    /// 查看 owned/external Server 状态
    Status,
    /// 仅停止由当前 CLI local state 所有的进程
    Stop {
        /// TERM 超时后允许强制终止
        #[arg(long)]
        force: bool,
    },
    /// 只运行数据库迁移
    Migrate,
    /// 查看本地 Server/Qdrant 日志尾部
    Logs {
        #[arg(long, default_value_t = 100)]
        lines: usize,
    },
}

#[derive(Debug, Subcommand)]
enum MediaCommand {
    /// 按 Server 返回的相对媒体路径下载文件
    Download {
        media_path: String,
        #[arg(long, value_name = "PATH")]
        output: PathBuf,
        /// 允许覆盖已有文件
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// 查看 Server、登录状态和配置路径；不会输出 token
    Show,
    /// 保存默认 Server 地址
    SetServer { url: String },
    /// 清除本地 token，不调用 Server
    ClearAuth,
    /// 管理自动注入 Host API 请求的本机运行时配置
    Runtime {
        #[command(subcommand)]
        command: RuntimeConfigCommand,
    },
}

#[derive(Debug, Subcommand)]
enum RuntimeConfigCommand {
    /// 查看运行时配置；密钥不会输出
    Show,
    /// 从 JSON、@文件或 stdin 保存完整 runtime_config_snapshot
    Set {
        #[arg(long, value_name = "JSON|@FILE|-")]
        input: String,
    },
    /// 清除已保存的运行时配置
    Clear,
}

#[derive(Debug, Subcommand)]
enum ContextCommand {
    /// 查看当前上下文；不会输出 token 或业务内容
    Show,
    /// 设置上下文；切换上级时自动清理下级
    Set {
        #[arg(long)]
        project_id: Option<i64>,
        #[arg(long)]
        chapter_id: Option<i64>,
        #[arg(long)]
        snapshot_id: Option<i64>,
    },
    /// 清除全部上下文
    Clear,
    /// 清除项目及其章节、快照
    ClearProject,
    /// 清除章节及其快照
    ClearChapter,
    /// 仅清除快照
    ClearSnapshot,
}

#[derive(Debug, Subcommand)]
enum CapabilitiesCommand {
    /// 列出命令描述符
    List {
        #[arg(long)]
        filter: Option<String>,
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        effect: Option<String>,
    },
    /// 查看一个命令的调用合同
    Describe { command: String },
    /// 导出完整能力 manifest
    Export,
    /// 检查当前 CLI 与 Server 的命令 manifest 是否兼容
    Check,
}

impl Command {
    fn label(&self) -> String {
        match self {
            Self::Doctor { .. } => "doctor".into(),
            Self::Local { command } => command.as_ref().map_or_else(
                || "local".into(),
                |command| format!("local {}", command.label()),
            ),
            Self::Shell { .. } => "shell".into(),
            Self::Health => "health".into(),
            Self::Auth { command } => format!("auth {}", command.label()),
            Self::Call { command, .. } => format!("call {command}"),
            Self::Commands { .. } => "commands".into(),
            Self::Capabilities { command } => {
                format!("capabilities {}", command.label())
            }
            Self::Request { method, .. } => format!("request {}", method.label()),
            Self::Task { command } => format!("task {}", command.label()),
            Self::Project { command } => format!("project {}", command.label()),
            Self::Chapter { command } => format!("chapter {}", command.label()),
            Self::Workflow { command } => format!("workflow {}", command.label()),
            Self::Media { command } => format!("media {}", command.label()),
            Self::Config { command } => format!("config {}", command.label()),
            Self::Context { command } => format!("context {}", command.label()),
        }
    }
}

impl CapabilitiesCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::List { .. } => "list",
            Self::Describe { .. } => "describe",
            Self::Export => "export",
            Self::Check => "check",
        }
    }
}

impl AuthCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::Login { .. } => "login",
            Self::Register { .. } => "register",
            Self::Status => "status",
            Self::Refresh => "refresh",
            Self::ChangePassword { .. } => "change-password",
            Self::UpdateProfile { .. } => "update-profile",
            Self::Sessions => "sessions",
            Self::RevokeSession { .. } => "revoke-session",
            Self::OauthAuthorize { .. } => "oauth-authorize",
            Self::OauthCallback { .. } => "oauth-callback",
            Self::Logout => "logout",
        }
    }
}

impl TaskCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::Get { .. } => "get",
            Self::Snapshot { .. } => "snapshot",
            Self::Active { .. } => "active",
            Self::History { .. } => "history",
            Self::Cancel { .. } => "cancel",
            Self::Watch { .. } => "watch",
        }
    }
}

impl ProjectCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::List { .. } => "list",
            Self::Get { .. } => "get",
            Self::Create { .. } => "create",
            Self::Use { .. } => "use",
            Self::Delete { .. } => "delete",
        }
    }
}

impl ChapterCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::List { .. } => "list",
            Self::Get { .. } => "get",
            Self::Use { .. } => "use",
            Self::Import { .. } => "import",
            Self::Update { .. } => "update",
        }
    }
}

impl WorkflowCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::List { .. } => "list",
            Self::Show { .. } => "show",
            Self::Events { .. } => "events",
            Self::Resume { .. } => "resume",
            Self::Cancel { .. } => "cancel",
            Self::Watch { .. } => "watch",
        }
    }
}

impl LocalCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::Start { .. } => "start",
            Self::Status => "status",
            Self::Stop { .. } => "stop",
            Self::Migrate => "migrate",
            Self::Logs { .. } => "logs",
        }
    }
}

impl MediaCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::Download { .. } => "download",
        }
    }
}

impl ConfigCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::SetServer { .. } => "set-server",
            Self::ClearAuth => "clear-auth",
            Self::Runtime { command } => command.label(),
        }
    }
}

impl RuntimeConfigCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::Show => "runtime show",
            Self::Set { .. } => "runtime set",
            Self::Clear => "runtime clear",
        }
    }
}

impl ContextCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::Set { .. } => "set",
            Self::Clear => "clear",
            Self::ClearProject => "clear-project",
            Self::ClearChapter => "clear-chapter",
            Self::ClearSnapshot => "clear-snapshot",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl From<HttpMethod> for Method {
    fn from(value: HttpMethod) -> Self {
        match value {
            HttpMethod::Get => Method::GET,
            HttpMethod::Post => Method::POST,
            HttpMethod::Put => Method::PUT,
            HttpMethod::Patch => Method::PATCH,
            HttpMethod::Delete => Method::DELETE,
        }
    }
}

impl HttpMethod {
    fn label(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Post => "post",
            Self::Put => "put",
            Self::Patch => "patch",
            Self::Delete => "delete",
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let mode = if cli.agent {
        OutputMode::AgentJson
    } else if matches!(
        cli.command,
        Command::Shell { .. } | Command::Local { command: None }
    ) && cli.output.is_none()
    {
        OutputMode::Human
    } else {
        cli.output.unwrap_or_default()
    };
    let output = OutputOptions::new(mode, cli.compact).for_command(cli.command.label());
    match run(cli, output.clone()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => output::print_error(&error, &output),
    }
}

async fn run(cli: Cli, output: OutputOptions) -> Result<()> {
    let Cli {
        server,
        config,
        context,
        agent,
        no_input,
        command,
        ..
    } = cli;
    let config_path = resolve_config_path(config)?;
    let stored = StoredConfig::load(&config_path)?;
    let client = ApiClient::new(config_path, stored, server.as_deref())?;
    let session_context = SessionContext::load(context, client.api_base())?;
    let mut runtime = CliRuntime {
        client,
        output,
        context: session_context,
        no_input: no_input || agent,
    };

    match command {
        Command::Shell {
            no_history,
            history,
        } => run_shell(&mut runtime, no_history, history).await,
        command => execute_command(&mut runtime, command).await,
    }
}

struct CliRuntime {
    client: ApiClient,
    output: OutputOptions,
    context: SessionContext,
    no_input: bool,
}

async fn execute_command(runtime: &mut CliRuntime, command: Command) -> Result<()> {
    let output = runtime.output.for_command(command.label());

    match command {
        Command::Doctor { strict } => {
            let outcome = local::doctor(
                runtime.client.api_base(),
                runtime.client.config_path(),
                strict,
            )
            .await?;
            if !outcome.healthy {
                return Err(output::CliError::new(
                    "doctor_failed",
                    "unavailable",
                    "A4N CLI 环境检查未通过",
                    7,
                )
                .with_details(outcome.report)
                .into());
            }
            print_json(&outcome.report, &output)
        }
        Command::Local { command } => run_local(runtime, command, &output).await,
        Command::Shell { .. } => bail!("不能在 shell 内再次启动 shell"),
        Command::Health => {
            let meta = runtime
                .client
                .send_json(JsonRequest {
                    method: Method::GET,
                    path: "/meta".into(),
                    body: None,
                    authenticated: false,
                })
                .await?;
            if meta.get("host").and_then(Value::as_str) != Some(catalog::HOST_IDENTITY) {
                return Err(output::CliError::new(
                    "server_identity_invalid",
                    "server",
                    "目标地址不是 A4N Server",
                    8,
                )
                .with_details(meta)
                .into());
            }
            print_json(
                &json!({ "server": runtime.client.api_base(), "status": "ok", "meta": meta }),
                &output,
            )
        }
        Command::Auth { command } => run_auth(&mut runtime.client, command, &output).await,
        Command::Call {
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
                bail!("未知 Host API 命令: {command}");
            }
            if catalog::is_destructive(&command) && !allow_destructive {
                return Err(output::CliError::new(
                    "destructive_confirmation_required",
                    "confirmation",
                    format!("命令 `{command}` 属于危险操作，需要 --allow-destructive 显式授权"),
                    10,
                )
                .into());
            }
            run_call(
                &mut runtime.client,
                &runtime.context,
                &command,
                &args,
                CallFiles {
                    set_base64: &set_base64,
                    set_text: &set_text,
                    save_base64: &save_base64,
                    save_text: &save_text,
                    force,
                },
                &output,
            )
            .await
        }
        Command::Commands { filter } => print_json(&catalog::list(filter.as_deref()), &output),
        Command::Capabilities { command } => match command {
            CapabilitiesCommand::List {
                filter,
                domain,
                effect,
            } => print_json(
                &catalog::capabilities(filter.as_deref(), domain.as_deref(), effect.as_deref()),
                &output,
            ),
            CapabilitiesCommand::Describe { command } => {
                let descriptor = catalog::describe(&command)
                    .ok_or_else(|| anyhow::anyhow!("未知 Host API 命令: {command}"))?;
                print_json(&descriptor, &output)
            }
            CapabilitiesCommand::Export => {
                print_json(&catalog::capabilities(None, None, None), &output)
            }
            CapabilitiesCommand::Check => {
                let actual = runtime
                    .client
                    .send_json(JsonRequest {
                        method: Method::GET,
                        path: "/meta/commands".into(),
                        body: None,
                        authenticated: false,
                    })
                    .await?;
                let expected = catalog::capabilities(None, None, None);
                if actual != expected {
                    return Err(output::CliError::new(
                        "contract_incompatible",
                        "compatibility",
                        "CLI 与 Server 的 Host API manifest 不兼容",
                        11,
                    )
                    .with_details(json!({
                        "expected_manifest_version": expected.get("manifest_version"),
                        "actual_manifest_version": actual.get("manifest_version"),
                        "expected_command_count": expected.get("total_commands"),
                        "actual_command_count": actual.get("total_commands"),
                        "commands_match": actual.get("commands") == expected.get("commands"),
                        "runtime_equivalents_match": actual.get("runtime_equivalents") == expected.get("runtime_equivalents"),
                    }))
                    .into());
                }
                print_json(
                    &json!({
                        "compatible": true,
                        "manifest_version": expected["manifest_version"],
                        "command_count": expected["total_commands"],
                        "server": runtime.client.api_base(),
                    }),
                    &output,
                )
            }
        },
        Command::Request {
            method,
            path,
            body,
            no_auth,
            output: output_path,
            force,
            allow_unsafe_request,
        } => {
            if method != HttpMethod::Get && !allow_unsafe_request {
                return Err(output::CliError::new(
                    "destructive_confirmation_required",
                    "confirmation",
                    "原始非 GET 请求需要 --allow-unsafe-request 显式授权",
                    10,
                )
                .into());
            }
            let body = body.as_deref().map(read_json_source).transpose()?;
            let request = JsonRequest {
                method: method.into(),
                path,
                body,
                authenticated: !no_auth,
            };
            if let Some(output_path) = output_path.as_deref() {
                ensure_output_available(output_path, force)?;
                let response = runtime.client.send_response(request).await?;
                write_response_file(output_path, response, force)
                    .await
                    .map(|_| ())
            } else {
                output_body(runtime.client.send(request).await?, &output)
            }
        }
        Command::Task { command } => run_task(&mut runtime.client, command, &output).await,
        Command::Project { command } => run_project(runtime, command, &output).await,
        Command::Chapter { command } => run_chapter(runtime, command, &output).await,
        Command::Workflow { command } => run_workflow(runtime, command, &output).await,
        Command::Media { command } => run_media(&mut runtime.client, command, &output).await,
        Command::Config { command } => run_config(&mut runtime.client, command, &output),
        Command::Context { command } => run_context(runtime, command, &output).await,
    }
}

async fn run_shell(
    runtime: &mut CliRuntime,
    no_history: bool,
    history: Option<PathBuf>,
) -> Result<()> {
    if runtime.no_input {
        bail!("shell 需要交互输入，不能与 --agent 或 --no-input 一起使用");
    }
    let history = if no_history {
        None
    } else {
        Some(history.unwrap_or_else(|| {
            runtime
                .client
                .config_path()
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("history")
        }))
    };
    let mut shell = shell::InteractiveShell::new(history, shell_completions())?;
    println!(
        "A4N shell；输入 help 查看命令，exit 退出。当前 Server: {}",
        runtime.client.api_base()
    );
    loop {
        match shell.read()? {
            shell::ShellInput::Continue => continue,
            shell::ShellInput::Exit => return Ok(()),
            shell::ShellInput::Args(args) if matches!(args[0].as_str(), "exit" | "quit") => {
                return Ok(());
            }
            shell::ShellInput::Args(args) if args[0] == "help" => {
                print_shell_help(&args[1..])?;
            }
            shell::ShellInput::Args(args) => {
                let mut argv = vec!["a4n".to_string()];
                argv.extend(args);
                let parsed = match Cli::try_parse_from(argv) {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        error.print().context("输出 shell 命令帮助失败")?;
                        continue;
                    }
                };
                if parsed.server.is_some()
                    || parsed.config.is_some()
                    || parsed.context.is_some()
                    || parsed.output.is_some()
                    || parsed.compact
                    || parsed.agent
                    || parsed.no_input
                {
                    eprintln!("error: shell 会话的全局选项已固定；请退出后重新启动 shell");
                    continue;
                }
                if let Err(error) = execute_command(runtime, parsed.command).await {
                    let command_output = runtime.output.for_command("shell command");
                    let _ = output::print_error(&error, &command_output);
                }
            }
        }
    }
}

async fn run_local(
    runtime: &mut CliRuntime,
    command: Option<LocalCommand>,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        None => {
            if runtime.no_input {
                bail!("不带子命令的 local 会进入 shell，不能与 --agent 或 --no-input 一起使用");
            }
            let value = local::start(
                runtime.client.api_base(),
                runtime.client.config_path(),
                local::StartOptions {
                    managed_sidecars: true,
                    qdrant: true,
                    full: false,
                },
            )
            .await?;
            print_json(&value, output)?;
            Box::pin(run_shell(runtime, false, None)).await
        }
        Some(LocalCommand::Start {
            no_sidecars,
            no_qdrant,
            full,
        }) => {
            let value = local::start(
                runtime.client.api_base(),
                runtime.client.config_path(),
                local::StartOptions {
                    managed_sidecars: !no_sidecars && !full,
                    qdrant: !no_qdrant,
                    full,
                },
            )
            .await?;
            print_json(&value, output)
        }
        Some(LocalCommand::Status) => {
            let value =
                local::status(runtime.client.api_base(), runtime.client.config_path()).await?;
            print_json(&value, output)
        }
        Some(LocalCommand::Stop { force }) => {
            let value = local::stop(runtime.client.config_path(), force).await?;
            print_json(&value, output)
        }
        Some(LocalCommand::Migrate) => print_json(&local::migrate()?, output),
        Some(LocalCommand::Logs { lines }) => {
            print_json(&local::logs(runtime.client.config_path(), lines)?, output)
        }
    }
}

fn print_shell_help(path: &[String]) -> Result<()> {
    let mut command = Cli::command();
    for segment in path {
        let Some(next) = command
            .get_subcommands_mut()
            .find(|subcommand| subcommand.get_name() == segment)
        else {
            bail!("未知命令帮助路径: {}", path.join(" "));
        };
        command = next.clone();
    }
    command.print_help().context("输出 shell 帮助失败")?;
    println!();
    Ok(())
}

fn shell_completions() -> Vec<String> {
    fn collect(command: &clap::Command, prefix: &str, values: &mut Vec<String>) {
        for subcommand in command.get_subcommands() {
            let path = if prefix.is_empty() {
                subcommand.get_name().to_string()
            } else {
                format!("{prefix} {}", subcommand.get_name())
            };
            values.push(path.clone());
            collect(subcommand, &path, values);
        }
    }

    let mut values = vec!["exit".into(), "help".into(), "quit".into()];
    collect(&Cli::command(), "", &mut values);
    values.extend(
        catalog::API_COMMANDS
            .iter()
            .map(|command| format!("call {command}")),
    );
    values.sort_unstable();
    values.dedup();
    values
}

struct CallFiles<'a> {
    set_base64: &'a [String],
    set_text: &'a [String],
    save_base64: &'a [String],
    save_text: &'a [String],
    force: bool,
}

async fn run_call(
    client: &mut ApiClient,
    context: &SessionContext,
    command: &str,
    args_source: &str,
    files: CallFiles<'_>,
    output: &OutputOptions,
) -> Result<()> {
    let args = read_json_source(args_source)?;
    let value = invoke_call(client, context, command, args, files).await?;
    print_json(&value, output)
}

async fn invoke_call(
    client: &mut ApiClient,
    context: &SessionContext,
    command: &str,
    mut args: Value,
    files: CallFiles<'_>,
) -> Result<Value> {
    let auth_command = match command {
        "login_user" => Some("auth login"),
        "register_user" => Some("auth register"),
        "refresh_token" => Some("auth refresh"),
        "change_user_password" => Some("auth change-password"),
        "revoke_session" => Some("auth revoke-session"),
        "oauth2_callback" => Some("auth oauth-callback"),
        _ => None,
    };
    if let Some(replacement) = auth_command {
        bail!("{command} 必须使用 `a4n {replacement}`，以安全处理凭证和本地会话状态");
    }

    let descriptor = catalog::command_descriptor(command)
        .ok_or_else(|| anyhow::anyhow!("未知 Host API 命令: {command}"))?;
    apply_context_args(&mut args, &descriptor, context)?;
    apply_runtime_config(&mut args, &descriptor, client.runtime_config_snapshot())?;
    apply_file_inputs(&mut args, files.set_base64, files.set_text)?;
    let outputs = prepare_outputs(files.save_base64, files.save_text, files.force)?;
    let secrets = request_secret_values(&descriptor, &args);
    let mut value = client
        .send_json(routing::command_request(command, args)?)
        .await
        .map_err(|error| client::redact_error_values(error, &secrets))?;
    save_outputs(&mut value, &outputs, files.force)?;
    client::redact_json_values(&mut value, &secrets);
    Ok(value)
}

fn request_secret_values(descriptor: &catalog::CommandDescriptor, args: &Value) -> Vec<String> {
    let mut secrets = descriptor
        .secret_pointers
        .iter()
        .filter_map(|pointer| args.pointer(pointer))
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<HashSet<_>>();
    client::visit_sensitive_values(args, &mut |_, value| {
        if let Value::String(value) = value
            && !value.trim().is_empty()
        {
            secrets.insert(value.clone());
        }
    });
    secrets.into_iter().collect()
}

fn apply_context_args(
    args: &mut Value,
    descriptor: &catalog::CommandDescriptor,
    context: &SessionContext,
) -> Result<()> {
    let Some(fields) = descriptor
        .args_schema
        .get("x-a4n-context-fields")
        .and_then(Value::as_array)
    else {
        return Ok(());
    };
    let object = args
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("call --args 必须是 JSON object"))?;
    for field in fields.iter().filter_map(Value::as_str) {
        if object.contains_key(field) {
            continue;
        }
        let value = match field {
            "userId" => context.user_id,
            "projectId" => context.project_id,
            "chapterId" => context.chapter_id,
            "snapshotId" => context.snapshot_id,
            _ => None,
        };
        if let Some(value) = value {
            object.insert(field.to_string(), Value::from(value));
        }
    }
    Ok(())
}

fn apply_runtime_config(
    args: &mut Value,
    descriptor: &catalog::CommandDescriptor,
    snapshot: Option<&RuntimeConfigSnapshot>,
) -> Result<()> {
    let object = args
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("call --args 必须是 JSON object"))?;
    if object
        .get("runtime_config_snapshot")
        .is_some_and(Value::is_null)
    {
        object.remove("runtime_config_snapshot");
        return Ok(());
    }
    if object.contains_key("runtime_config_snapshot") {
        return Ok(());
    }
    let Some(snapshot) = snapshot else {
        return Ok(());
    };
    let snapshot = match descriptor.runtime_config_policy {
        catalog::RuntimeConfigPolicy::None => return Ok(()),
        catalog::RuntimeConfigPolicy::WithoutSecrets => snapshot.without_secrets(),
        catalog::RuntimeConfigPolicy::Full => snapshot.clone(),
    };
    object.insert(
        "runtime_config_snapshot".into(),
        serde_json::to_value(snapshot).context("序列化运行时配置失败")?,
    );
    Ok(())
}

async fn run_auth(
    client: &mut ApiClient,
    command: AuthCommand,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        AuthCommand::Login {
            username,
            password_stdin,
        } => {
            let password = read_password(password_stdin)?;
            let auth = client.login(username, password).await?;
            print_json(
                &json!({
                    "server": client.api_base(),
                    "user": auth.user,
                    "session": auth.session
                }),
                output,
            )
        }
        AuthCommand::Register {
            username,
            email,
            invite_code,
            password_stdin,
        } => {
            let password = read_password(password_stdin)?;
            let auth = client
                .register(username, email, invite_code, password)
                .await?;
            print_json(
                &json!({
                    "server": client.api_base(),
                    "user": auth.user,
                    "session": auth.session
                }),
                output,
            )
        }
        AuthCommand::Status => {
            let user = client.current_user().await?;
            print_json(&serde_json::to_value(user)?, output)
        }
        AuthCommand::Refresh => {
            let tokens = client.refresh().await?;
            print_json(
                &json!({
                    "refreshed": true,
                    "session_id": tokens.session_id,
                    "token_type": tokens.token_type,
                    "expires_in": tokens.expires_in
                }),
                output,
            )
        }
        AuthCommand::ChangePassword { passwords_stdin } => {
            let (old_password, new_password) = read_password_pair(passwords_stdin)?;
            let secrets = [old_password.clone(), new_password.clone()];
            client
                .send_json(routing::command_request(
                    "change_user_password",
                    json!({
                        "input": ChangePasswordInput { old_password, new_password }
                    }),
                )?)
                .await
                .map_err(|error| client::redact_error_values(error, &secrets))?;
            print_json(&json!({ "password_changed": true }), output)
        }
        AuthCommand::UpdateProfile {
            email,
            avatar_url,
            clear_avatar_url,
        } => {
            if email.is_none() && avatar_url.is_none() && !clear_avatar_url {
                bail!("至少提供 --email、--avatar-url 或 --clear-avatar-url 之一");
            }
            let mut input = Map::new();
            if let Some(email) = email {
                input.insert("email".to_string(), Value::String(email));
            }
            if clear_avatar_url {
                input.insert("avatar_url".to_string(), Value::Null);
            } else if let Some(avatar_url) = avatar_url {
                input.insert("avatar_url".to_string(), Value::String(avatar_url));
            }
            let value = client
                .send_json(routing::command_request(
                    "update_user_profile",
                    json!({ "input": Value::Object(input) }),
                )?)
                .await?;
            print_json(&value, output)
        }
        AuthCommand::Sessions => {
            let value = client
                .send_json(routing::command_request("list_sessions", json!({}))?)
                .await?;
            print_json(&value, output)
        }
        AuthCommand::RevokeSession { session_id, yes } => {
            if !yes {
                return Err(output::CliError::new(
                    "destructive_confirmation_required",
                    "confirmation",
                    "auth revoke-session 需要 --yes 显式授权",
                    10,
                )
                .into());
            }
            validate_non_empty(&session_id, "session_id")?;
            let current_session = client.session_id().map(str::to_string);
            client
                .send_json(routing::command_request(
                    "revoke_session",
                    json!({
                        "input": RevokeSessionInput {
                            session_id: session_id.clone()
                        }
                    }),
                )?)
                .await?;
            let revoked_current = current_session.as_deref() == Some(session_id.as_str());
            if revoked_current {
                client.clear_auth()?;
            }
            print_json(
                &json!({ "revoked": true, "session_id": session_id, "logged_out": revoked_current }),
                output,
            )
        }
        AuthCommand::OauthAuthorize { provider } => {
            let value = client
                .send_json(routing::command_request(
                    "oauth2_authorize_url",
                    json!({ "input": { "provider": provider } }),
                )?)
                .await?;
            print_json(&value, output)
        }
        AuthCommand::OauthCallback {
            provider,
            code_stdin,
        } => {
            let code = read_secret_line(
                code_stdin,
                "必须使用 --code-stdin 从 stdin 提供 OAuth2 code",
            )?;
            let auth = client.oauth2_callback(provider, code).await?;
            print_json(
                &json!({
                    "server": client.api_base(),
                    "user": auth.user,
                    "session": auth.session
                }),
                output,
            )
        }
        AuthCommand::Logout => {
            client.logout().await?;
            print_json(&json!({ "logged_out": true }), output)
        }
    }
}

async fn run_task(
    client: &mut ApiClient,
    command: TaskCommand,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        TaskCommand::Get { task_id, attempt } => {
            let value = client
                .send_json(get_request(&task_path(&task_id, attempt, "")?))
                .await?;
            print_json(&value, output)
        }
        TaskCommand::Snapshot { task_id, attempt } => {
            let value = client
                .send_json(get_request(&task_path(&task_id, attempt, "/snapshot")?))
                .await?;
            print_json(&value, output)
        }
        TaskCommand::Active { project_id } => {
            validate_optional_positive(project_id, "project_id")?;
            let path = query_path(
                "/generation-tasks/active",
                [("project_id", project_id.map(|value| value.to_string()))],
            );
            let value = client.send_json(get_request(&path)).await?;
            print_json(&value, output)
        }
        TaskCommand::History {
            project_id,
            limit,
            cursor,
        } => {
            validate_optional_positive(project_id, "project_id")?;
            if !(1..=100).contains(&limit) {
                bail!("--limit 必须在 1 到 100 之间");
            }
            let path = query_path(
                "/generation-tasks/history",
                [
                    ("project_id", project_id.map(|value| value.to_string())),
                    ("limit", Some(limit.to_string())),
                    ("cursor", cursor),
                ],
            );
            let value = client.send_json(get_request(&path)).await?;
            print_json(&value, output)
        }
        TaskCommand::Cancel {
            task_id,
            attempt,
            yes,
        } => {
            if !yes {
                return Err(output::CliError::new(
                    "destructive_confirmation_required",
                    "confirmation",
                    "task cancel 需要 --yes 显式授权",
                    10,
                )
                .into());
            }
            validate_non_empty(&task_id, "task_id")?;
            validate_attempt(attempt)?;
            let path = format!("/generation-tasks/{}/cancel", path_segment(&task_id));
            let value = client
                .send_json(JsonRequest {
                    method: Method::POST,
                    path,
                    body: Some(json!({ "task_id": task_id, "attempt": attempt })),
                    authenticated: true,
                })
                .await?;
            print_json(&value, output)
        }
        TaskCommand::Watch {
            task_id,
            attempt,
            after_sequence,
        } => watch_generation_task(client, &task_id, attempt, after_sequence, output).await,
    }
}

async fn run_project(
    runtime: &mut CliRuntime,
    command: ProjectCommand,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        ProjectCommand::List {
            page,
            page_size,
            kind,
            root_project_id,
            access_scope,
        } => {
            if page == 0 || !(1..=100).contains(&page_size) {
                bail!("--page 必须为正整数，--page-size 必须在 1 到 100 之间");
            }
            validate_optional_positive(root_project_id, "root_project_id")?;
            let value = runtime
                .client
                .send_json(routing::command_request(
                    "list_projects",
                    json!({
                        "page": page,
                        "pageSize": page_size,
                        "projectKind": kind,
                        "rootProjectId": root_project_id,
                        "accessScope": access_scope,
                    }),
                )?)
                .await?;
            print_json(&value, output)
        }
        ProjectCommand::Get { project_id } => {
            let project_id = resolve_id(project_id, runtime.context.project_id, "project_id")?;
            let value = runtime
                .client
                .send_json(routing::command_request(
                    "get_project",
                    json!({ "projectId": project_id }),
                )?)
                .await?;
            print_json(&value, output)
        }
        ProjectCommand::Create {
            title,
            description,
            project_type,
            use_context,
        } => {
            if title.trim().is_empty() {
                bail!("--title 不能为空");
            }
            let input = CreateProjectInput {
                title,
                description,
                project_type,
            };
            let value = runtime
                .client
                .send_json(routing::command_request(
                    "create_project",
                    json!({ "input": input }),
                )?)
                .await?;
            if use_context {
                let project_id = value
                    .get("id")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| anyhow::anyhow!("create_project 响应缺少有效 id"))?;
                bind_current_user(runtime).await?;
                runtime.context.set(Some(project_id), None, None)?;
            }
            print_json(&value, output)
        }
        ProjectCommand::Use { project_id } => {
            validate_positive_id(project_id, "project_id")?;
            let project = runtime
                .client
                .send_json(routing::command_request(
                    "get_project",
                    json!({ "projectId": project_id }),
                )?)
                .await?;
            bind_current_user(runtime).await?;
            runtime.context.set(Some(project_id), None, None)?;
            print_json(
                &json!({ "project": project, "context": runtime.context.as_json() }),
                output,
            )
        }
        ProjectCommand::Delete { project_id, yes } => {
            if !yes {
                return Err(output::CliError::new(
                    "destructive_confirmation_required",
                    "confirmation",
                    "project delete 需要 --yes 显式授权",
                    10,
                )
                .into());
            }
            let project_id = resolve_id(project_id, runtime.context.project_id, "project_id")?;
            let value = runtime
                .client
                .send_json(routing::command_request(
                    "delete_project",
                    json!({ "projectId": project_id }),
                )?)
                .await?;
            if runtime.context.project_id == Some(project_id) {
                runtime.context.clear_project()?;
            }
            print_json(
                &json!({ "deleted": true, "project_id": project_id, "result": value }),
                output,
            )
        }
    }
}

async fn run_chapter(
    runtime: &mut CliRuntime,
    command: ChapterCommand,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        ChapterCommand::List { project_id } => {
            let project_id = resolve_id(project_id, runtime.context.project_id, "project_id")?;
            let value = invoke_host(
                &mut runtime.client,
                "list_chapters",
                json!({ "projectId": project_id }),
            )
            .await?;
            print_json(&value, output)
        }
        ChapterCommand::Get {
            chapter_id,
            project_id,
        } => {
            let chapter_id = resolve_id(chapter_id, runtime.context.chapter_id, "chapter_id")?;
            let project_id = resolve_id(project_id, runtime.context.project_id, "project_id")?;
            let chapters = invoke_host(
                &mut runtime.client,
                "list_chapters",
                json!({ "projectId": project_id }),
            )
            .await?;
            let chapter = find_record_by_id(&chapters, chapter_id, "章节")?;
            print_json(chapter, output)
        }
        ChapterCommand::Use {
            chapter_id,
            project_id,
        } => {
            validate_positive_id(chapter_id, "chapter_id")?;
            let project_id = resolve_id(project_id, runtime.context.project_id, "project_id")?;
            let chapters = invoke_host(
                &mut runtime.client,
                "list_chapters",
                json!({ "projectId": project_id }),
            )
            .await?;
            let chapter = find_record_by_id(&chapters, chapter_id, "章节")?.clone();
            bind_current_user(runtime).await?;
            runtime
                .context
                .set(Some(project_id), Some(chapter_id), None)?;
            print_json(
                &json!({ "chapter": chapter, "context": runtime.context.as_json() }),
                output,
            )
        }
        ChapterCommand::Import {
            project_id,
            number,
            content,
            title,
        } => {
            let project_id = resolve_id(project_id, runtime.context.project_id, "project_id")?;
            if number <= 0 {
                bail!("--number 必须为正整数");
            }
            let content = read_text_source(&content)?;
            if content.trim().is_empty() {
                bail!("章节正文不能为空");
            }
            let value = invoke_host(
                &mut runtime.client,
                "import_chapter",
                json!({
                    "input": {
                        "project_id": project_id,
                        "chapter_number": number,
                        "content": content,
                        "title": title,
                    }
                }),
            )
            .await?;
            print_json(&value, output)
        }
        ChapterCommand::Update {
            chapter_id,
            content,
            title,
        } => {
            let chapter_id = resolve_id(chapter_id, runtime.context.chapter_id, "chapter_id")?;
            let content = read_text_source(&content)?;
            if content.trim().is_empty() {
                bail!("章节正文不能为空");
            }
            let value = invoke_host(
                &mut runtime.client,
                "update_chapter_content",
                json!({
                    "input": {
                        "chapter_id": chapter_id,
                        "content": content,
                        "title": title,
                    }
                }),
            )
            .await?;
            print_json(&value, output)
        }
    }
}

async fn run_workflow(
    runtime: &mut CliRuntime,
    command: WorkflowCommand,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        WorkflowCommand::List {
            project_id,
            chapter_id,
            kind,
            status,
            resumable_only,
            limit,
        } => {
            if !(1..=200).contains(&limit) {
                bail!("--limit 必须在 1 到 200 之间");
            }
            let project_id = project_id.or(runtime.context.project_id);
            let chapter_id = chapter_id.or(runtime.context.chapter_id);
            validate_optional_positive(project_id, "project_id")?;
            validate_optional_positive(chapter_id, "chapter_id")?;
            let value = invoke_host(
                &mut runtime.client,
                "list_workflow_runs",
                json!({
                    "input": {
                        "project_id": project_id,
                        "chapter_id": chapter_id,
                        "workflow_kind": kind,
                        "status": status,
                        "resumable_only": resumable_only,
                        "limit": limit,
                    }
                }),
            )
            .await?;
            print_json(&value, output)
        }
        WorkflowCommand::Show {
            workflow_run_id,
            no_generation_runs,
        } => {
            validate_positive_id(workflow_run_id, "workflow_run_id")?;
            let value =
                workflow_detail(&mut runtime.client, workflow_run_id, !no_generation_runs).await?;
            print_json(&value, output)
        }
        WorkflowCommand::Events {
            workflow_run_id,
            limit,
        } => {
            validate_positive_id(workflow_run_id, "workflow_run_id")?;
            if !(1..=200).contains(&limit) {
                bail!("--limit 必须在 1 到 200 之间");
            }
            let value = workflow_events(&mut runtime.client, workflow_run_id, limit).await?;
            print_json(&value, output)
        }
        WorkflowCommand::Resume { workflow_run_id } => {
            validate_positive_id(workflow_run_id, "workflow_run_id")?;
            let value = invoke_host(
                &mut runtime.client,
                "resume_workflow_run",
                json!({
                    "input": { "workflow_run_id": workflow_run_id },
                }),
            )
            .await?;
            print_json(&value, output)
        }
        WorkflowCommand::Cancel {
            workflow_run_id,
            task_id,
            yes,
        } => {
            if !yes {
                return Err(output::CliError::new(
                    "destructive_confirmation_required",
                    "confirmation",
                    "workflow cancel 需要 --yes 显式授权",
                    10,
                )
                .into());
            }
            if workflow_run_id.is_none() == task_id.is_none() {
                bail!("必须且只能提供 --workflow-run-id 或 --task-id 之一");
            }
            if let Some(id) = workflow_run_id {
                validate_positive_id(id, "workflow_run_id")?;
            }
            if let Some(task_id) = task_id.as_deref() {
                validate_non_empty(task_id, "task_id")?;
            }
            let value = invoke_host(
                &mut runtime.client,
                "cancel_workflow",
                json!({
                    "input": {
                        "workflow_run_id": workflow_run_id,
                        "task_id": task_id,
                    },
                }),
            )
            .await?;
            print_json(
                &json!({
                    "cancelled": true,
                    "workflow_run_id": workflow_run_id,
                    "result": value,
                }),
                output,
            )
        }
        WorkflowCommand::Watch {
            workflow_run_id,
            interval_ms,
            timeout_seconds,
        } => {
            validate_positive_id(workflow_run_id, "workflow_run_id")?;
            if !(200..=60_000).contains(&interval_ms) {
                bail!("--interval-ms 必须在 200 到 60000 之间");
            }
            watch_workflow(
                &mut runtime.client,
                workflow_run_id,
                interval_ms,
                timeout_seconds,
                output,
            )
            .await
        }
    }
}

async fn bind_current_user(runtime: &mut CliRuntime) -> Result<i64> {
    let user = runtime.client.current_user().await?;
    runtime.context.bind_user(user.id)?;
    Ok(user.id)
}

async fn invoke_host(client: &mut ApiClient, command: &str, args: Value) -> Result<Value> {
    client
        .send_json(routing::command_request(command, args)?)
        .await
}

fn resolve_id(explicit: Option<i64>, context: Option<i64>, field: &str) -> Result<i64> {
    let value = explicit
        .or(context)
        .ok_or_else(|| anyhow::anyhow!("缺少 {field}；请显式提供或先设置 context"))?;
    validate_positive_id(value, field)?;
    Ok(value)
}

fn validate_positive_id(value: i64, field: &str) -> Result<()> {
    if value <= 0 {
        bail!("{field} 必须为正整数");
    }
    Ok(())
}

fn find_record_by_id<'a>(value: &'a Value, id: i64, label: &str) -> Result<&'a Value> {
    value
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("id").and_then(Value::as_i64) == Some(id))
        })
        .ok_or_else(|| anyhow::anyhow!("{label}不存在或不属于当前项目: {id}"))
}

async fn workflow_detail(
    client: &mut ApiClient,
    workflow_run_id: i64,
    include_generation_runs: bool,
) -> Result<Value> {
    invoke_host(
        client,
        "get_workflow_run_detail",
        json!({
            "input": {
                "workflow_run_id": workflow_run_id,
                "include_generation_runs": include_generation_runs,
            },
        }),
    )
    .await
}

async fn workflow_events(
    client: &mut ApiClient,
    workflow_run_id: i64,
    limit: u32,
) -> Result<Value> {
    invoke_host(
        client,
        "get_workflow_progress_events",
        json!({
            "input": { "workflow_run_id": workflow_run_id, "limit": limit },
        }),
    )
    .await
}

async fn watch_workflow(
    client: &mut ApiClient,
    workflow_run_id: i64,
    interval_ms: u64,
    timeout_seconds: Option<u64>,
    output: &OutputOptions,
) -> Result<()> {
    let started = std::time::Instant::now();
    let mut seen = HashSet::new();
    loop {
        let events = workflow_events(client, workflow_run_id, 200).await?;
        // 响应是 { events, stream_text }：stream_text 是当前活体信号快照，
        // 无历史语义，watch 只消费去重后的事件列表。
        let items = events
            .get("events")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                workflow_contract_error(
                    workflow_run_id,
                    "progress events 响应缺少 events 数组",
                    events.clone(),
                )
            })?;
        for event in items {
            let key = event
                .get("id")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    workflow_contract_error(
                        workflow_run_id,
                        "progress event 缺少有效 id",
                        event.clone(),
                    )
                })?;
            if seen.insert(key.to_string()) {
                print_json(event, output)?;
            }
        }
        let detail = workflow_detail(client, workflow_run_id, false).await?;
        let status = detail
            .pointer("/run/status")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                workflow_contract_error(workflow_run_id, "响应缺少 run.status", detail.clone())
            })?;
        match status {
            "completed" => {
                print_json(&detail, output)?;
                return Ok(());
            }
            "failed" => {
                return Err(output::CliError::new(
                    "workflow_failed",
                    "server",
                    format!("workflow run {workflow_run_id} 终态为 {status}"),
                    8,
                )
                .with_details(detail)
                .into());
            }
            "cancelled" | "interrupted" | "superseded" => {
                return Err(output::CliError::new(
                    format!("workflow_{status}"),
                    "cancelled",
                    format!("workflow run {workflow_run_id} 终态为 {status}"),
                    9,
                )
                .with_details(detail)
                .into());
            }
            "running" => {}
            _ => {
                return Err(workflow_contract_error(
                    workflow_run_id,
                    &format!("返回未知状态: {status}"),
                    detail,
                ));
            }
        }
        if timeout_seconds.is_some_and(|seconds| started.elapsed().as_secs() >= seconds) {
            return Err(output::CliError::new(
                "workflow_watch_timeout",
                "unavailable",
                format!("等待 workflow run {workflow_run_id} 超时"),
                7,
            )
            .with_details(json!({ "workflow_run_id": workflow_run_id, "status": status }))
            .into());
        }
        tokio::time::sleep(Duration::from_millis(interval_ms)).await;
    }
}

fn workflow_contract_error(workflow_run_id: i64, message: &str, details: Value) -> anyhow::Error {
    output::CliError::new(
        "workflow_contract_invalid",
        "server",
        format!("workflow run {workflow_run_id} {message}"),
        8,
    )
    .with_details(details)
    .into()
}

async fn watch_generation_task(
    client: &mut ApiClient,
    task_id: &str,
    attempt: u32,
    mut cursor: Option<u64>,
    output: &OutputOptions,
) -> Result<()> {
    let path = task_path(task_id, attempt, "/events")?;
    let mut reconnect_attempt = 0_u32;
    loop {
        let last_event_id = cursor.map(|sequence| format!("{attempt}:{sequence}"));
        let response = client
            .generation_stream(&path, last_event_id.as_deref())
            .await?;
        match sse::consume_generation_events(response, |event| print_json(event, output)).await? {
            sse::StreamOutcome::Terminal(event) => {
                let kind = event
                    .pointer("/event/kind")
                    .and_then(Value::as_str)
                    .expect("terminal generation event must have a terminal kind");
                return match kind {
                    "completed" => Ok(()),
                    "cancelled" | "interrupted" => Err(output::CliError::new(
                        "task_cancelled",
                        "cancelled",
                        format!("generation task 终态为 {kind}"),
                        9,
                    )
                    .with_details(event)
                    .into()),
                    "failed" => Err(output::CliError::new(
                        "task_failed",
                        "server",
                        format!("generation task 终态为 {kind}"),
                        8,
                    )
                    .with_details(event)
                    .into()),
                    _ => unreachable!("SSE parser only returns known terminal task kinds"),
                };
            }
            sse::StreamOutcome::Disconnected {
                last_sequence,
                reason,
            } => {
                if last_sequence.is_some() {
                    cursor = last_sequence;
                    reconnect_attempt = 0;
                } else {
                    reconnect_attempt = reconnect_attempt.saturating_add(1);
                }
                let shift = reconnect_attempt.min(5);
                let delay_ms = 200_u64.saturating_mul(1_u64 << shift).min(5_000);
                if output.mode == OutputMode::Human {
                    if let Some(reason) = reason {
                        eprintln!("warning: generation SSE 断开: {reason}");
                    }
                    eprintln!(
                        "warning: generation SSE 将在 {delay_ms}ms 后从 sequence={} 续传",
                        cursor.map_or_else(|| "start".into(), |value| value.to_string())
                    );
                }
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }
}

async fn run_media(
    client: &mut ApiClient,
    command: MediaCommand,
    output_options: &OutputOptions,
) -> Result<()> {
    match command {
        MediaCommand::Download {
            media_path,
            output,
            force,
        } => print_json(
            &download_media(client, &media_path, &output, force).await?,
            output_options,
        ),
    }
}

async fn download_media(
    client: &mut ApiClient,
    media_path: &str,
    output: &Path,
    force: bool,
) -> Result<Value> {
    validate_non_empty(media_path, "media_path")?;
    if output.as_os_str().is_empty() {
        bail!("output 不能为空");
    }
    ensure_output_available(output, force)?;
    let response = client.download_media(media_path).await?;
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let byte_size = write_response_file(output, response, force).await?;
    Ok(json!({
        "media_path": media_path,
        "output": output,
        "byte_size": byte_size,
        "content_type": content_type,
    }))
}

fn run_config(
    client: &mut ApiClient,
    command: ConfigCommand,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        ConfigCommand::Show => print_json(
            &json!({
                "server": client.api_base(),
                "authenticated": client.is_authenticated(),
                "auth_source": client.auth_source(),
                "config_file": client.config_path()
            }),
            output,
        ),
        ConfigCommand::SetServer { url } => {
            client.set_server(&url)?;
            print_json(&json!({ "server": client.api_base() }), output)
        }
        ConfigCommand::ClearAuth => {
            client.clear_auth()?;
            print_json(
                &json!({
                    "stored_auth_cleared": true,
                    "authenticated": client.is_authenticated(),
                    "auth_source": client.auth_source(),
                }),
                output,
            )
        }
        ConfigCommand::Runtime { command } => match command {
            RuntimeConfigCommand::Show => print_json(
                &json!({
                    "configured": client.runtime_config_snapshot().is_some(),
                    "snapshot": client
                        .runtime_config_snapshot()
                        .map(RuntimeConfigSnapshot::without_secrets),
                    "config_file": client.config_path(),
                }),
                output,
            ),
            RuntimeConfigCommand::Set { input } => {
                let snapshot: RuntimeConfigSnapshot =
                    serde_json::from_value(read_json_source(&input)?)
                        .context("runtime_config_snapshot 无效")?;
                client.set_runtime_config_snapshot(snapshot)?;
                print_json(
                    &json!({
                        "configured": true,
                        "snapshot": client
                            .runtime_config_snapshot()
                            .map(RuntimeConfigSnapshot::without_secrets),
                    }),
                    output,
                )
            }
            RuntimeConfigCommand::Clear => {
                client.clear_runtime_config_snapshot()?;
                print_json(&json!({ "configured": false }), output)
            }
        },
    }
}

async fn run_context(
    runtime: &mut CliRuntime,
    command: ContextCommand,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        ContextCommand::Show => print_json(&runtime.context.as_json(), output),
        ContextCommand::Set {
            project_id,
            chapter_id,
            snapshot_id,
        } => {
            if project_id.is_none() && chapter_id.is_none() && snapshot_id.is_none() {
                bail!("context set 至少需要一个 ID");
            }
            validate_optional_positive(project_id, "project_id")?;
            validate_optional_positive(chapter_id, "chapter_id")?;
            validate_optional_positive(snapshot_id, "snapshot_id")?;
            let user = runtime.client.current_user().await?;
            runtime.context.bind_user(user.id)?;
            runtime.context.set(project_id, chapter_id, snapshot_id)?;
            print_json(&runtime.context.as_json(), output)
        }
        ContextCommand::Clear => {
            runtime.context.clear()?;
            print_json(&runtime.context.as_json(), output)
        }
        ContextCommand::ClearProject => {
            runtime.context.clear_project()?;
            print_json(&runtime.context.as_json(), output)
        }
        ContextCommand::ClearChapter => {
            runtime.context.clear_chapter()?;
            print_json(&runtime.context.as_json(), output)
        }
        ContextCommand::ClearSnapshot => {
            runtime.context.clear_snapshot()?;
            print_json(&runtime.context.as_json(), output)
        }
    }
}

fn get_request(path: &str) -> JsonRequest {
    JsonRequest {
        method: Method::GET,
        path: path.to_string(),
        body: None,
        authenticated: true,
    }
}

fn task_path(task_id: &str, attempt: u32, suffix: &str) -> Result<String> {
    validate_non_empty(task_id, "task_id")?;
    validate_attempt(attempt)?;
    Ok(format!(
        "/generation-tasks/{}{suffix}?attempt={attempt}",
        path_segment(task_id)
    ))
}

fn validate_attempt(attempt: u32) -> Result<()> {
    if attempt == 0 {
        bail!("attempt 必须为正整数");
    }
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{field} 不能为空");
    }
    Ok(())
}

fn validate_optional_positive(value: Option<i64>, field: &str) -> Result<()> {
    if value.is_some_and(|value| value <= 0) {
        bail!("{field} 必须为正整数");
    }
    Ok(())
}

fn query_path<const N: usize>(base: &str, pairs: [(&str, Option<String>); N]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in pairs {
        if let Some(value) = value {
            serializer.append_pair(key, &value);
        }
    }
    let query = serializer.finish();
    if query.is_empty() {
        base.to_string()
    } else {
        format!("{base}?{query}")
    }
}

fn path_segment(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn read_json_source(source: &str) -> Result<Value> {
    let contents = if source == "-" {
        let mut contents = String::new();
        std::io::stdin()
            .read_to_string(&mut contents)
            .context("从 stdin 读取 JSON 失败")?;
        contents
    } else if let Some(path) = source.strip_prefix('@') {
        if path.is_empty() {
            bail!("@ 后必须提供 JSON 文件路径");
        }
        std::fs::read_to_string(path).with_context(|| format!("读取 JSON 文件失败: {path}"))?
    } else {
        source.to_string()
    };
    serde_json::from_str(&contents).context("输入不是有效 JSON")
}

fn read_text_source(source: &str) -> Result<String> {
    if source == "-" {
        let mut contents = String::new();
        std::io::stdin()
            .read_to_string(&mut contents)
            .context("从 stdin 读取文本失败")?;
        return Ok(contents);
    }
    if let Some(path) = source.strip_prefix('@') {
        if path.is_empty() {
            bail!("@ 后必须提供文本文件路径");
        }
        return std::fs::read_to_string(path).with_context(|| format!("读取文本文件失败: {path}"));
    }
    Ok(source.to_string())
}

struct FileAssignment {
    pointer: String,
    path: PathBuf,
}

fn apply_file_inputs(
    args: &mut Value,
    base64_files: &[String],
    text_files: &[String],
) -> Result<()> {
    for raw in base64_files {
        let assignment = parse_file_assignment(raw, false)?;
        let bytes = std::fs::read(&assignment.path)
            .with_context(|| format!("读取文件失败: {}", assignment.path.display()))?;
        set_json_pointer(
            args,
            &assignment.pointer,
            Value::String(base64::engine::general_purpose::STANDARD.encode(bytes)),
        )?;
    }
    for raw in text_files {
        let assignment = parse_file_assignment(raw, false)?;
        let text = std::fs::read_to_string(&assignment.path)
            .with_context(|| format!("读取 UTF-8 文件失败: {}", assignment.path.display()))?;
        set_json_pointer(args, &assignment.pointer, Value::String(text))?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum OutputEncoding {
    Base64,
    Text,
}

struct OutputAssignment {
    file: FileAssignment,
    encoding: OutputEncoding,
}

fn prepare_outputs(
    base64_outputs: &[String],
    text_outputs: &[String],
    force: bool,
) -> Result<Vec<OutputAssignment>> {
    if force && base64_outputs.is_empty() && text_outputs.is_empty() {
        bail!("--force 只能与 --save-base64 或 --save-text 一起使用");
    }
    let assignments = base64_outputs
        .iter()
        .map(|raw| {
            parse_file_assignment(raw, true).map(|file| OutputAssignment {
                file,
                encoding: OutputEncoding::Base64,
            })
        })
        .chain(text_outputs.iter().map(|raw| {
            parse_file_assignment(raw, true).map(|file| OutputAssignment {
                file,
                encoding: OutputEncoding::Text,
            })
        }))
        .collect::<Result<Vec<_>>>()?;
    let mut paths = HashSet::new();
    for assignment in &assignments {
        if !paths.insert(assignment.file.path.clone()) {
            bail!("重复的输出文件: {}", assignment.file.path.display());
        }
        if assignment.file.path.exists() && !force {
            bail!(
                "输出文件已存在；使用 --force 覆盖: {}",
                assignment.file.path.display()
            );
        }
    }
    Ok(assignments)
}

fn save_outputs(value: &mut Value, assignments: &[OutputAssignment], force: bool) -> Result<()> {
    let decoded = assignments
        .iter()
        .map(|assignment| {
            let source = response_pointer(value, &assignment.file.pointer)
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "响应 {} 不是字符串",
                        display_pointer(&assignment.file.pointer)
                    )
                })?;
            let bytes = match assignment.encoding {
                OutputEncoding::Base64 => base64::engine::general_purpose::STANDARD
                    .decode(source)
                    .with_context(|| {
                        format!(
                            "响应 {} 不是有效 Base64",
                            display_pointer(&assignment.file.pointer)
                        )
                    })?,
                OutputEncoding::Text => source.as_bytes().to_vec(),
            };
            Ok((assignment, bytes))
        })
        .collect::<Result<Vec<_>>>()?;

    for (assignment, bytes) in decoded {
        write_output_file(&assignment.file.path, &bytes, force)?;
        let replacement = json!({
            "saved_to": assignment.file.path,
            "byte_size": bytes.len()
        });
        replace_response_pointer(value, &assignment.file.pointer, replacement)?;
    }
    Ok(())
}

fn response_pointer<'a>(value: &'a Value, pointer: &str) -> Option<&'a Value> {
    if pointer.is_empty() {
        Some(value)
    } else {
        value.pointer(pointer)
    }
}

fn replace_response_pointer(value: &mut Value, pointer: &str, replacement: Value) -> Result<()> {
    if pointer.is_empty() {
        *value = replacement;
        return Ok(());
    }
    let target = value
        .pointer_mut(pointer)
        .ok_or_else(|| anyhow::anyhow!("响应 JSON Pointer 不存在: {}", display_pointer(pointer)))?;
    *target = replacement;
    Ok(())
}

fn display_pointer(pointer: &str) -> &str {
    if pointer.is_empty() {
        "<root>"
    } else {
        pointer
    }
}

fn parse_file_assignment(raw: &str, allow_root: bool) -> Result<FileAssignment> {
    let (pointer, path) = raw
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("文件参数必须使用 POINTER=PATH: {raw}"))?;
    if (!allow_root || !pointer.is_empty()) && !pointer.starts_with('/') {
        bail!("JSON Pointer 必须以 / 开头: {pointer}");
    }
    if path.is_empty() {
        bail!("文件路径不能为空: {raw}");
    }
    Ok(FileAssignment {
        pointer: pointer.to_string(),
        path: PathBuf::from(path),
    })
}

fn set_json_pointer(root: &mut Value, pointer: &str, value: Value) -> Result<()> {
    let tokens = pointer
        .strip_prefix('/')
        .ok_or_else(|| anyhow::anyhow!("JSON Pointer 必须以 / 开头: {pointer}"))?
        .split('/')
        .map(decode_pointer_token)
        .collect::<Result<Vec<_>>>()?;
    let mut current = root;
    for (index, token) in tokens.iter().enumerate() {
        let last = index + 1 == tokens.len();
        match current {
            Value::Object(object) => {
                if last {
                    object.insert(token.clone(), value);
                    return Ok(());
                }
                current = object.entry(token).or_insert_with(|| json!({}));
            }
            Value::Array(array) => {
                let index = token
                    .parse::<usize>()
                    .map_err(|_| anyhow::anyhow!("数组 JSON Pointer 必须使用索引: {pointer}"))?;
                current = array
                    .get_mut(index)
                    .ok_or_else(|| anyhow::anyhow!("数组 JSON Pointer 越界: {pointer}"))?;
                if last {
                    *current = value;
                    return Ok(());
                }
            }
            Value::Null => {
                *current = json!({});
                let Value::Object(object) = current else {
                    unreachable!();
                };
                if last {
                    object.insert(token.clone(), value);
                    return Ok(());
                }
                current = object.entry(token).or_insert_with(|| json!({}));
            }
            _ => bail!("JSON Pointer 的父节点不是 object 或 array: {pointer}"),
        }
    }
    bail!("JSON Pointer 不能为空")
}

fn decode_pointer_token(token: &str) -> Result<String> {
    let mut decoded = String::with_capacity(token.len());
    let mut chars = token.chars();
    while let Some(ch) = chars.next() {
        if ch != '~' {
            decoded.push(ch);
            continue;
        }
        match chars.next() {
            Some('0') => decoded.push('~'),
            Some('1') => decoded.push('/'),
            _ => bail!("JSON Pointer 包含无效 ~ 转义: {token}"),
        }
    }
    Ok(decoded)
}

fn read_password(password_stdin: bool) -> Result<String> {
    read_secret_line(
        password_stdin,
        "必须使用 --password-stdin 从 stdin 提供密码，避免密码进入 shell history",
    )
}

fn read_password_pair(passwords_stdin: bool) -> Result<(String, String)> {
    if !passwords_stdin {
        bail!("必须使用 --passwords-stdin；stdin 第一行旧密码、第二行新密码");
    }
    let mut old_password = String::new();
    let mut new_password = String::new();
    std::io::stdin()
        .read_line(&mut old_password)
        .context("从 stdin 读取旧密码失败")?;
    std::io::stdin()
        .read_line(&mut new_password)
        .context("从 stdin 读取新密码失败")?;
    let old_password = trim_secret_line(old_password)?;
    let new_password = trim_secret_line(new_password)?;
    Ok((old_password, new_password))
}

fn read_secret_line(enabled: bool, missing_message: &str) -> Result<String> {
    if !enabled {
        bail!(missing_message.to_string());
    }
    let mut value = String::new();
    std::io::stdin()
        .read_line(&mut value)
        .context("从 stdin 读取敏感信息失败")?;
    trim_secret_line(value)
}

fn trim_secret_line(value: String) -> Result<String> {
    let value = value.trim_end_matches(['\r', '\n']).to_string();
    if value.is_empty() {
        bail!("敏感信息不能为空");
    }
    Ok(value)
}

fn output_body(body: ApiBody, output: &OutputOptions) -> Result<()> {
    if body
        .content_type
        .as_deref()
        .is_some_and(|value| value.contains("json") || value.starts_with("text/"))
    {
        return print_json(&body.into_value()?, output);
    }
    std::io::stdout()
        .write_all(&body.bytes)
        .context("写入 stdout 失败")
}

async fn write_response_file(path: &Path, mut response: Response, force: bool) -> Result<u64> {
    let expected_length = response.content_length();
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let temp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("创建临时输出文件失败: {}", parent.display()))?;
    let (file, temp_path) = temp.into_parts();
    let mut file = tokio::fs::File::from_std(file);

    let result = async {
        let mut written = 0_u64;
        while let Some(chunk) = response.chunk().await.map_err(client::transport_error)? {
            file.write_all(&chunk)
                .await
                .with_context(|| format!("写入输出文件失败: {}", path.display()))?;
            written = written
                .checked_add(chunk.len() as u64)
                .ok_or_else(|| anyhow::anyhow!("下载文件大小溢出 u64"))?;
        }
        if let Some(expected) = expected_length
            && expected != written
        {
            bail!("下载响应长度不匹配: expected={} actual={written}", expected);
        }
        file.sync_all()
            .await
            .with_context(|| format!("同步输出文件失败: {}", path.display()))?;
        Ok(written)
    }
    .await;

    drop(file);
    let written = result?;
    if force {
        temp_path.persist(path)
    } else {
        temp_path.persist_noclobber(path)
    }
    .map_err(|error| error.error)
    .with_context(|| {
        if path.exists() && !force {
            format!("输出文件已存在；使用 --force 覆盖: {}", path.display())
        } else {
            format!("原子写入输出文件失败: {}", path.display())
        }
    })?;
    Ok(written)
}

fn ensure_output_available(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        bail!("输出文件已存在；使用 --force 覆盖: {}", path.display());
    }
    Ok(())
}

fn write_output_file(path: &Path, bytes: &[u8], force: bool) -> Result<()> {
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("创建临时输出文件失败: {}", parent.display()))?;
    temp.write_all(bytes)
        .with_context(|| format!("写入输出文件失败: {}", path.display()))?;
    temp.as_file_mut()
        .sync_all()
        .with_context(|| format!("同步输出文件失败: {}", path.display()))?;
    if force {
        temp.persist(path)
    } else {
        temp.persist_noclobber(path)
    }
    .map_err(|error| error.error)
    .with_context(|| {
        if path.exists() && !force {
            format!("输出文件已存在；使用 --force 覆盖: {}", path.display())
        } else {
            format!("原子写入输出文件失败: {}", path.display())
        }
    })?;
    Ok(())
}

fn print_json(value: &Value, output: &OutputOptions) -> Result<()> {
    output::print_value(value, output)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use super::{
        RuntimeConfigSnapshot, SessionContext, apply_context_args, apply_runtime_config, catalog,
        prepare_outputs, query_path, read_json_source, request_secret_values, save_outputs,
        set_json_pointer,
    };

    #[test]
    fn inline_json_and_query_values_are_parsed() {
        assert_eq!(
            read_json_source(r#"{"projectId":1}"#).unwrap(),
            json!({"projectId": 1})
        );
        assert_eq!(
            query_path(
                "/generation-tasks/history",
                [("limit", Some("20".into())), ("cursor", Some("a/b".into()))]
            ),
            "/generation-tasks/history?limit=20&cursor=a%2Fb"
        );
    }

    #[test]
    fn file_json_pointers_create_inputs_and_decode_outputs_without_overwrite() {
        let mut input = json!({});
        set_json_pointer(&mut input, "/input/data_base64", json!("aGVsbG8=")).unwrap();
        assert_eq!(input["input"]["data_base64"], "aGVsbG8=");

        let path = std::env::temp_dir().join(format!("a4n-cli-{}.txt", Uuid::new_v4()));
        let spec = format!("/data_base64={}", path.display());
        let mut output = json!({ "data_base64": "aGVsbG8=" });
        let outputs = prepare_outputs(std::slice::from_ref(&spec), &[], false).unwrap();
        save_outputs(&mut output, &outputs, false).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
        assert_eq!(output["data_base64"]["byte_size"], 5);

        assert!(prepare_outputs(&[spec], &[], false).is_err());
        std::fs::remove_file(path).unwrap();

        let text_path = std::env::temp_dir().join(format!("a4n-cli-{}.json", Uuid::new_v4()));
        let mut text = json!("{\"ok\":true}");
        let outputs = prepare_outputs(&[], &[format!("={}", text_path.display())], false).unwrap();
        save_outputs(&mut text, &outputs, false).unwrap();
        assert_eq!(
            std::fs::read_to_string(&text_path).unwrap(),
            "{\"ok\":true}"
        );
        std::fs::remove_file(text_path).unwrap();
    }

    #[test]
    fn call_context_injects_only_explicit_resource_identity() {
        let mut context = SessionContext::load(None, "http://localhost/api").unwrap();
        context.bind_user(7).unwrap();
        let mut settings_args = json!({});
        let settings_descriptor = catalog::command_descriptor("detect_llm_config_models").unwrap();
        apply_context_args(&mut settings_args, &settings_descriptor, &context).unwrap();
        assert_eq!(settings_args, json!({}));

        let mut user_args = json!({});
        let user_descriptor = catalog::command_descriptor("create_scene_profile").unwrap();
        apply_context_args(&mut user_args, &user_descriptor, &context).unwrap();
        assert_eq!(user_args["userId"], 7);

        let mut admin_args = json!({});
        let admin_descriptor = catalog::command_descriptor("list_all_users").unwrap();
        apply_context_args(&mut admin_args, &admin_descriptor, &context).unwrap();
        assert_eq!(admin_args, json!({}));
    }

    #[test]
    fn call_runtime_config_respects_manifest_policy_and_explicit_override() {
        let snapshot: RuntimeConfigSnapshot = serde_json::from_value(json!({
            "schema_version": 1,
            "llm": {
                "id": 1,
                "name": "local",
                "provider": "openai",
                "base_url": "https://example.test/v1",
                "api_key_encrypted": "secret",
                "model": "model",
                "max_concurrent": 1,
                "timeout_secs": 30,
                "max_context_tokens": 4096,
                "max_output_tokens": 1024
            }
        }))
        .unwrap();

        let mut full = json!({});
        apply_runtime_config(
            &mut full,
            &catalog::command_descriptor("check_character_conflict").unwrap(),
            Some(&snapshot),
        )
        .unwrap();
        assert_eq!(
            full["runtime_config_snapshot"]["llm"]["api_key_encrypted"],
            "secret"
        );

        let mut preview = json!({});
        apply_runtime_config(
            &mut preview,
            &catalog::command_descriptor("preview_check_character_conflict_request").unwrap(),
            Some(&snapshot),
        )
        .unwrap();
        assert!(
            preview["runtime_config_snapshot"]["llm"]
                .get("api_key_encrypted")
                .is_none()
        );

        let mut no_runtime = json!({});
        apply_runtime_config(
            &mut no_runtime,
            &catalog::command_descriptor("list_projects").unwrap(),
            Some(&snapshot),
        )
        .unwrap();
        assert_eq!(no_runtime, json!({}));

        let mut explicit = json!({ "runtime_config_snapshot": { "schema_version": 1 } });
        apply_runtime_config(
            &mut explicit,
            &catalog::command_descriptor("check_character_conflict").unwrap(),
            Some(&snapshot),
        )
        .unwrap();
        assert_eq!(
            explicit["runtime_config_snapshot"],
            json!({ "schema_version": 1 })
        );

        let mut disabled = json!({ "runtime_config_snapshot": null });
        apply_runtime_config(
            &mut disabled,
            &catalog::command_descriptor("check_character_conflict").unwrap(),
            Some(&snapshot),
        )
        .unwrap();
        assert_eq!(disabled, json!({}));
    }

    #[test]
    fn call_redaction_collects_nested_runtime_config_secrets() {
        let descriptor = catalog::command_descriptor("apply_source_sync_decisions").unwrap();
        let secrets = request_secret_values(
            &descriptor,
            &json!({
                "runtime_config_snapshot": {
                    "llm": { "api_key_encrypted": "active-key" },
                    "llm_configs": [
                        { "api_key_encrypted": "first-key" },
                        { "api_key_encrypted": "second-key" }
                    ],
                    "system_llm_api_key_encrypted": "system-key",
                    "token_count": 4096
                }
            }),
        );
        for secret in ["active-key", "first-key", "second-key", "system-key"] {
            assert!(secrets.iter().any(|value| value == secret), "{secret}");
        }
        assert!(!secrets.iter().any(|value| value == "4096"));
    }
}
