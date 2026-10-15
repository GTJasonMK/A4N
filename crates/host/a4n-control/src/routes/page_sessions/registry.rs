impl PageSessionRegistry {
    pub(crate) fn new() -> Self {
        let (events, _) = tokio::sync::broadcast::channel(256);
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            page_cli_gate: Arc::new(tokio::sync::RwLock::new(())),
            events,
        }
    }

    pub(crate) fn page_cli_gate(&self) -> &tokio::sync::RwLock<()> {
        self.page_cli_gate.as_ref()
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<PageEvent> {
        self.events.subscribe()
    }

    fn create(
        &self,
        owner_user_id: i64,
        input: CreatePageSessionInput,
        page_cli_enabled: bool,
    ) -> Result<PageSessionCreateResponse, ApiError> {
        if !page_cli_enabled {
            return Err(ApiError::forbidden("当前账户未启用 Page CLI"));
        }
        let client = required_text(input.client, "client")?;
        let tab_id = required_text(input.tab_id, "tab_id")?;
        let now = chrono::Utc::now();
        let now_text = now.to_rfc3339();
        let (response, event_kind) = {
            let mut records = self.records.write();
            if let Some(record) = records.values_mut().find(|record| {
                record.owner_user_id == owner_user_id
                    && record.view.client == client
                    && record.view.tab_id == tab_id
                    && record.view.status != PageSessionStatus::Offline
            }) {
                touch(&mut record.view, now);
                (
                    PageSessionCreateResponse {
                        session: record.view.clone(),
                        ui_session_token: record.token.clone(),
                        lease_seconds: PAGE_LEASE_SECONDS,
                    },
                    "resumed",
                )
            } else {
                let id = Uuid::new_v4().to_string();
                let token = Uuid::new_v4().to_string();
                let view = PageSessionView {
                    protocol_version: PAGE_SESSION_PROTOCOL_VERSION,
                    ui_session_id: id.clone(),
                    mode: "page".into(),
                    status: PageSessionStatus::AwaitingAttach,
                    client: client.clone(),
                    tab_id: tab_id.clone(),
                    page_kind: None,
                    route: None,
                    project_id: None,
                    chapter_id: None,
                    active_tab: None,
                    selected_ids: Value::Object(Default::default()),
                    dirty_state: false,
                    local_state: Value::Object(Default::default()),
                    context_revision: 0,
                    agent_id: None,
                    operation_id: None,
                    last_operation: None,
                    created_at: now_text.clone(),
                    updated_at: now_text.clone(),
                    last_seen_at: now_text.clone(),
                    expires_at: lease_expiry(now),
                };
                let response = PageSessionCreateResponse {
                    session: view.clone(),
                    ui_session_token: token.clone(),
                    lease_seconds: PAGE_LEASE_SECONDS,
                };
                records.insert(
                    id,
                    PageRecord {
                        view: view.clone(),
                        token,
                        owner_user_id,
                        authorization: None,
                        operations: VecDeque::new(),
                        operation_cancellation: None,
                    },
                );
                (response, "created")
            }
        };
        self.publish(
            owner_user_id,
            event_kind,
            response.session.clone(),
            None,
            None,
            None,
        );
        Ok(response)
    }

    pub(crate) fn revoke_user_sessions(&self, owner_user_id: i64) {
        let now = chrono::Utc::now();
        let sessions = {
            let mut records = self.records.write();
            records
                .values_mut()
                .filter(|record| record.owner_user_id == owner_user_id)
                .filter_map(|record| {
                    let operation_id = record.view.operation_id.clone();
                    let operation = cancel_operation(
                        record,
                        operation_id.as_deref(),
                        "Page CLI 已停用",
                        now,
                    );
                    let changed = record.view.status != PageSessionStatus::Offline
                        || record.view.agent_id.is_some()
                        || operation_id.is_some()
                        || record.authorization.is_some();
                    if !changed {
                        return None;
                    }
                    record.authorization = None;
                    record.view.status = PageSessionStatus::Offline;
                    record.view.agent_id = None;
                    record.view.operation_id = None;
                    if let Some(operation) = operation.as_ref() {
                        record.view.last_operation = Some(operation.clone());
                    }
                    touch(&mut record.view, now);
                    Some((record.view.clone(), operation))
                })
                .collect::<Vec<_>>()
        };
        for (session, operation) in sessions {
            self.publish(
                owner_user_id,
                "offline",
                session,
                operation,
                None,
                Some("page_cli_disabled".into()),
            );
        }
    }

    fn list(&self, owner_user_id: i64) -> Vec<PageSessionView> {
        self.expire();
        let mut sessions: Vec<_> = self
            .records
            .read()
            .values()
            .filter(|record| record.owner_user_id == owner_user_id)
            .map(|record| record.view.clone())
            .collect();
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        sessions
    }

    fn get(&self, owner_user_id: i64, session_id: &str) -> Result<PageSessionView, ApiError> {
        self.expire();
        self.records
            .read()
            .get(session_id)
            .filter(|record| record.owner_user_id == owner_user_id)
            .map(|record| record.view.clone())
            .ok_or_else(|| ApiError::not_found("Page Session 不存在"))
    }

    fn commands(
        &self,
        owner_user_id: i64,
        session_id: &str,
        token: &str,
        is_admin: bool,
        page_cli_enabled: bool,
    ) -> Result<PageCommandManifest, ApiError> {
        self.expire();
        let records = self.records.read();
        let session = records
            .get(session_id)
            .filter(|record| record.owner_user_id == owner_user_id)
            .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
        authorize_token(session, owner_user_id, token)?;
        let view = &session.view;
        let commands = command_manifest(None, None, None)
            .commands
            .into_iter()
            .filter(|descriptor| {
                descriptor.auth != "public"
                    && descriptor
                        .page_scopes
                        .iter()
                        .any(|scope| page_scope_matches(scope, view.page_kind.as_deref()))
            })
            .map(|descriptor| {
                let reason = if !page_cli_enabled {
                    Some("当前账户未启用 Page CLI".into())
                } else if view.status == PageSessionStatus::Offline {
                    Some("当前 Page Session 已离线".into())
                } else if view.status == PageSessionStatus::Paused {
                    Some("当前 Page Session 已暂停".into())
                } else if view.agent_id.is_none() {
                    Some("当前 Page Session 尚未绑定 Agent".into())
                } else if view.operation_id.is_some() {
                    Some("当前 Page Session 正在执行操作".into())
                } else if descriptor.auth == "admin" && !is_admin {
                    Some("需要管理员权限".into())
                } else {
                    missing_page_context(&descriptor.required_ui_context, view)
                };
                PageCommandAvailability {
                    available: reason.is_none(),
                    reason,
                    descriptor,
                }
            })
            .collect();
        Ok(PageCommandManifest {
            protocol_version: PAGE_SESSION_PROTOCOL_VERSION,
            ui_session_id: session_id.to_string(),
            page_kind: view.page_kind.clone(),
            route: view.route.clone(),
            commands,
        })
    }

    pub(crate) fn command_cancellation_token(
        &self,
        owner_user_id: i64,
        session_id: &str,
        operation_id: &str,
        command: &str,
        session_token: &str,
        agent_id: &str,
        agent_token: &str,
        agents: &AgentRegistry,
    ) -> Result<tokio_util::sync::CancellationToken, ApiError> {
        self.expire();
        agents.authorize(owner_user_id, agent_id, agent_token)?;
        let records = self.records.read();
        let record = records
            .get(session_id)
            .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
        authorize_token(record, owner_user_id, session_token)?;
        if !matches!(
            record.view.status,
            PageSessionStatus::Running | PageSessionStatus::Paused
        )
            || record.view.operation_id.as_deref() != Some(operation_id)
        {
            return Err(ApiError::page_operation_cancelled());
        }
        let operation = record
            .operations
            .iter()
            .find(|operation| operation.operation_id == operation_id)
            .ok_or_else(|| ApiError::not_found("Page Operation 不存在"))?;
        if operation.agent_id.as_deref() != Some(agent_id) || operation.command != command {
            return Err(ApiError::forbidden("Page Operation 与 Host 命令身份不匹配"));
        }
        record
            .operation_cancellation
            .clone()
            .filter(|token| !token.is_cancelled())
            .ok_or_else(ApiError::page_operation_cancelled)
    }

    fn attach_by_code(
        &self,
        owner_user_id: i64,
        input: AttachPageSessionInput,
        agent_registry: &AgentRegistry,
        agent_token: &str,
        page_cli_enabled: bool,
    ) -> Result<PageAttachResponse, ApiError> {
        self.expire();
        if !page_cli_enabled {
            return Err(ApiError::forbidden("当前账户未启用 Page CLI"));
        }
        let session_id = self
            .records
            .read()
            .values()
            .find(|record| {
                record.owner_user_id == owner_user_id
                    && record.authorization.as_ref().is_some_and(|authorization| {
                        authorization.code == input.authorization_code
                            && authorization.expires_at > chrono::Utc::now()
                    })
            })
            .map(|record| record.view.ui_session_id.clone())
            .ok_or_else(|| ApiError::unauthorized("Page Session 授权码无效或已过期"))?;
        self.attach(
            owner_user_id,
            &session_id,
            input,
            agent_registry,
            agent_token,
            page_cli_enabled,
        )
    }

    fn authorize(
        &self,
        owner_user_id: i64,
        session_id: &str,
        token: &str,
        page_cli_enabled: bool,
    ) -> Result<PageAuthorization, ApiError> {
        self.expire();
        if !page_cli_enabled {
            return Err(ApiError::forbidden("当前账户未启用 Page CLI"));
        }
        let now = chrono::Utc::now();
        let code = format!("A4N-{}-{}", random_code_part(), random_code_part());
        let expires_at = now + chrono::Duration::seconds(AUTHORIZATION_SECONDS as i64);
        let view = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            authorize_token(record, owner_user_id, token)?;
            if !matches!(
                record.view.status,
                PageSessionStatus::AwaitingAttach | PageSessionStatus::Idle
            ) {
                return Err(ApiError::conflict("当前 Page Session 不允许重新授权"));
            }
            record.authorization = Some(PageAuthorizationRecord {
                code: code.clone(),
                expires_at,
            });
            record.view.status = PageSessionStatus::AwaitingAttach;
            touch(&mut record.view, now);
            record.view.clone()
        };
        self.publish(owner_user_id, "authorization_ready", view, None, None, None);
        Ok(PageAuthorization {
            ui_session_id: session_id.to_string(),
            authorization_code: code,
            expires_in: AUTHORIZATION_SECONDS,
        })
    }

    fn attach(
        &self,
        owner_user_id: i64,
        session_id: &str,
        input: AttachPageSessionInput,
        agent_registry: &AgentRegistry,
        agent_token: &str,
        page_cli_enabled: bool,
    ) -> Result<PageAttachResponse, ApiError> {
        self.expire();
        if !page_cli_enabled {
            return Err(ApiError::forbidden("当前账户未启用 Page CLI"));
        }
        agent_registry.authorize(owner_user_id, &input.agent_id, agent_token)?;
        let now = chrono::Utc::now();
        let response = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            if record.owner_user_id != owner_user_id {
                return Err(ApiError::forbidden("无权绑定该 Page Session"));
            }
            if record.view.status != PageSessionStatus::AwaitingAttach {
                return Err(ApiError::conflict("Page Session 当前不在等待绑定状态"));
            }
            let authorization = record
                .authorization
                .as_ref()
                .ok_or_else(|| ApiError::unauthorized("Page Session 尚未授权或授权码已使用"))?;
            if authorization.code != input.authorization_code || authorization.expires_at <= now {
                return Err(ApiError::unauthorized("Page Session 授权码无效或已过期"));
            }
            if record.view.agent_id.is_some() {
                return Err(ApiError::conflict("Page Session 已被其他 Agent 接管"));
            }
            record.authorization = None;
            record.view.agent_id = Some(input.agent_id);
            record.view.status = PageSessionStatus::Attached;
            touch(&mut record.view, now);
            PageAttachResponse {
                session: record.view.clone(),
                ui_session_token: record.token.clone(),
            }
        };
        self.publish(
            owner_user_id,
            "attached",
            response.session.clone(),
            None,
            None,
            None,
        );
        Ok(response)
    }

    fn heartbeat(
        &self,
        owner_user_id: i64,
        session_id: &str,
        token: &str,
        context: Option<PageContext>,
    ) -> Result<PageSessionView, ApiError> {
        self.expire();
        let now = chrono::Utc::now();
        let view = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            authorize_token(record, owner_user_id, token)?;
            if record.view.status == PageSessionStatus::Offline {
                return Err(ApiError::conflict("Page Session 已失效"));
            }
            if let Some(context) = context {
                apply_context(&mut record.view, context)?;
            }
            touch(&mut record.view, now);
            record.view.clone()
        };
        self.publish(owner_user_id, "heartbeat", view.clone(), None, None, None);
        Ok(view)
    }

    fn update_context(
        &self,
        owner_user_id: i64,
        session_id: &str,
        token: &str,
        context: PageContext,
    ) -> Result<PageSessionView, ApiError> {
        self.expire();
        let now = chrono::Utc::now();
        let view = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            authorize_token(record, owner_user_id, token)?;
            if record.view.status == PageSessionStatus::Offline {
                return Err(ApiError::conflict("Page Session 已失效"));
            }
            apply_context(&mut record.view, context)?;
            touch(&mut record.view, now);
            record.view.clone()
        };
        self.publish(
            owner_user_id,
            "context_updated",
            view.clone(),
            None,
            None,
            None,
        );
        Ok(view)
    }

    fn start_operation(
        &self,
        owner_user_id: i64,
        session_id: &str,
        token: &str,
        agent_id: String,
        agent_registry: &AgentRegistry,
        agent_token: &str,
        page_cli_enabled: bool,
        is_admin: bool,
        input: PageOperationInput,
    ) -> Result<PageOperationView, ApiError> {
        self.expire();
        if !page_cli_enabled {
            return Err(ApiError::forbidden("当前账户未启用 Page CLI"));
        }
        agent_registry.authorize(owner_user_id, &agent_id, agent_token)?;
        let now = chrono::Utc::now();
        let operation = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            authorize_token(record, owner_user_id, token)?;
            if record.view.agent_id.as_deref() != Some(agent_id.as_str()) {
                return Err(ApiError::forbidden("当前 Agent 未绑定该 Page Session"));
            }
            if record.view.operation_id.is_some() {
                return Err(ApiError::conflict("当前 Page Session 已有正在执行的操作"));
            }
            if matches!(
                record.view.status,
                PageSessionStatus::Paused | PageSessionStatus::Offline
            ) {
                return Err(ApiError::conflict("当前 Page Session 不允许开始操作"));
            }
            if let Some(expected) = input.context_revision
                && expected != record.view.context_revision
            {
                return Err(ApiError::conflict(
                    "Page Context 已变化，请重新读取当前页面上下文",
                ));
            }
            validate_operation_input(&input)?;
            validate_page_command(
                &input.command,
                &record.view,
                is_admin,
                input.allow_destructive,
            )?;
            let operation = PageOperationView {
                operation_id: Uuid::new_v4().to_string(),
                ui_session_id: session_id.to_string(),
                agent_id: Some(agent_id),
                command: input.command,
                page_kind: record.view.page_kind.clone(),
                phase: input.phase.unwrap_or_else(|| "running".into()),
                target: input.target,
                status: PageOperationStatus::Running,
                progress: None,
                summary: None,
                result: None,
                error: None,
                started_at: now.to_rfc3339(),
                updated_at: now.to_rfc3339(),
            };
            push_operation(record, operation.clone());
            record.view.operation_id = Some(operation.operation_id.clone());
            record.view.status = PageSessionStatus::Running;
            record.operation_cancellation = Some(tokio_util::sync::CancellationToken::new());
            touch(&mut record.view, now);
            operation
        };
        let session = self.get(owner_user_id, session_id)?;
        self.publish(
            owner_user_id,
            "operation_started",
            session,
            Some(operation.clone()),
            None,
            None,
        );
        Ok(operation)
    }

    fn update_operation(
        &self,
        owner_user_id: i64,
        session_id: &str,
        operation_id: &str,
        token: &str,
        agent_id: &str,
        agent_registry: &AgentRegistry,
        agent_token: &str,
        page_cli_enabled: bool,
        update: PageOperationUpdate,
    ) -> Result<PageOperationView, ApiError> {
        self.expire();
        if !page_cli_enabled {
            return Err(ApiError::forbidden("当前账户未启用 Page CLI"));
        }
        agent_registry.authorize(owner_user_id, agent_id, agent_token)?;
        let now = chrono::Utc::now();
        let (operation, session) = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            authorize_token(record, owner_user_id, token)?;
            let operation = record
                .operations
                .iter_mut()
                .find(|operation| operation.operation_id == operation_id)
                .ok_or_else(|| ApiError::not_found("Page Operation 不存在"))?;
            if operation.agent_id.as_deref() != Some(agent_id) {
                return Err(ApiError::forbidden("当前 Agent 未创建该 Page Operation"));
            }
            if operation.status == PageOperationStatus::Cancelled {
                return Err(ApiError::page_operation_cancelled());
            }
            apply_operation_update(operation, update)?;
            operation.updated_at = now.to_rfc3339();
            let operation = operation.clone();
            if matches!(
                operation.status,
                PageOperationStatus::Succeeded
                    | PageOperationStatus::Failed
                    | PageOperationStatus::Cancelled
            ) {
                record.view.operation_id = None;
                record.operation_cancellation = None;
                if record.view.status != PageSessionStatus::Paused {
                    record.view.status = if record.view.agent_id.is_some() {
                        PageSessionStatus::Attached
                    } else {
                        PageSessionStatus::Idle
                    };
                }
            }
            record.view.last_operation = Some(operation.clone());
            touch(&mut record.view, now);
            (operation, record.view.clone())
        };
        self.publish(
            owner_user_id,
            "operation_updated",
            session,
            Some(operation.clone()),
            None,
            None,
        );
        Ok(operation)
    }

    fn navigate(
        &self,
        owner_user_id: i64,
        session_id: &str,
        token: &str,
        agent_id: &str,
        agent_registry: &AgentRegistry,
        agent_token: &str,
        page_cli_enabled: bool,
        input: PageNavigationInput,
    ) -> Result<PageSessionView, ApiError> {
        self.expire();
        if !page_cli_enabled {
            return Err(ApiError::forbidden("当前账户未启用 Page CLI"));
        }
        agent_registry.authorize(owner_user_id, agent_id, agent_token)?;
        let now = chrono::Utc::now();
        validate_route(&input.route)?;
        let view = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            authorize_token(record, owner_user_id, token)?;
            if record.view.agent_id.as_deref() != Some(agent_id) {
                return Err(ApiError::conflict("Page Session 尚未绑定 Agent"));
            }
            if record.view.status == PageSessionStatus::Paused {
                return Err(ApiError::conflict("Page Session 已暂停，不能导航"));
            }
            if record.view.dirty_state {
                return Err(ApiError::conflict(
                    "当前页面有未保存修改，保存或放弃修改后才能导航",
                ));
            }
            record.view.context_revision = record.view.context_revision.saturating_add(1);
            record.view.route = Some(input.route.clone());
            if input.page_kind.is_some() {
                record.view.page_kind = input.page_kind.clone();
            }
            touch(&mut record.view, now);
            record.view.clone()
        };
        self.publish(
            owner_user_id,
            "navigate",
            view.clone(),
            None,
            Some(input.route),
            input.reason,
        );
        Ok(view)
    }

    fn control(
        &self,
        owner_user_id: i64,
        session_id: &str,
        token: &str,
        action: &str,
    ) -> Result<PageSessionView, ApiError> {
        self.expire();
        let now = chrono::Utc::now();
        let (view, operation) = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            authorize_token(record, owner_user_id, token)?;
            match action {
                "pause" => {
                    if record.view.agent_id.is_none() {
                        return Err(ApiError::conflict("Page Session 尚未绑定 Agent"));
                    }
                    record.view.status = PageSessionStatus::Paused;
                    touch(&mut record.view, now);
                    (record.view.clone(), None)
                }
                "resume" => {
                    if record.view.agent_id.is_none() {
                        return Err(ApiError::conflict("Page Session 尚未绑定 Agent"));
                    }
                    record.view.status = if record.view.operation_id.is_some() {
                        PageSessionStatus::Running
                    } else {
                        PageSessionStatus::Attached
                    };
                    touch(&mut record.view, now);
                    (record.view.clone(), None)
                }
                "cancel" => {
                    if record.view.status == PageSessionStatus::Offline {
                        return Err(ApiError::conflict("Page Session 已失效"));
                    }
                    let operation_id = record.view.operation_id.clone();
                    let operation = cancel_operation(
                        record,
                        operation_id.as_deref(),
                        "Page Session 已由用户 cancel",
                        now,
                    );
                    record.view.operation_id = None;
                    record.operation_cancellation = None;
                    record.view.status = if record.view.agent_id.is_some() {
                        PageSessionStatus::Attached
                    } else {
                        PageSessionStatus::Idle
                    };
                    record.view.last_operation = operation.clone();
                    touch(&mut record.view, now);
                    (record.view.clone(), operation)
                }
                "takeover" | "detach" => {
                    if record.view.status == PageSessionStatus::Offline {
                        (record.view.clone(), None)
                    } else {
                        let operation_id = record.view.operation_id.clone();
                        let operation = cancel_operation(
                            record,
                            operation_id.as_deref(),
                            &format!("Page Session 已由用户 {action}"),
                            now,
                        );
                        record.view.status = PageSessionStatus::Idle;
                        record.view.agent_id = None;
                        record.view.operation_id = None;
                        record.operation_cancellation = None;
                        record.authorization = None;
                        record.view.last_operation = operation.clone();
                        touch(&mut record.view, now);
                        (record.view.clone(), operation)
                    }
                }
                _ => return Err(ApiError::bad_request("未知 Page Session 控制操作")),
            }
        };
        self.publish(owner_user_id, action, view.clone(), operation, None, None);
        Ok(view)
    }

    fn expire(&self) {
        let now = chrono::Utc::now();
        let mut expired = Vec::new();
        {
            let mut records = self.records.write();
            for record in records.values_mut() {
                if record.view.expires_at <= now.to_rfc3339()
                    && record.view.status != PageSessionStatus::Offline
                {
                    let operation_id = record.view.operation_id.clone();
                    record.view.status = PageSessionStatus::Offline;
                    record.view.agent_id = None;
                    record.view.operation_id = None;
                    record.operation_cancellation = None;
                    record.authorization = None;
                    let operation = cancel_operation(
                        record,
                        operation_id.as_deref(),
                        "Page Session 心跳已过期",
                        now,
                    );
                    record.view.last_operation = operation.clone();
                    touch(&mut record.view, now);
                    expired.push((record.owner_user_id, record.view.clone(), operation));
                }
            }
        }
        for (owner, view, operation) in expired {
            self.publish(
                owner,
                "offline",
                view,
                operation,
                None,
                Some("heartbeat_expired".into()),
            );
        }
    }

    fn publish(
        &self,
        owner_user_id: i64,
        kind: &str,
        session: PageSessionView,
        operation: Option<PageOperationView>,
        route: Option<String>,
        reason: Option<String>,
    ) {
        let _ = self.events.send(PageEvent {
            owner_user_id,
            event: PageSessionEvent {
                kind: kind.into(),
                session,
                operation,
                route,
                reason,
            },
        });
    }
}
