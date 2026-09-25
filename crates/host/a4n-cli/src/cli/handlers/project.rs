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
            expected_revision,
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
                        "expected_revision": expected_revision,
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
