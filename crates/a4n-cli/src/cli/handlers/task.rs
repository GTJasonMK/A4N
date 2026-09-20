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

