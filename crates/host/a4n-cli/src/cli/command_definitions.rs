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
    /// 查询和管理 Host 功能包
    Package {
        #[command(subcommand)]
        command: PackageCommand,
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
    /// 注册并报告当前 CLI Agent 的工作状态
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
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
    /// 根据当前 GUI 页面上下文操作 Page Session
    Page {
        #[command(subcommand)]
        command: page::PageCommand,
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
    OauthAuthorize {
        provider: String,
        #[arg(long)]
        redirect_uri: Option<String>,
    },
    /// 从 stdin 的完整回调 URL 提取授权码、校验 state 并完成 OAuth2 登录
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
    /// 读取当前快照并通过 SSE 监听至终态；指定 --after-sequence 时从该游标回放
    Watch {
        task_id: String,
        #[arg(long, default_value_t = 1)]
        attempt: u32,
        #[arg(long)]
        after_sequence: Option<u64>,
    },
}

#[derive(Debug, Subcommand)]
enum PackageCommand {
    /// 查看 Host 功能包状态和健康度
    Overview,
    /// 查询命令的实时包准入结果；可重复提供 --command
    Availability {
        #[arg(long = "command")]
        command_names: Vec<String>,
    },
    /// 查询一个生命周期操作
    Operation { operation_id: String },
    /// 查看当前用户的功能包导航偏好
    Preferences,
    /// 更新当前用户隐藏的可选功能包
    SetPreferences {
        #[arg(long = "hidden-package")]
        hidden_packages: Vec<String>,
    },
    /// 启用功能包
    Enable {
        package_id: String,
        #[arg(long)]
        operation_id: Option<String>,
        #[arg(long)]
        expected_status_version: Option<u64>,
        #[arg(long)]
        requested_version: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    /// 停用功能包；不会删除已有数据
    Disable {
        package_id: String,
        #[arg(long)]
        operation_id: Option<String>,
        #[arg(long)]
        expected_status_version: Option<u64>,
        #[arg(long)]
        requested_version: Option<String>,
        #[arg(long)]
        yes: bool,
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
        /// 读取章节时返回的 edit_revision，用于拒绝覆盖并发修改
        #[arg(long)]
        expected_revision: String,
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
enum AgentCommand {
    /// 管理持久化 Agent Thread
    Thread {
        #[command(subcommand)]
        command: AgentThreadCommand,
    },
    /// 向当前用户唯一的默认主 Agent 发送一句话
    Ask {
        /// 消息正文、@文件路径或 -（stdin）
        #[arg(value_name = "TEXT|@FILE|-")]
        content: String,
    },
    /// 向一个 Agent Thread 发送用户消息并创建或纠偏 Run
    Chat {
        thread_id: String,
        /// 消息正文、@文件路径或 -（stdin）
        #[arg(value_name = "TEXT|@FILE|-")]
        content: String,
        /// 客户端幂等消息 ID；省略时自动生成 UUID
        #[arg(long)]
        client_message_id: Option<String>,
        #[arg(long, value_enum, default_value_t = AgentDeliveryModeArg::Followup)]
        delivery_mode: AgentDeliveryModeArg,
        #[arg(long, value_enum, default_value_t = AgentExecutionModeArg::Autonomous)]
        execution_mode: AgentExecutionModeArg,
        #[arg(long)]
        model_config_id: Option<i64>,
        /// AgentBudget JSON、@文件路径或 -（stdin）
        #[arg(long, value_name = "JSON|@FILE|-")]
        budget: Option<String>,
        /// 绑定原编辑器定位元数据；不接受正文或方案内容
        #[arg(long, value_name = "JSON|@FILE|-")]
        editor_source: Option<String>,
    },
    /// 暂停、恢复或取消一个持久化 Agent Run
    Run {
        run_id: String,
        #[arg(value_enum)]
        action: AgentControlActionArg,
        /// Run 所属 Thread；用于权限和资源归属校验
        #[arg(long)]
        thread_id: String,
        /// 新的绝对预算上限 JSON（仅 resume）；不清空已有用量
        #[arg(long)]
        budget: Option<String>,
    },
    /// 获取一个持久化 Agent Run
    RunShow { run_id: String },
    /// 查询持久化 Agent 消息
    Message {
        #[command(subcommand)]
        command: AgentMessageCommand,
    },
    /// 查询或持续监听一个 Thread 的耐久事件
    Events {
        thread_id: String,
        #[arg(long)]
        after_sequence: Option<u64>,
        #[arg(long, default_value_t = 100)]
        limit: u32,
        /// 使用 SSE 持续监听并在断线后按 sequence 续传
        #[arg(long)]
        follow: bool,
    },
    /// 决定一次待用户确认的工具执行
    Approve {
        approval_id: String,
        #[arg(value_enum, default_value_t = AgentApprovalDecisionArg::Approve)]
        decision: AgentApprovalDecisionArg,
        #[arg(long)]
        reason: Option<String>,
    },
    /// 搜索 Agent 可发现的 Host 工具
    Tools {
        #[arg(long)]
        query: Option<String>,
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        effect: Option<String>,
        #[arg(long)]
        capability: Option<String>,
        #[arg(long, default_value_t = a4n_api_contract::agent::AGENT_TOOLS_DEFAULT_LIMIT)]
        limit: u32,
    },
    /// 查看一个 Agent Host 工具
    Tool {
        #[command(subcommand)]
        command: AgentToolCommand,
    },
    /// 管理 Agent 长期记忆
    Memory {
        #[command(subcommand)]
        command: AgentMemoryCommand,
    },
    /// 向已委派的子 Agent 发消息或控制其 Run
    Child {
        #[command(subcommand)]
        command: AgentChildCommand,
    },
    /// 注册一个 Agent 并保存返回的租约凭证
    Register {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "general")]
        kind: String,
        #[arg(long, default_value = "cli")]
        client: String,
        #[arg(long)]
        project_id: Option<i64>,
        #[arg(long)]
        chapter_id: Option<i64>,
    },
    /// 更新 Agent 工作状态
    Update {
        #[arg(long)]
        agent_id: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        phase: Option<String>,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        progress: Option<f32>,
        #[arg(long)]
        current_task_id: Option<String>,
        #[arg(long)]
        project_id: Option<i64>,
        #[arg(long)]
        chapter_id: Option<i64>,
    },
    /// 刷新 Agent 租约并可选更新状态
    Heartbeat {
        #[arg(long)]
        agent_id: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        phase: Option<String>,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        progress: Option<f32>,
        #[arg(long)]
        current_task_id: Option<String>,
    },
    /// 注销当前 Agent
    Unregister { agent_id: Option<String> },
    /// 查看当前用户的 Agent 列表
    Status,
}

#[derive(Debug, Subcommand)]
enum AgentThreadCommand {
    /// 列出当前用户的 Agent Threads
    List {
        #[arg(long)]
        capability: Option<String>,
        #[arg(long)]
        include_archived: bool,
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// 获取或创建当前用户唯一的主 Agent Thread
    Default {
        #[arg(long)]
        title: Option<String>,
    },
    /// 创建主 Agent 或领域子 Agent Thread
    Create {
        #[arg(long, value_enum, default_value_t = AgentThreadKindArg::Main)]
        kind: AgentThreadKindArg,
        #[arg(long)]
        capability: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        project_id: Option<i64>,
        #[arg(long)]
        chapter_id: Option<i64>,
        #[arg(long)]
        resource_kind: Option<String>,
        #[arg(long, requires = "resource_kind")]
        resource_id: Option<String>,
        #[arg(long)]
        allowed_capability: Vec<String>,
        #[arg(long)]
        allowed_tool: Vec<String>,
        #[arg(long)]
        model_config_id: Option<i64>,
    },
    /// 获取 Thread 的消息、Run、委派、审批和事件快照
    Show { thread_id: String },
    /// 从现有 Thread 创建一个继承其上下文的子 Agent Thread
    Fork {
        thread_id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        capability: Option<String>,
    },
    /// 更新 Thread 标题、默认模型或工具权限
    Update {
        thread_id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        model_config_id: Option<i64>,
        #[arg(long)]
        expected_revision: Option<u64>,
        #[arg(long, conflicts_with = "use_capability_tools")]
        allowed_tool: Vec<String>,
        /// 显式使用当前能力域工具目录；要求 --expected-revision 且没有活动 Run
        #[arg(long)]
        use_capability_tools: bool,
    },
    /// 归档一个 Thread
    Archive { thread_id: String },
}

#[derive(Debug, Subcommand)]
enum AgentMessageCommand {
    /// 按时间倒序列出 Thread 消息
    List {
        thread_id: String,
        #[arg(long)]
        before: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: u32,
    },
}

#[derive(Debug, Subcommand)]
enum AgentToolCommand {
    /// 查看工具的参数、风险和执行合同
    Describe { name: String },
}

#[derive(Debug, Subcommand)]
enum AgentMemoryCommand {
    /// 列出当前用户可见的 Agent memory
    List {
        #[arg(long, value_enum)]
        scope_kind: Option<AgentMemoryScopeKindArg>,
        #[arg(long)]
        scope_id: Option<String>,
    },
    /// 创建一条 Agent memory
    Create {
        #[arg(value_enum)]
        scope_kind: AgentMemoryScopeKindArg,
        scope_id: String,
        memory_kind: String,
        /// Memory 正文、@文件路径或 -（stdin）
        #[arg(value_name = "TEXT|@FILE|-")]
        content: String,
    },
    /// 撤回一条 Agent memory
    Retract {
        memory_id: String,
        #[arg(long)]
        reason: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum AgentChildCommand {
    /// 由用户在一个现有 Agent Run 下创建并启动 child Agent delegation
    Spawn {
        parent_run_id: String,
        capability: String,
        /// 任务正文、@文件路径或 -（stdin）
        #[arg(value_name = "TEXT|@FILE|-")]
        task: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        project_id: Option<i64>,
        #[arg(long)]
        chapter_id: Option<i64>,
        /// AgentBudget JSON、@文件路径或 -（stdin）
        #[arg(long, value_name = "JSON|@FILE|-")]
        budget: Option<String>,
    },
    /// 向委派的子 Agent 发送用户消息
    Send {
        delegation_id: String,
        /// 消息正文、@文件路径或 -（stdin）
        #[arg(value_name = "TEXT|@FILE|-")]
        content: String,
        #[arg(long, value_enum, default_value_t = AgentDeliveryModeArg::Followup)]
        delivery_mode: AgentDeliveryModeArg,
    },
    /// 暂停、恢复或取消委派的子 Agent
    Control {
        delegation_id: String,
        #[arg(value_enum)]
        action: AgentControlActionArg,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
enum AgentThreadKindArg {
    Main,
    Subagent,
}

impl From<AgentThreadKindArg> for AgentThreadKind {
    fn from(value: AgentThreadKindArg) -> Self {
        match value {
            AgentThreadKindArg::Main => Self::Main,
            AgentThreadKindArg::Subagent => Self::Subagent,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
enum AgentMemoryScopeKindArg {
    User,
    Project,
    Thread,
}

impl AgentMemoryScopeKindArg {
    fn label(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
            Self::Thread => "thread",
        }
    }
}

impl From<AgentMemoryScopeKindArg> for AgentMemoryScopeKind {
    fn from(value: AgentMemoryScopeKindArg) -> Self {
        match value {
            AgentMemoryScopeKindArg::User => Self::User,
            AgentMemoryScopeKindArg::Project => Self::Project,
            AgentMemoryScopeKindArg::Thread => Self::Thread,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
enum AgentDeliveryModeArg {
    Followup,
    Steer,
    Inject,
}

impl From<AgentDeliveryModeArg> for AgentDeliveryMode {
    fn from(value: AgentDeliveryModeArg) -> Self {
        match value {
            AgentDeliveryModeArg::Followup => Self::Followup,
            AgentDeliveryModeArg::Steer => Self::Steer,
            AgentDeliveryModeArg::Inject => Self::Inject,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
enum AgentExecutionModeArg {
    Autonomous,
    ReviewWrites,
    AdviceOnly,
}

impl From<AgentExecutionModeArg> for AgentExecutionMode {
    fn from(value: AgentExecutionModeArg) -> Self {
        match value {
            AgentExecutionModeArg::Autonomous => Self::Autonomous,
            AgentExecutionModeArg::ReviewWrites => Self::ReviewWrites,
            AgentExecutionModeArg::AdviceOnly => Self::AdviceOnly,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
enum AgentControlActionArg {
    Pause,
    Resume,
    Cancel,
}

impl From<AgentControlActionArg> for AgentControlAction {
    fn from(value: AgentControlActionArg) -> Self {
        match value {
            AgentControlActionArg::Pause => Self::Pause,
            AgentControlActionArg::Resume => Self::Resume,
            AgentControlActionArg::Cancel => Self::Cancel,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
enum AgentApprovalDecisionArg {
    Approve,
    Reject,
}

impl From<AgentApprovalDecisionArg> for AgentApprovalDecision {
    fn from(value: AgentApprovalDecisionArg) -> Self {
        match value {
            AgentApprovalDecisionArg::Approve => Self::Approve,
            AgentApprovalDecisionArg::Reject => Self::Reject,
        }
    }
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
