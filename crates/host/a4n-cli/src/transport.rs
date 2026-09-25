use std::path::{Path, PathBuf};

use a4n_api_contract::auth::{
    AuthResult, LoginInput, RefreshTokenInput, RegisterInput, RevokeSessionInput, TokenPairSummary,
    UserSummary,
};
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;
use anyhow::{Context, Result, anyhow, bail};
use reqwest::header::{ACCEPT, HeaderValue};
use reqwest::{Method, Response, StatusCode};
use serde_json::{Value, json};

use crate::client::{
    ApiBody, JsonRequest, ensure_success_response, read_response, redact_error_values,
    transport_error,
};
use crate::config::{
    StoredConfig, lock_config, non_empty_env_string, normalize_api_base, resolve_api_base,
};
use crate::routing;

mod oauth;

pub(crate) struct ApiClient {
    pub(super) http: reqwest::Client,
    pub(super) api_base: String,
    pub(super) config_path: PathBuf,
    pub(super) stored: StoredConfig,
    pub(super) access_token_override: Option<String>,
}

impl ApiClient {
    pub(crate) fn new(
        config_path: PathBuf,
        mut stored: StoredConfig,
        server_override: Option<&str>,
    ) -> Result<Self> {
        let api_base = resolve_api_base(server_override, stored.server.as_deref())?;
        let stored_server_matches = stored
            .server
            .as_deref()
            .map(normalize_api_base)
            .transpose()?
            .is_some_and(|server| server == api_base);
        if !stored_server_matches {
            stored.clear_auth();
        }
        let access_token_override = non_empty_env_string("A4N_ACCESS_TOKEN")?;
        let http = reqwest::Client::builder()
            .user_agent(concat!("a4n-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("创建 HTTP client 失败")?;
        Ok(Self {
            http,
            api_base,
            config_path,
            stored,
            access_token_override,
        })
    }

    pub(crate) fn api_base(&self) -> &str {
        &self.api_base
    }

    pub(crate) fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub(crate) fn is_authenticated(&self) -> bool {
        self.access_token().is_some()
    }

    pub(crate) fn auth_source(&self) -> Option<&'static str> {
        if self.access_token_override.is_some() {
            Some("environment")
        } else if self
            .stored
            .access_token
            .as_deref()
            .is_some_and(|token| !token.trim().is_empty())
        {
            Some("stored")
        } else {
            None
        }
    }

    pub(crate) fn session_id(&self) -> Option<&str> {
        self.access_token_override
            .is_none()
            .then_some(self.stored.session_id.as_deref())
            .flatten()
            .map(str::trim)
            .filter(|session_id| !session_id.is_empty())
    }

    pub(crate) fn agent_id(&self) -> Option<&str> {
        self.stored
            .agent_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    pub(crate) fn agent_token(&self) -> Option<&str> {
        self.stored
            .agent_token
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    pub(crate) fn ui_session_id(&self) -> Option<&str> {
        self.stored
            .ui_session_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    pub(crate) fn ui_session_token(&self) -> Option<&str> {
        self.stored
            .ui_session_token
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    pub(crate) fn has_agent_credentials(&self) -> bool {
        self.agent_id().is_some() && self.agent_token().is_some()
    }

    pub(crate) async fn register_agent(&mut self, body: Value) -> Result<Value> {
        let value = self
            .send_json(JsonRequest {
                method: Method::POST,
                path: "/agents".into(),
                body: Some(body),
                authenticated: true,
            })
            .await?;
        let agent_id = value
            .pointer("/agent/agent_id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Agent 注册响应缺少 agent.agent_id"))?;
        let agent_token = value
            .get("agent_token")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Agent 注册响应缺少 agent_token"))?;
        self.save_agent_credentials(agent_id.into(), agent_token.into())?;
        Ok(value)
    }

    pub(crate) fn agent_heartbeat_client(&self) -> Result<AgentHeartbeatClient> {
        Ok(AgentHeartbeatClient {
            http: self.http.clone(),
            api_base: self.api_base.clone(),
            agent_id: self
                .agent_id()
                .ok_or_else(|| anyhow!("自动 Agent 跟踪缺少 agent_id"))?
                .to_string(),
            access_token: self
                .access_token()
                .ok_or_else(|| anyhow!("未登录；无法自动跟踪 Agent"))?
                .to_string(),
            agent_token: self
                .agent_token()
                .ok_or_else(|| anyhow!("自动 Agent 跟踪缺少 agent token"))?
                .to_string(),
        })
    }

    pub(crate) fn save_agent_credentials(
        &mut self,
        agent_id: String,
        agent_token: String,
    ) -> Result<()> {
        let _lock = lock_config(&self.config_path)?;
        let mut stored = StoredConfig::load_unlocked(&self.config_path)?;
        self.ensure_stored_server_matches(&stored)?;
        stored.agent_id = Some(agent_id);
        stored.agent_token = Some(agent_token);
        stored.save_unlocked(&self.config_path)?;
        self.stored = stored;
        Ok(())
    }

    pub(crate) fn clear_agent_credentials(&mut self) -> Result<()> {
        let _lock = lock_config(&self.config_path)?;
        let mut stored = StoredConfig::load_unlocked(&self.config_path)?;
        self.ensure_stored_server_matches(&stored)?;
        stored.agent_id = None;
        stored.agent_token = None;
        stored.save_unlocked(&self.config_path)?;
        self.stored = stored;
        Ok(())
    }

    pub(crate) fn save_page_credentials(
        &mut self,
        session_id: String,
        session_token: String,
    ) -> Result<()> {
        let _lock = lock_config(&self.config_path)?;
        let mut stored = StoredConfig::load_unlocked(&self.config_path)?;
        self.ensure_stored_server_matches(&stored)?;
        stored.ui_session_id = Some(session_id);
        stored.ui_session_token = Some(session_token);
        stored.save_unlocked(&self.config_path)?;
        self.stored = stored;
        Ok(())
    }

    pub(crate) fn clear_page_credentials(&mut self) -> Result<()> {
        let _lock = lock_config(&self.config_path)?;
        let mut stored = StoredConfig::load_unlocked(&self.config_path)?;
        self.ensure_stored_server_matches(&stored)?;
        stored.ui_session_id = None;
        stored.ui_session_token = None;
        stored.save_unlocked(&self.config_path)?;
        self.stored = stored;
        Ok(())
    }

    pub(crate) fn runtime_config_snapshot(&self) -> Option<&RuntimeConfigSnapshot> {
        self.stored.runtime_config_snapshot.as_ref()
    }

    pub(crate) fn set_runtime_config_snapshot(
        &mut self,
        snapshot: RuntimeConfigSnapshot,
    ) -> Result<()> {
        let _lock = lock_config(&self.config_path)?;
        let mut stored = StoredConfig::load_unlocked(&self.config_path)?;
        self.ensure_stored_server_matches(&stored)?;
        stored.runtime_config_snapshot = Some(snapshot);
        stored.save_unlocked(&self.config_path)?;
        self.stored = stored;
        Ok(())
    }

    pub(crate) fn clear_runtime_config_snapshot(&mut self) -> Result<()> {
        let _lock = lock_config(&self.config_path)?;
        let mut stored = StoredConfig::load_unlocked(&self.config_path)?;
        self.ensure_stored_server_matches(&stored)?;
        stored.runtime_config_snapshot = None;
        stored.save_unlocked(&self.config_path)?;
        self.stored = stored;
        Ok(())
    }

    pub(crate) fn set_server(&mut self, server: &str) -> Result<()> {
        let api_base = normalize_api_base(server)?;
        let _lock = lock_config(&self.config_path)?;
        let mut stored = StoredConfig::load_unlocked(&self.config_path)?;
        let stored_server = stored
            .server
            .as_deref()
            .map(normalize_api_base)
            .transpose()?;
        if stored_server.as_deref() != Some(api_base.as_str()) {
            stored.clear_auth();
        }
        stored.server = Some(api_base.clone());
        stored.save_unlocked(&self.config_path)?;
        self.api_base = api_base;
        self.stored = stored;
        Ok(())
    }

    pub(crate) fn clear_auth(&mut self) -> Result<()> {
        let _lock = lock_config(&self.config_path)?;
        let mut stored = StoredConfig::load_unlocked(&self.config_path)?;
        if stored
            .server
            .as_deref()
            .map(normalize_api_base)
            .transpose()?
            .is_some_and(|server| server != self.api_base)
        {
            bail!("CLI 配置中的 Server 已被另一进程修改；拒绝清除其他 Server 的登录状态");
        }
        stored.clear_auth();
        stored.save_unlocked(&self.config_path)?;
        self.stored = stored;
        Ok(())
    }

    pub(crate) async fn login(&mut self, username: String, password: String) -> Result<AuthResult> {
        self.reject_environment_auth_mutation("登录")?;
        let secrets = [password.clone()];
        let value = self
            .send_json(routing::command_request(
                "login_user",
                json!({ "input": LoginInput { username, password } }),
            )?)
            .await
            .map_err(|error| redact_error_values(error, &secrets))?;
        let auth: AuthResult = serde_json::from_value(value).context("登录响应合同无效")?;
        self.store_tokens(&auth.tokens)?;
        Ok(auth)
    }

    pub(crate) async fn register(
        &mut self,
        username: String,
        email: String,
        invite_code: String,
        password: String,
    ) -> Result<AuthResult> {
        self.reject_environment_auth_mutation("注册")?;
        let secrets = [password.clone()];
        let value = self
            .send_json(routing::command_request(
                "register_user",
                json!({ "input": RegisterInput {
                    username,
                    email,
                    password,
                    invite_code,
                }}),
            )?)
            .await
            .map_err(|error| redact_error_values(error, &secrets))?;
        let auth: AuthResult = serde_json::from_value(value).context("注册响应合同无效")?;
        self.store_tokens(&auth.tokens)?;
        Ok(auth)
    }

    pub(crate) async fn current_user(&mut self) -> Result<UserSummary> {
        let value = self
            .send_json(routing::command_request("get_current_user", json!({}))?)
            .await?;
        serde_json::from_value(value).context("当前用户响应合同无效")
    }

    pub(crate) async fn logout(&mut self) -> Result<()> {
        self.reject_environment_auth_mutation("登出")?;
        let Some(session_id) = self.stored.session_id.clone() else {
            if self.access_token().is_some()
                || self
                    .stored
                    .refresh_token
                    .as_deref()
                    .is_some_and(|token| !token.trim().is_empty())
            {
                bail!(
                    "本地登录状态缺少 session ID，无法确认 Server 已登出；请使用 `a4n config clear-auth` 仅清理本地凭证"
                );
            }
            return self.clear_auth();
        };
        let result = self
            .send_json(routing::command_request(
                "revoke_session",
                json!({ "input": RevokeSessionInput { session_id } }),
            )?)
            .await;
        self.clear_auth()?;
        result.map(|_| ())
    }

    pub(crate) async fn refresh(&mut self) -> Result<TokenPairSummary> {
        self.refresh_access_token().await
    }

    pub(crate) async fn send_json(&mut self, request: JsonRequest) -> Result<Value> {
        self.send(request).await?.into_value()
    }

    pub(crate) async fn send_agent_json(
        &mut self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        let mut response = self
            .send_agent_once(method.clone(), path, body.clone())
            .await?;
        if response.status() == StatusCode::UNAUTHORIZED && self.can_refresh() {
            self.refresh_after_unauthorized().await?;
            response = self.send_agent_once(method, path, body).await?;
        }
        let secrets = self.auth_secrets();
        read_response(response)
            .await
            .map_err(|error| redact_error_values(error, &secrets))?
            .into_value()
    }

    pub(crate) async fn send_page_json(
        &mut self,
        method: Method,
        path: &str,
        body: Option<Value>,
        with_ui_session: bool,
        with_agent: bool,
    ) -> Result<Value> {
        let mut response = self
            .send_page_once(
                method.clone(),
                path,
                body.clone(),
                with_ui_session,
                with_agent,
            )
            .await?;
        if response.status() == StatusCode::UNAUTHORIZED && self.can_refresh() {
            self.refresh_after_unauthorized().await?;
            response = self
                .send_page_once(method, path, body, with_ui_session, with_agent)
                .await?;
        }
        let secrets = self.auth_secrets();
        read_response(response)
            .await
            .map_err(|error| redact_error_values(error, &secrets))?
            .into_value()
    }

    pub(crate) async fn send(&mut self, request: JsonRequest) -> Result<ApiBody> {
        let response = self
            .send_with_refresh(
                request.method,
                &request.path,
                request.body,
                request.authenticated,
            )
            .await?;
        let secrets = self.auth_secrets();
        read_response(response)
            .await
            .map_err(|error| redact_error_values(error, &secrets))
    }

    pub(crate) async fn send_response(&mut self, request: JsonRequest) -> Result<Response> {
        let response = self
            .send_with_refresh(
                request.method,
                &request.path,
                request.body,
                request.authenticated,
            )
            .await?;
        let secrets = self.auth_secrets();
        ensure_success_response(response)
            .await
            .map_err(|error| redact_error_values(error, &secrets))
    }

    pub(crate) async fn generation_stream(
        &mut self,
        path: &str,
        last_event_id: Option<&str>,
    ) -> Result<Response> {
        let mut response = self.send_stream_once(path, last_event_id).await?;
        if response.status() == StatusCode::UNAUTHORIZED && self.can_refresh() {
            self.refresh_after_unauthorized().await?;
            response = self.send_stream_once(path, last_event_id).await?;
        }
        let secrets = self.auth_secrets();
        ensure_success_response(response)
            .await
            .map_err(|error| redact_error_values(error, &secrets))
    }

    pub(crate) async fn agent_events_stream(
        &mut self,
        path: &str,
        after_sequence: Option<u64>,
    ) -> Result<Response> {
        let last_event_id = after_sequence.map(|value| value.to_string());
        self.generation_stream(path, last_event_id.as_deref()).await
    }

    pub(crate) async fn page_stream(&mut self, path: &str) -> Result<Response> {
        let mut response = self.send_page_stream_once(path).await?;
        if response.status() == StatusCode::UNAUTHORIZED && self.can_refresh() {
            self.refresh_after_unauthorized().await?;
            response = self.send_page_stream_once(path).await?;
        }
        let secrets = self.auth_secrets();
        ensure_success_response(response)
            .await
            .map_err(|error| redact_error_values(error, &secrets))
    }

    pub(crate) async fn download_media(&mut self, media_path: &str) -> Result<Response> {
        let mut response = self.send_media_once(media_path).await?;
        if response.status() == StatusCode::UNAUTHORIZED && self.can_refresh() {
            self.refresh_after_unauthorized().await?;
            response = self.send_media_once(media_path).await?;
        }
        let secrets = self.auth_secrets();
        ensure_success_response(response)
            .await
            .map_err(|error| redact_error_values(error, &secrets))
    }

    async fn send_with_refresh(
        &mut self,
        method: Method,
        path: &str,
        body: Option<Value>,
        authenticated: bool,
    ) -> Result<Response> {
        let mut response = self
            .send_once(method.clone(), path, body.clone(), authenticated)
            .await?;
        if response.status() == StatusCode::UNAUTHORIZED && authenticated && self.can_refresh() {
            self.refresh_after_unauthorized().await?;
            response = self.send_once(method, path, body, true).await?;
        }
        Ok(response)
    }

    async fn send_once(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        authenticated: bool,
    ) -> Result<Response> {
        let url = self.url(path)?;
        let mut request = self.http.request(method, url);
        if authenticated {
            let token = self
                .access_token()
                .ok_or_else(|| anyhow!("未登录；请先执行 `a4n auth login --password-stdin`"))?;
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        request.send().await.map_err(transport_error)
    }

    async fn send_agent_once(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Response> {
        let access_token = self
            .access_token()
            .ok_or_else(|| anyhow!("未登录；请先执行 `a4n auth login --password-stdin`"))?;
        let agent_token = self
            .agent_token()
            .ok_or_else(|| anyhow!("没有 Agent 凭证；请先执行 `a4n agent register`"))?;
        let mut request = self
            .http
            .request(method, self.url(path)?)
            .bearer_auth(access_token)
            .header("x-agent-token", agent_token);
        if let Some(body) = body {
            request = request.json(&body);
        }
        request.send().await.map_err(transport_error)
    }

    async fn send_page_once(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        with_ui_session: bool,
        with_agent: bool,
    ) -> Result<Response> {
        let access_token = self
            .access_token()
            .ok_or_else(|| anyhow!("未登录；请先执行 `a4n auth login --password-stdin`"))?;
        let mut request = self
            .http
            .request(method, self.url(path)?)
            .bearer_auth(access_token);
        if with_ui_session {
            request = request.header(
                "x-ui-session-token",
                self.ui_session_token().ok_or_else(|| {
                    anyhow!("没有 Page Session；请先执行 `a4n page attach --code CODE`")
                })?,
            );
        }
        if with_agent {
            request = request
                .header(
                    "x-agent-id",
                    self.agent_id()
                        .ok_or_else(|| anyhow!("没有 Agent；请先执行 `a4n agent register`"))?,
                )
                .header(
                    "x-agent-token",
                    self.agent_token().ok_or_else(|| {
                        anyhow!("没有 Agent token；请先执行 `a4n agent register`")
                    })?,
                );
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        request.send().await.map_err(transport_error)
    }

    async fn send_stream_once(&self, path: &str, last_event_id: Option<&str>) -> Result<Response> {
        let token = self
            .access_token()
            .ok_or_else(|| anyhow!("未登录；请先执行 `a4n auth login --password-stdin`"))?;
        let mut request = self
            .http
            .get(self.url(path)?)
            .header(ACCEPT, "text/event-stream")
            .bearer_auth(token);
        if let Some(last_event_id) = last_event_id {
            request = request.header(
                "last-event-id",
                HeaderValue::from_str(last_event_id).context("Last-Event-ID 无效")?,
            );
        }
        request.send().await.map_err(transport_error)
    }

    async fn send_page_stream_once(&self, path: &str) -> Result<Response> {
        let token = self
            .access_token()
            .ok_or_else(|| anyhow!("未登录；请先执行 `a4n auth login --password-stdin`"))?;
        self.http
            .get(self.url(path)?)
            .header(ACCEPT, "text/event-stream")
            .bearer_auth(token)
            .send()
            .await
            .map_err(transport_error)
    }

    async fn send_media_once(&self, media_path: &str) -> Result<Response> {
        let token = self
            .access_token()
            .ok_or_else(|| anyhow!("未登录；请先执行 `a4n auth login --password-stdin`"))?;
        let query = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("path", media_path)
            .finish();
        self.http
            .get(self.url(&format!("/media?{query}"))?)
            .bearer_auth(token)
            .send()
            .await
            .map_err(transport_error)
    }

    async fn refresh_access_token(&mut self) -> Result<TokenPairSummary> {
        self.reject_environment_auth_mutation("刷新登录会话")?;
        let _lock = lock_config(&self.config_path)?;
        self.reload_config_locked()?;
        self.refresh_access_token_locked().await
    }

    async fn refresh_after_unauthorized(&mut self) -> Result<()> {
        let previous_auth = self.auth_fingerprint();
        let _lock = lock_config(&self.config_path)?;
        self.reload_config_locked()?;
        if self.auth_fingerprint() != previous_auth {
            if self.access_token().is_some() {
                return Ok(());
            }
            bail!("登录状态已被另一 CLI 进程清除；请重新登录");
        }
        self.refresh_access_token_locked().await.map(|_| ())
    }

    async fn refresh_access_token_locked(&mut self) -> Result<TokenPairSummary> {
        let refresh_token = self
            .stored
            .refresh_token
            .clone()
            .ok_or_else(|| anyhow!("登录已过期且没有 refresh token；请重新登录"))?;
        let session_id = self
            .stored
            .session_id
            .clone()
            .ok_or_else(|| anyhow!("登录已过期且没有 session ID；请重新登录"))?;
        let secrets = [refresh_token.clone()];
        let request = routing::command_request(
            "refresh_token",
            json!({ "input": RefreshTokenInput {
                refresh_token,
                session_id,
            }}),
        )?;
        let response = self
            .send_once(
                request.method,
                &request.path,
                request.body,
                request.authenticated,
            )
            .await?;
        let value = read_response(response)
            .await
            .map_err(|error| redact_error_values(error, &secrets))?
            .into_value()?;
        let tokens: TokenPairSummary =
            serde_json::from_value(value).context("刷新 token 响应合同无效")?;
        self.store_tokens_locked(&tokens)?;
        Ok(tokens)
    }

    pub(super) fn reload_config_locked(&mut self) -> Result<()> {
        let stored = StoredConfig::load_unlocked(&self.config_path)?;
        match stored
            .server
            .as_deref()
            .map(normalize_api_base)
            .transpose()?
        {
            Some(server) if server != self.api_base => {
                bail!("CLI 配置中的 Server 已被另一进程修改；请重试当前命令");
            }
            None if stored.access_token.is_some()
                || stored.refresh_token.is_some()
                || stored.session_id.is_some() =>
            {
                bail!("CLI 配置中的登录状态缺少 Server 归属；请先执行 `a4n config clear-auth`");
            }
            _ => {}
        }
        self.stored = stored;
        Ok(())
    }

    fn store_tokens(&mut self, tokens: &TokenPairSummary) -> Result<()> {
        let _lock = lock_config(&self.config_path)?;
        let current = StoredConfig::load_unlocked(&self.config_path)?;
        if current
            .server
            .as_deref()
            .map(normalize_api_base)
            .transpose()?
            .is_some_and(|server| server != self.api_base)
        {
            bail!("CLI 配置中的 Server 已被另一进程修改；拒绝覆盖其登录状态");
        }
        let stored = self.token_config(tokens);
        stored.save_unlocked(&self.config_path)?;
        self.stored = stored;
        Ok(())
    }

    fn store_tokens_locked(&mut self, tokens: &TokenPairSummary) -> Result<()> {
        let stored = self.token_config(tokens);
        stored.save_unlocked(&self.config_path)?;
        self.stored = stored;
        Ok(())
    }

    fn token_config(&self, tokens: &TokenPairSummary) -> StoredConfig {
        StoredConfig {
            server: Some(self.api_base.clone()),
            access_token: Some(tokens.access_token.clone()),
            refresh_token: Some(tokens.refresh_token.clone()),
            session_id: Some(tokens.session_id.clone()),
            agent_id: self.stored.agent_id.clone(),
            agent_token: self.stored.agent_token.clone(),
            ui_session_id: self.stored.ui_session_id.clone(),
            ui_session_token: self.stored.ui_session_token.clone(),
            runtime_config_snapshot: self.stored.runtime_config_snapshot.clone(),
            pending_oauth: None,
        }
    }

    fn ensure_stored_server_matches(&self, stored: &StoredConfig) -> Result<()> {
        if stored
            .server
            .as_deref()
            .map(normalize_api_base)
            .transpose()?
            .is_some_and(|server| server != self.api_base)
        {
            bail!("CLI 配置中的 Server 已被另一进程修改；请重试当前命令");
        }
        Ok(())
    }

    fn auth_fingerprint(&self) -> (Option<String>, Option<String>, Option<String>) {
        (
            self.stored.access_token.clone(),
            self.stored.refresh_token.clone(),
            self.stored.session_id.clone(),
        )
    }

    fn auth_secrets(&self) -> Vec<String> {
        [
            self.access_token().map(str::to_string),
            self.stored.refresh_token.clone(),
            self.stored.agent_token.clone(),
            self.stored.ui_session_token.clone(),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn access_token(&self) -> Option<&str> {
        self.access_token_override
            .as_deref()
            .or(self.stored.access_token.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    fn can_refresh(&self) -> bool {
        self.access_token_override.is_none()
            && self
                .stored
                .refresh_token
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            && self
                .stored
                .session_id
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }

    fn reject_environment_auth_mutation(&self, action: &str) -> Result<()> {
        if self.access_token_override.is_some() {
            bail!("A4N_ACCESS_TOKEN 是只读环境认证；{action}前请先取消该环境变量");
        }
        Ok(())
    }

    pub(super) fn url(&self, path: &str) -> Result<String> {
        if !path.starts_with('/') || path.starts_with("//") {
            bail!("API path 必须是以单个 / 开头的相对路径");
        }
        let base = url::Url::parse(&self.api_base).context("API base URL 无效")?;
        let url =
            url::Url::parse(&format!("{}{path}", self.api_base)).context("API request URL 无效")?;
        let base_prefix = format!("{}/", base.path().trim_end_matches('/'));
        if url.origin() != base.origin()
            || !url.path().starts_with(&base_prefix)
            || url.fragment().is_some()
        {
            bail!("API path 不能逃出 Server API 根路径或包含 fragment");
        }
        Ok(url.into())
    }
}

#[derive(Clone)]
pub(crate) struct AgentHeartbeatClient {
    http: reqwest::Client,
    api_base: String,
    agent_id: String,
    access_token: String,
    agent_token: String,
}

impl AgentHeartbeatClient {
    pub(crate) async fn update(
        &self,
        status: &str,
        phase: &str,
        summary: Option<&str>,
    ) -> Result<()> {
        let encoded_id: String =
            url::form_urlencoded::byte_serialize(self.agent_id.as_bytes()).collect();
        let response = self
            .http
            .post(format!("{}/agents/{encoded_id}/heartbeat", self.api_base))
            .bearer_auth(&self.access_token)
            .header("x-agent-token", &self.agent_token)
            .json(&json!({
                "status": status,
                "phase": phase,
                "summary": summary,
            }))
            .send()
            .await
            .map_err(transport_error)?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("自动 Agent 心跳失败（HTTP {}）", response.status()))
        }
    }
}
