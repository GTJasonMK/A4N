async fn run_package(
    client: &mut ApiClient,
    command: PackageCommand,
    output: &OutputOptions,
) -> Result<()> {
    let lifecycle_action = if matches!(&command, PackageCommand::Enable { .. }) {
        "enable"
    } else {
        "disable"
    };
    let (host_command, args) = match command {
        PackageCommand::Overview => ("get_package_overview", json!({})),
        PackageCommand::Availability { command_names } => (
            "list_package_command_availability",
            json!({ "input": { "commands": command_names } }),
        ),
        PackageCommand::Operation { operation_id } => {
            validate_non_empty(&operation_id, "operation_id")?;
            (
                "get_package_operation",
                json!({ "input": { "operation_id": operation_id } }),
            )
        }
        PackageCommand::Preferences => ("get_package_preferences", json!({})),
        PackageCommand::SetPreferences { hidden_packages } => {
            let hidden_package_ids = hidden_packages
                .into_iter()
                .map(|value| {
                    PackageId::parse(&value)
                        .map(|package| package.as_str().to_string())
                        .ok_or_else(|| anyhow!("未知功能包: {value}"))
                })
                .collect::<Result<Vec<_>>>()?;
            (
                "update_package_preferences",
                json!({ "input": { "hidden_package_ids": hidden_package_ids } }),
            )
        }
        PackageCommand::Enable {
            package_id,
            operation_id,
            expected_status_version,
            requested_version,
            yes,
        }
        | PackageCommand::Disable {
            package_id,
            operation_id,
            expected_status_version,
            requested_version,
            yes,
        } => {
            if !yes {
                bail!("package enable/disable 需要 --yes 显式授权");
            }
            let package =
                PackageId::parse(&package_id).ok_or_else(|| anyhow!("未知功能包: {package_id}"))?;
            (
                "start_package_lifecycle",
                json!({
                    "input": {
                        "package_id": package,
                        "action": lifecycle_action,
                        "operation_id": operation_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                        "expected_status_version": expected_status_version,
                        "requested_version": requested_version,
                    }
                }),
            )
        }
    };
    print_json(&invoke_host(client, host_command, args).await?, output)
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

#[cfg(test)]
mod shell_catalog_tests {
    use super::shell_completions;
    use a4n_api_contract::command_manifest::API_COMMANDS;

    #[test]
    fn shell_completions_include_builtin_subcommands_and_host_api_commands() {
        let completions = shell_completions();
        for value in [
            "exit",
            "help",
            "quit",
            "project",
            "project list",
            "call list_projects",
        ] {
            assert!(
                completions.iter().any(|item| item == value),
                "missing {value}"
            );
        }
        for command in API_COMMANDS {
            assert!(
                completions
                    .iter()
                    .any(|item| item == &format!("call {command}")),
                "missing API completion {command}"
            );
        }
    }
}

pub(crate) struct CallFiles<'a> {
    pub(crate) set_base64: &'a [String],
    pub(crate) set_text: &'a [String],
    pub(crate) save_base64: &'a [String],
    pub(crate) save_text: &'a [String],
    pub(crate) force: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct PageCallContext {
    pub(crate) session_id: String,
    pub(crate) operation_id: String,
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
    let value = invoke_call(client, context, command, args, files, None).await?;
    print_json(&value, output)
}

pub(crate) async fn invoke_call(
    client: &mut ApiClient,
    context: &SessionContext,
    command: &str,
    mut args: Value,
    files: CallFiles<'_>,
    page: Option<PageCallContext>,
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
    let request = routing::command_request(command, args)?;
    let mut value = match page {
        Some(page) => client
            .send_page_host_json(
                &request.path,
                request.body,
                &page.session_id,
                &page.operation_id,
            )
            .await,
        None => client.send_json(request).await,
    }
    .map_err(|error| client::redact_error_values(error, &secrets))?;
    save_outputs(&mut value, &outputs, files.force)?;
    client::redact_json_values(&mut value, &secrets);
    Ok(value)
}

pub(crate) fn request_secret_values(
    descriptor: &catalog::CommandDescriptor,
    args: &Value,
) -> Vec<String> {
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
    for field in fields {
        let field = field
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("CLI manifest context field 必须是字符串"))?;
        if object.contains_key(field) {
            continue;
        }
        let value = match field {
            "projectId" => context.project_id,
            "chapterId" => context.chapter_id,
            "snapshotId" => context.snapshot_id,
            _ => bail!("CLI manifest 包含未知 context field: {field}"),
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
