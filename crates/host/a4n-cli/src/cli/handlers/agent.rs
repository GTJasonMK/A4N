async fn run_agent(
    runtime: &mut CliRuntime,
    command: AgentCommand,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        AgentCommand::Thread { command } => {
            run_agent_thread(&mut runtime.client, command, output).await
        }
        AgentCommand::Ask { content } => {
            let content = read_text_source(&content)?.trim().to_string();
            validate_non_empty(&content, "content")?;
            // Fail before ensuring the default thread: without a snapshot the new Run cannot execute.
            let runtime_config_snapshot = required_agent_runtime_snapshot(&runtime.client)?;
            let thread = runtime
                .client
                .send_json(JsonRequest {
                    method: Method::POST,
                    path: "/agent-threads/default".into(),
                    body: Some(serde_json::to_value(EnsureMainAgentThreadInput {
                        title: None,
                    })?),
                    authenticated: true,
                })
                .await?;
            let thread_id = thread
                .get("id")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("默认主 Agent Thread 响应缺少有效 id"))?
                .to_string();
            let input = SendAgentMessageInput {
                thread_id,
                client_message_id: uuid::Uuid::new_v4().to_string(),
                content,
                runtime_config_snapshot: Some(runtime_config_snapshot),
                ..SendAgentMessageInput::default()
            };
            let value = post_agent_message(&mut runtime.client, input).await?;
            print_json(&value, output)
        }
        AgentCommand::Chat {
            thread_id,
            content,
            client_message_id,
            delivery_mode,
            execution_mode,
            model_config_id,
            budget,
            editor_source,
        } => {
            validate_non_empty(&thread_id, "thread_id")?;
            let content = read_text_source(&content)?.trim().to_string();
            validate_non_empty(&content, "content")?;
            // Followups share the same entrypoint as new Runs; require the execution contract locally.
            let runtime_config_snapshot = required_agent_runtime_snapshot(&runtime.client)?;
            let budget = budget
                .as_deref()
                .map(read_json_source)
                .transpose()?
                .map(|value| {
                    serde_json::from_value::<AgentBudget>(value).context("budget 合同无效")
                })
                .transpose()?;
            let input = SendAgentMessageInput {
                thread_id: thread_id.clone(),
                client_message_id: client_message_id
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                content,
                delivery_mode: delivery_mode.into(),
                execution_mode: execution_mode.into(),
                model_config_id,
                budget,
                editor_source: editor_source
                    .as_deref()
                    .map(read_json_source)
                    .transpose()?
                    .map(serde_json::from_value)
                    .transpose()
                    .context("editor_source 合同无效")?,
                runtime_config_snapshot: Some(runtime_config_snapshot),
            };
            let value = post_agent_message(&mut runtime.client, input).await?;
            print_json(&value, output)
        }
        AgentCommand::Run {
            run_id,
            action,
            thread_id,
            budget,
        } => {
            validate_non_empty(&run_id, "run_id")?;
            validate_non_empty(&thread_id, "thread_id")?;
            let input = ControlAgentRunInput {
                thread_id,
                action: action.into(),
                run_id,
                budget: budget
                    .as_deref()
                    .map(read_json_source)
                    .transpose()?
                    .map(serde_json::from_value)
                    .transpose()
                    .context("budget 合同无效")?,
            };
            let value = runtime
                .client
                .send_json(JsonRequest {
                    method: Method::POST,
                    path: format!("/agent-runs/{}/control", path_segment(&input.run_id)),
                    body: Some(serde_json::to_value(input)?),
                    authenticated: true,
                })
                .await?;
            print_json(&value, output)
        }
        AgentCommand::RunShow { run_id } => {
            validate_non_empty(&run_id, "run_id")?;
            let value = runtime
                .client
                .send_json(get_request(&format!(
                    "/agent-runs/{}",
                    path_segment(&run_id)
                )))
                .await?;
            print_json(&value, output)
        }
        AgentCommand::Message { command } => {
            run_agent_message(&mut runtime.client, command, output).await
        }
        AgentCommand::Events {
            thread_id,
            after_sequence,
            limit,
            follow,
        } => {
            validate_non_empty(&thread_id, "thread_id")?;
            if !(1..=1_000).contains(&limit) {
                bail!("--limit 必须在 1 到 1000 之间");
            }
            let base = format!("/agent-threads/{}/events", path_segment(&thread_id));
            if follow {
                watch_agent_events(&mut runtime.client, &base, after_sequence, output).await
            } else {
                let path = query_path(
                    &base,
                    [
                        (
                            "after_sequence",
                            after_sequence.map(|value| value.to_string()),
                        ),
                        ("limit", Some(limit.to_string())),
                    ],
                );
                let value = runtime.client.send_json(get_request(&path)).await?;
                print_json(&value, output)
            }
        }
        AgentCommand::Approve {
            approval_id,
            decision,
            reason,
        } => {
            validate_non_empty(&approval_id, "approval_id")?;
            let input = ResolveAgentApprovalInput {
                approval_id: approval_id.clone(),
                decision: decision.into(),
                reason,
            };
            let value = runtime
                .client
                .send_json(JsonRequest {
                    method: Method::POST,
                    path: format!("/agent-approvals/{}", path_segment(&approval_id)),
                    body: Some(serde_json::to_value(input)?),
                    authenticated: true,
                })
                .await?;
            print_json(&value, output)
        }
        AgentCommand::Tools {
            query,
            domain,
            effect,
            capability,
            limit,
        } => {
            if !(1..=500).contains(&limit) {
                bail!("--limit 必须在 1 到 500 之间");
            }
            let path = query_path(
                "/agent-tools",
                [
                    ("query", query),
                    ("domain", domain),
                    ("effect", effect),
                    ("capability", capability),
                    ("limit", Some(limit.to_string())),
                ],
            );
            let value = runtime.client.send_json(get_request(&path)).await?;
            print_json(&value, output)
        }
        AgentCommand::Tool { command } => {
            run_agent_tool(&mut runtime.client, command, output).await
        }
        AgentCommand::Memory { command } => {
            run_agent_memory(&mut runtime.client, command, output).await
        }
        AgentCommand::Child { command } => {
            run_agent_child(&mut runtime.client, command, output).await
        }
        AgentCommand::Register {
            name,
            kind,
            client,
            project_id,
            chapter_id,
        } => {
            validate_non_empty(&name, "name")?;
            validate_non_empty(&kind, "kind")?;
            validate_non_empty(&client, "client")?;
            validate_optional_positive(project_id, "project_id")?;
            validate_optional_positive(chapter_id, "chapter_id")?;
            let value = runtime
                .client
                .register_agent(json!({
                    "name": name,
                    "kind": kind,
                    "client": client,
                    "project_id": project_id,
                    "chapter_id": chapter_id,
                }))
                .await?;
            print_json(&value, output)
        }
        AgentCommand::Status => {
            let value = runtime
                .client
                .send_json(JsonRequest {
                    method: Method::GET,
                    path: "/agents".into(),
                    body: None,
                    authenticated: true,
                })
                .await?;
            print_json(&value, output)
        }
        AgentCommand::Update {
            agent_id,
            status,
            phase,
            summary,
            progress,
            current_task_id,
            project_id,
            chapter_id,
        } => {
            let agent_id = resolve_agent_id(&runtime.client, agent_id)?;
            let body = agent_update_body(
                status,
                phase,
                summary,
                progress,
                current_task_id,
                project_id,
                chapter_id,
            )?;
            let value = runtime
                .client
                .send_agent_json(Method::PATCH, &format!("/agents/{agent_id}"), Some(body))
                .await?;
            print_json(&value, output)
        }
        AgentCommand::Heartbeat {
            agent_id,
            status,
            phase,
            summary,
            progress,
            current_task_id,
        } => {
            let agent_id = resolve_agent_id(&runtime.client, agent_id)?;
            let body = agent_update_body(
                status,
                phase,
                summary,
                progress,
                current_task_id,
                None,
                None,
            )?;
            let value = runtime
                .client
                .send_agent_json(
                    Method::POST,
                    &format!("/agents/{agent_id}/heartbeat"),
                    Some(body),
                )
                .await?;
            print_json(&value, output)
        }
        AgentCommand::Unregister { agent_id } => {
            let agent_id = resolve_agent_id(&runtime.client, agent_id)?;
            runtime
                .client
                .send_agent_json(Method::DELETE, &format!("/agents/{agent_id}"), None)
                .await?;
            runtime.client.clear_agent_credentials()?;
            print_json(
                &json!({ "unregistered": true, "agent_id": agent_id }),
                output,
            )
        }
    }
}

fn required_agent_runtime_snapshot(client: &ApiClient) -> Result<RuntimeConfigSnapshot> {
    client.runtime_config_snapshot().cloned().ok_or_else(|| {
        anyhow!(
            "Agent ask/chat/child send 缺少本地 runtime_config_snapshot；请先执行 `a4n config runtime set --input <JSON|@FILE|->`"
        )
    })
}

async fn run_agent_thread(
    client: &mut ApiClient,
    command: AgentThreadCommand,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        AgentThreadCommand::List {
            capability,
            include_archived,
            limit,
        } => {
            if !(1..=500).contains(&limit) {
                bail!("--limit 必须在 1 到 500 之间");
            }
            let path = query_path(
                "/agent-threads",
                [
                    ("capability", capability),
                    ("include_archived", Some(include_archived.to_string())),
                    ("limit", Some(limit.to_string())),
                ],
            );
            let value = client.send_json(get_request(&path)).await?;
            print_json(&value, output)
        }
        AgentThreadCommand::Default { title } => {
            let input = EnsureMainAgentThreadInput { title };
            let value = client
                .send_json(JsonRequest {
                    method: Method::POST,
                    path: "/agent-threads/default".into(),
                    body: Some(serde_json::to_value(input)?),
                    authenticated: true,
                })
                .await?;
            print_json(&value, output)
        }
        AgentThreadCommand::Create {
            kind,
            capability,
            title,
            project_id,
            chapter_id,
            resource_kind,
            resource_id,
            allowed_capability,
            allowed_tool,
            model_config_id,
        } => {
            validate_optional_positive(project_id, "project_id")?;
            validate_optional_positive(chapter_id, "chapter_id")?;
            if resource_kind.is_some() != resource_id.is_some() {
                bail!("--resource-kind 与 --resource-id 必须同时提供");
            }
            let bound_resource = resource_kind.map(|kind| AgentResourceRef {
                kind,
                id: resource_id.expect("resource id checked above"),
                label: None,
                revision: None,
                hash: None,
            });
            let input = CreateAgentThreadInput {
                kind: kind.into(),
                capability,
                title,
                bound_resource,
                scope: AgentScope {
                    project_id,
                    chapter_id,
                    allowed_capabilities: allowed_capability,
                    allowed_tools: allowed_tool,
                    ..AgentScope::default()
                },
                default_model_config_id: model_config_id,
            };
            let value = client
                .send_json(JsonRequest {
                    method: Method::POST,
                    path: "/agent-threads".into(),
                    body: Some(serde_json::to_value(input)?),
                    authenticated: true,
                })
                .await?;
            print_json(&value, output)
        }
        AgentThreadCommand::Show { thread_id } => {
            validate_non_empty(&thread_id, "thread_id")?;
            let value = client
                .send_json(get_request(&format!(
                    "/agent-threads/{}",
                    path_segment(&thread_id)
                )))
                .await?;
            print_json(&value, output)
        }
        AgentThreadCommand::Fork {
            thread_id,
            title,
            capability,
        } => {
            validate_non_empty(&thread_id, "thread_id")?;
            let input = ForkAgentThreadInput {
                thread_id: thread_id.clone(),
                title,
                capability,
                bound_resource: None,
                scope: None,
            };
            let value = client
                .send_json(JsonRequest {
                    method: Method::POST,
                    path: format!("/agent-threads/{}/fork", path_segment(&thread_id)),
                    body: Some(serde_json::to_value(input)?),
                    authenticated: true,
                })
                .await?;
            print_json(&value, output)
        }
        AgentThreadCommand::Update {
            thread_id,
            title,
            model_config_id,
            expected_revision,
            allowed_tool,
            use_capability_tools,
        } => {
            validate_non_empty(&thread_id, "thread_id")?;
            validate_optional_positive(model_config_id, "model_config_id")?;
            if title.is_none()
                && model_config_id.is_none()
                && allowed_tool.is_empty()
                && !use_capability_tools
            {
                bail!(
                    "至少提供 --title、--model-config-id、--allowed-tool 或 --use-capability-tools"
                );
            }
            let input = UpdateAgentThreadInput {
                thread_id: thread_id.clone(),
                title,
                default_model_config_id: model_config_id,
                expected_revision,
                allowed_tools: if use_capability_tools {
                    Some(Vec::new())
                } else {
                    (!allowed_tool.is_empty()).then_some(allowed_tool)
                },
            };
            let value = client
                .send_json(JsonRequest {
                    method: Method::PATCH,
                    path: format!("/agent-threads/{}", path_segment(&thread_id)),
                    body: Some(serde_json::to_value(input)?),
                    authenticated: true,
                })
                .await?;
            print_json(&value, output)
        }
        AgentThreadCommand::Archive { thread_id } => {
            validate_non_empty(&thread_id, "thread_id")?;
            let input = ArchiveAgentThreadInput {
                thread_id: thread_id.clone(),
            };
            let value = client
                .send_json(JsonRequest {
                    method: Method::POST,
                    path: format!("/agent-threads/{}/archive", path_segment(&thread_id)),
                    body: Some(serde_json::to_value(input)?),
                    authenticated: true,
                })
                .await?;
            print_json(&value, output)
        }
    }
}

async fn post_agent_message(client: &mut ApiClient, input: SendAgentMessageInput) -> Result<Value> {
    let thread_id = input.thread_id.clone();
    client
        .send_json(JsonRequest {
            method: Method::POST,
            path: format!("/agent-threads/{}/messages", path_segment(&thread_id)),
            body: Some(serde_json::to_value(input)?),
            authenticated: true,
        })
        .await
}

async fn run_agent_message(
    client: &mut ApiClient,
    command: AgentMessageCommand,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        AgentMessageCommand::List {
            thread_id,
            before,
            limit,
        } => {
            validate_non_empty(&thread_id, "thread_id")?;
            if !(1..=1_000).contains(&limit) {
                bail!("--limit 必须在 1 到 1000 之间");
            }
            let path = query_path(
                &format!("/agent-threads/{}/messages", path_segment(&thread_id)),
                [("before", before), ("limit", Some(limit.to_string()))],
            );
            let value = client.send_json(get_request(&path)).await?;
            print_json(&value, output)
        }
    }
}

async fn run_agent_tool(
    client: &mut ApiClient,
    command: AgentToolCommand,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        AgentToolCommand::Describe { name } => {
            validate_non_empty(&name, "name")?;
            let value = client
                .send_json(get_request(&format!(
                    "/agent-tools/{}",
                    path_segment(&name)
                )))
                .await?;
            print_json(&value, output)
        }
    }
}

async fn run_agent_memory(
    client: &mut ApiClient,
    command: AgentMemoryCommand,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        AgentMemoryCommand::List {
            scope_kind,
            scope_id,
        } => {
            if let Some(scope_id) = scope_id.as_deref() {
                validate_non_empty(scope_id, "scope_id")?;
            }
            let path = query_path(
                "/agent-memories",
                [
                    (
                        "scope_kind",
                        scope_kind.map(|value| value.label().to_string()),
                    ),
                    ("scope_id", scope_id),
                ],
            );
            let value = client.send_json(get_request(&path)).await?;
            print_json(&value, output)
        }
        AgentMemoryCommand::Create {
            scope_kind,
            scope_id,
            memory_kind,
            content,
        } => {
            validate_non_empty(&scope_id, "scope_id")?;
            validate_non_empty(&memory_kind, "memory_kind")?;
            let content = read_text_source(&content)?.trim().to_string();
            validate_non_empty(&content, "content")?;
            let input = AgentMemoryInput {
                scope_kind: scope_kind.into(),
                scope_id,
                memory_kind,
                content,
                resource_refs: Vec::new(),
            };
            let value = client
                .send_json(JsonRequest {
                    method: Method::POST,
                    path: "/agent-memories".into(),
                    body: Some(serde_json::to_value(input)?),
                    authenticated: true,
                })
                .await?;
            print_json(&value, output)
        }
        AgentMemoryCommand::Retract { memory_id, reason } => {
            validate_non_empty(&memory_id, "memory_id")?;
            let input = RetractAgentMemoryInput {
                memory_id: memory_id.clone(),
                reason,
            };
            let value = client
                .send_json(JsonRequest {
                    method: Method::POST,
                    path: format!("/agent-memories/{}/retract", path_segment(&memory_id)),
                    body: Some(serde_json::to_value(input)?),
                    authenticated: true,
                })
                .await?;
            print_json(&value, output)
        }
    }
}

async fn run_agent_child(
    client: &mut ApiClient,
    command: AgentChildCommand,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        AgentChildCommand::Spawn {
            parent_run_id,
            capability,
            task,
            title,
            project_id,
            chapter_id,
            budget,
        } => {
            validate_non_empty(&parent_run_id, "parent_run_id")?;
            validate_non_empty(&capability, "capability")?;
            let task = read_text_source(&task)?.trim().to_string();
            validate_non_empty(&task, "task")?;
            let runtime_config_snapshot = required_agent_runtime_snapshot(client)?;
            let budget = budget
                .as_deref()
                .map(read_json_source)
                .transpose()?
                .map(|value| {
                    serde_json::from_value::<AgentBudget>(value).context("budget 合同无效")
                })
                .transpose()?;
            let input = SpawnAgentInput {
                parent_run_id: parent_run_id.clone(),
                capability,
                task,
                title,
                bound_resource: None,
                scope: AgentScope {
                    project_id,
                    chapter_id,
                    ..AgentScope::default()
                },
                success_criteria: Vec::new(),
                budget,
                runtime_config_snapshot: Some(runtime_config_snapshot),
            };
            let value = client
                .send_json(JsonRequest {
                    method: Method::POST,
                    path: format!("/agent-runs/{}/children", path_segment(&parent_run_id)),
                    body: Some(serde_json::to_value(input)?),
                    authenticated: true,
                })
                .await?;
            print_json(&value, output)
        }
        AgentChildCommand::Send {
            delegation_id,
            content,
            delivery_mode,
        } => {
            validate_non_empty(&delegation_id, "delegation_id")?;
            let content = read_text_source(&content)?.trim().to_string();
            validate_non_empty(&content, "content")?;
            let runtime_config_snapshot = required_agent_runtime_snapshot(client)?;
            let input = SendAgentMessageToChildInput {
                delegation_id: delegation_id.clone(),
                content,
                delivery_mode: delivery_mode.into(),
                runtime_config_snapshot: Some(runtime_config_snapshot),
            };
            let value = client
                .send_json(JsonRequest {
                    method: Method::POST,
                    path: format!(
                        "/agent-delegations/{}/messages",
                        path_segment(&delegation_id)
                    ),
                    body: Some(serde_json::to_value(input)?),
                    authenticated: true,
                })
                .await?;
            print_json(&value, output)
        }
        AgentChildCommand::Control {
            delegation_id,
            action,
        } => {
            validate_non_empty(&delegation_id, "delegation_id")?;
            let input = ControlChildAgentInput {
                delegation_id: delegation_id.clone(),
                action: action.into(),
            };
            let value = client
                .send_json(JsonRequest {
                    method: Method::POST,
                    path: format!(
                        "/agent-delegations/{}/control",
                        path_segment(&delegation_id)
                    ),
                    body: Some(serde_json::to_value(input)?),
                    authenticated: true,
                })
                .await?;
            print_json(&value, output)
        }
    }
}

async fn watch_agent_events(
    client: &mut ApiClient,
    base_path: &str,
    mut cursor: Option<u64>,
    output: &OutputOptions,
) -> Result<()> {
    let mut reconnect_attempt = 0_u32;
    loop {
        let path = query_path(
            base_path,
            [("after_sequence", cursor.map(|value| value.to_string()))],
        );
        let response = client.agent_events_stream(&path, cursor).await?;
        match sse::consume_agent_events(response, |event| {
            if let Some(sequence) = event.get("sequence").and_then(Value::as_u64) {
                cursor = Some(cursor.map_or(sequence, |current| current.max(sequence)));
            }
            print_json(event, output)
        })
        .await?
        {
            sse::StreamOutcome::Disconnected {
                last_sequence,
                reason,
            } => {
                cursor = last_sequence.or(cursor);
                reconnect_attempt = reconnect_attempt.saturating_add(1);
                let shift = reconnect_attempt.min(5);
                let delay_ms = 200_u64.saturating_mul(1_u64 << shift).min(5_000);
                if output.mode == OutputMode::Human {
                    if let Some(reason) = reason {
                        eprintln!("warning: Agent SSE 断开: {reason}");
                    }
                    eprintln!(
                        "warning: Agent SSE 将在 {delay_ms}ms 后从 sequence={} 续传",
                        cursor.map_or_else(|| "start".into(), |value| value.to_string())
                    );
                }
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            sse::StreamOutcome::Terminal(_) => {
                unreachable!("Agent event stream has no terminal event")
            }
        }
    }
}

fn resolve_agent_id(client: &ApiClient, agent_id: Option<String>) -> Result<String> {
    let agent_id = agent_id
        .or_else(|| client.agent_id().map(ToString::to_string))
        .ok_or_else(|| anyhow!("缺少 agent_id；请先执行 `a4n agent register` 或使用 --agent-id"))?;
    validate_non_empty(&agent_id, "agent_id")?;
    if !client.has_agent_credentials() {
        bail!("没有 Agent token；请先执行 `a4n agent register`");
    }
    Ok(agent_id)
}

fn agent_update_body(
    status: Option<String>,
    phase: Option<String>,
    summary: Option<String>,
    progress: Option<f32>,
    current_task_id: Option<String>,
    project_id: Option<i64>,
    chapter_id: Option<i64>,
) -> Result<Value> {
    if let Some(status) = &status
        && ![
            "idle",
            "running",
            "paused",
            "completed",
            "failed",
            "offline",
        ]
        .contains(&status.as_str())
    {
        bail!("status 必须是 idle、running、paused、completed、failed 或 offline");
    }
    if let Some(progress) = progress
        && (!progress.is_finite() || !(0.0..=100.0).contains(&progress))
    {
        bail!("progress 必须在 0 到 100 之间");
    }
    Ok(json!({
        "status": status,
        "phase": phase,
        "summary": summary,
        "progress": progress,
        "current_task_id": current_task_id,
        "project_id": project_id,
        "chapter_id": chapter_id,
    }))
}
