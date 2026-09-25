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
    if cursor.is_none() {
        let value = client
            .send_json(get_request(&task_path(task_id, attempt, "/snapshot")?))
            .await?;
        let snapshot: a4n_service_protocol::GenerationTaskSnapshot =
            serde_json::from_value(value.clone()).context("generation snapshot 合同无效")?;
        snapshot.validate().context("generation snapshot 合同无效")?;
        if snapshot.task_id != task_id || snapshot.attempt != attempt {
            bail!("generation snapshot task_id/attempt 与请求不一致");
        }
        if let Some(event) = snapshot.terminal_event {
            let event = serde_json::to_value(event)?;
            print_json(&event, output)?;
            return finish_generation_task(event);
        }
        print_json(&value, output)?;
        cursor = Some(snapshot.covers_through_sequence);
    }
    let mut reconnect_attempt = 0_u32;
    loop {
        let last_event_id = cursor.map(|sequence| format!("{attempt}:{sequence}"));
        let response = client
            .generation_stream(&path, last_event_id.as_deref())
            .await?;
        match sse::consume_generation_events(response, |event| print_json(event, output)).await? {
            sse::StreamOutcome::Terminal(event) => {
                return finish_generation_task(event);
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

fn finish_generation_task(event: Value) -> Result<()> {
    let kind = event
        .pointer("/event/kind")
        .and_then(Value::as_str)
        .expect("validated terminal generation event must have a terminal kind");
    match kind {
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
        _ => unreachable!("validated terminal generation event must have a known kind"),
    }
}
