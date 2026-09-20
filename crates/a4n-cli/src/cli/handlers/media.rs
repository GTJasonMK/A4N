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

