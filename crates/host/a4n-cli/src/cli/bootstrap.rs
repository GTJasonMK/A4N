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
    let auto_agent_phase = agent.then(|| command.auto_agent_phase()).flatten();

    match command {
        Command::Shell {
            no_history,
            history,
        } => run_shell(&mut runtime, no_history, history).await,
        command => {
            let activity = if let Some(phase) = auto_agent_phase.as_deref() {
                Some(AutoAgentActivity::start(&runtime.client, phase.to_string()).await?)
            } else {
                None
            };
            let result = execute_command(&mut runtime, command).await;
            let tracking_result =
                if let (Some(activity), Some(phase)) = (activity, auto_agent_phase.as_deref()) {
                    Some(activity.finish(result.is_ok(), phase).await)
                } else {
                    None
                };
            match (result, tracking_result) {
                (Err(command_error), Some(Err(tracking_error))) => Err(anyhow!(
                    "CLI 命令失败: {command_error:#}; Agent 跟踪失败: {tracking_error:#}"
                )),
                (Err(command_error), _) => Err(command_error),
                (Ok(()), Some(Err(tracking_error))) => Err(tracking_error),
                (Ok(()), _) => Ok(()),
            }
        }
    }
}

pub(crate) struct CliRuntime {
    pub(crate) client: ApiClient,
    pub(crate) output: OutputOptions,
    pub(crate) context: SessionContext,
    pub(crate) no_input: bool,
}

struct AutoAgentActivity {
    heartbeat: AgentHeartbeatClient,
    task: tokio::task::JoinHandle<Result<()>>,
}

impl AutoAgentActivity {
    async fn start(client: &ApiClient, phase: String) -> Result<Self> {
        let heartbeat = client.agent_heartbeat_client()?;
        heartbeat.update("running", &phase, None).await?;
        let recurring = heartbeat.clone();
        let recurring_phase = phase.clone();
        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            interval.tick().await;
            let error = loop {
                interval.tick().await;
                match recurring.update("running", &recurring_phase, None).await {
                    Ok(()) => {}
                    Err(error) => break error,
                }
            };
            Err(error)
        });
        Ok(Self { heartbeat, task })
    }

    async fn finish(self, succeeded: bool, phase: &str) -> Result<()> {
        self.task.abort();
        match self.task.await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => return Err(error),
            Err(error) if error.is_cancelled() => {}
            Err(error) => return Err(anyhow!("Agent 心跳任务异常: {error}")),
        }
        let status = if succeeded { "completed" } else { "failed" };
        let summary = if succeeded {
            "CLI 命令已完成"
        } else {
            "CLI 命令执行失败"
        };
        self.heartbeat.update(status, phase, Some(summary)).await
    }
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
        Command::Package { command } => run_package(&mut runtime.client, command, &output).await,
        Command::Project { command } => run_project(runtime, command, &output).await,
        Command::Chapter { command } => run_chapter(runtime, command, &output).await,
        Command::Workflow { command } => run_workflow(runtime, command, &output).await,
        Command::Agent { command } => run_agent(runtime, command, &output).await,
        Command::Media { command } => run_media(&mut runtime.client, command, &output).await,
        Command::Config { command } => run_config(&mut runtime.client, command, &output),
        Command::Context { command } => run_context(runtime, command, &output).await,
        Command::Page { command } => page::run(runtime, command, &output).await,
    }
}
