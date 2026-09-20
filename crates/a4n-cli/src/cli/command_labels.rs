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
            Self::Package { command } => format!("package {}", command.label()),
            Self::Project { command } => format!("project {}", command.label()),
            Self::Chapter { command } => format!("chapter {}", command.label()),
            Self::Workflow { command } => format!("workflow {}", command.label()),
            Self::Agent { command } => format!("agent {}", command.label()),
            Self::Media { command } => format!("media {}", command.label()),
            Self::Config { command } => format!("config {}", command.label()),
            Self::Context { command } => format!("context {}", command.label()),
            Self::Page { command } => format!("page {}", command.label()),
        }
    }

    fn auto_agent_phase(&self) -> Option<String> {
        let phase = match self {
            Self::Call { command, .. } => format!("CLI 调用 {command}"),
            Self::Task {
                command: TaskCommand::Watch { task_id, .. },
            } => format!("观察任务 {task_id}"),
            Self::Workflow {
                command:
                    WorkflowCommand::Watch {
                        workflow_run_id, ..
                    },
            } => format!("观察工作流 {workflow_run_id}"),
            Self::Workflow {
                command: WorkflowCommand::Resume { workflow_run_id },
            } => format!("恢复工作流 {workflow_run_id}"),
            Self::Project {
                command: ProjectCommand::Create { title, .. },
            } => format!("创建项目 {title}"),
            Self::Project {
                command: ProjectCommand::Delete { .. },
            } => "删除项目".into(),
            Self::Chapter {
                command: ChapterCommand::Import { number, .. },
            } => format!("导入第 {number} 章"),
            Self::Chapter {
                command: ChapterCommand::Update { .. },
            } => "更新章节".into(),
            Self::Media {
                command: MediaCommand::Download { media_path, .. },
            } => format!("下载媒体 {media_path}"),
            _ => return None,
        };
        Some(phase)
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

impl PackageCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Availability { .. } => "availability",
            Self::Operation { .. } => "operation",
            Self::Preferences => "preferences",
            Self::SetPreferences { .. } => "set-preferences",
            Self::Enable { .. } => "enable",
            Self::Disable { .. } => "disable",
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

impl AgentCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::Thread { command } => command.label(),
            Self::Ask { .. } => "ask",
            Self::Chat { .. } => "chat",
            Self::Run { .. } => "run",
            Self::RunShow { .. } => "run show",
            Self::Message { command } => command.label(),
            Self::Events { .. } => "events",
            Self::Approve { .. } => "approve",
            Self::Tools { .. } => "tools",
            Self::Tool { command } => command.label(),
            Self::Memory { command } => command.label(),
            Self::Child { command } => command.label(),
            Self::Register { .. } => "register",
            Self::Update { .. } => "update",
            Self::Heartbeat { .. } => "heartbeat",
            Self::Unregister { .. } => "unregister",
            Self::Status => "status",
        }
    }
}

impl AgentThreadCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::List { .. } => "thread list",
            Self::Default { .. } => "thread default",
            Self::Create { .. } => "thread create",
            Self::Show { .. } => "thread show",
            Self::Fork { .. } => "thread fork",
            Self::Update { .. } => "thread update",
            Self::Archive { .. } => "thread archive",
        }
    }
}

impl AgentMessageCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::List { .. } => "message list",
        }
    }
}

impl AgentToolCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::Describe { .. } => "tool describe",
        }
    }
}

impl AgentMemoryCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::List { .. } => "memory list",
            Self::Create { .. } => "memory create",
            Self::Retract { .. } => "memory retract",
        }
    }
}

impl AgentChildCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::Spawn { .. } => "child spawn",
            Self::Send { .. } => "child send",
            Self::Control { .. } => "child control",
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
