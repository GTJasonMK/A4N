use std::path::{Path, PathBuf};

use a4n_api_contract::auth::{
    AuthResult, LoginInput, RefreshTokenInput, RegisterInput, RevokeSessionInput, TokenPairSummary,
    UserSummary,
};
use a4n_api_contract::error::ApiErrorResponse;
use a4n_api_contract::generation_tasks::HostErrorResponse;
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;
use anyhow::{Context, Result, anyhow, bail};
use reqwest::header::{ACCEPT, HeaderValue};
use reqwest::{Method, Response, StatusCode};
use serde_json::{Value, json};

use crate::config::{
    StoredConfig, lock_config, non_empty_env_string, normalize_api_base, resolve_api_base,
};
use crate::routing;

#[derive(Clone)]
pub(crate) struct JsonRequest {
    pub(crate) method: Method,
    pub(crate) path: String,
    pub(crate) body: Option<Value>,
    pub(crate) authenticated: bool,
}

pub(crate) struct ApiBody {
    pub(crate) content_type: Option<String>,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct ApiRequestError {
    pub(crate) status: StatusCode,
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) retryable: bool,
    pub(crate) trace_id: Option<String>,
    pub(crate) details: Value,
}

impl std::fmt::Display for ApiRequestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "HTTP {}: {}", self.status, self.message)
    }
}

impl std::error::Error for ApiRequestError {}

impl ApiRequestError {
    fn redact(&mut self, secrets: &[String]) {
        let secrets = sorted_secret_refs(secrets);
        self.code = redact_text(&self.code, &secrets);
        self.message = redact_text(&self.message, &secrets);
        if let Some(trace_id) = &mut self.trace_id {
            *trace_id = redact_text(trace_id, &secrets);
        }
        redact_value(&mut self.details, &secrets);
    }
}

pub(crate) fn redact_error_values(mut error: anyhow::Error, secrets: &[String]) -> anyhow::Error {
    if let Some(error) = error.downcast_mut::<ApiRequestError>() {
        error.redact(secrets);
    }
    error
}

pub(crate) fn transport_error(error: reqwest::Error) -> anyhow::Error {
    anyhow::Error::new(error.without_url())
}

impl ApiBody {
    pub(crate) fn into_value(self) -> Result<Value> {
        if self.bytes.is_empty() {
            return Ok(Value::Null);
        }
        if self
            .content_type
            .as_deref()
            .is_some_and(|value| value.contains("json"))
            || matches!(self.bytes.first(), Some(b'{' | b'[' | b'"'))
        {
            return serde_json::from_slice(&self.bytes).context("Server 返回了无效 JSON");
        }
        Ok(Value::String(
            String::from_utf8(self.bytes).context("Server 返回了非 UTF-8 文本")?,
        ))
    }
}

pub(crate) struct ApiClient {
    http: reqwest::Client,
    api_base: String,
    config_path: PathBuf,
    stored: StoredConfig,
    access_token_override: Option<String>,
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

    pub(crate) async fn oauth2_callback(
        &mut self,
        provider: String,
        code: String,
    ) -> Result<AuthResult> {
        self.reject_environment_auth_mutation("OAuth2 登录")?;
        let secrets = [code.clone()];
        let value = self
            .send_json(routing::command_request(
                "oauth2_callback",
                json!({ "input": { "provider": provider, "code": code } }),
            )?)
            .await
            .map_err(|error| redact_error_values(error, &secrets))?;
        let auth: AuthResult = serde_json::from_value(value).context("OAuth2 登录响应合同无效")?;
        self.store_tokens(&auth.tokens)?;
        Ok(auth)
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

    fn reload_config_locked(&mut self) -> Result<()> {
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

    fn url(&self, path: &str) -> Result<String> {
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

async fn read_response(response: Response) -> Result<ApiBody> {
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let bytes = response.bytes().await.map_err(transport_error)?.to_vec();
    if !status.is_success() {
        return Err(anyhow::Error::new(api_request_error(status, &bytes)));
    }
    Ok(ApiBody {
        content_type,
        bytes,
    })
}

async fn ensure_success_response(response: Response) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let bytes = response.bytes().await.map_err(transport_error)?;
    Err(anyhow::Error::new(api_request_error(status, &bytes)))
}

fn api_request_error(status: StatusCode, bytes: &[u8]) -> ApiRequestError {
    if let Ok(body) = serde_json::from_slice::<ApiErrorResponse>(bytes) {
        return validated_api_request_error(
            status,
            body.code,
            body.message,
            body.retryable,
            body.trace_id,
            Value::Object(body.details),
        );
    }
    if let Ok(body) = serde_json::from_slice::<HostErrorResponse>(bytes) {
        return validated_api_request_error(
            status,
            body.code.as_str().to_string(),
            body.message,
            body.retryable,
            body.trace_id,
            Value::Object(body.details),
        );
    }
    invalid_api_error_contract(status)
}

fn validated_api_request_error(
    status: StatusCode,
    code: String,
    message: String,
    retryable: bool,
    trace_id: Option<String>,
    details: Value,
) -> ApiRequestError {
    if code.trim().is_empty()
        || message.trim().is_empty()
        || trace_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
    {
        return invalid_api_error_contract(status);
    }
    ApiRequestError {
        status,
        code,
        message,
        retryable,
        trace_id,
        details,
    }
}

fn invalid_api_error_contract(status: StatusCode) -> ApiRequestError {
    ApiRequestError {
        status,
        code: "invalid_server_error_contract".to_string(),
        message: format!("Server 返回的 HTTP {status} 错误体不符合 A4N API 合同"),
        retryable: false,
        trace_id: None,
        details: json!({}),
    }
}

fn redact_text(value: &str, secrets: &[&str]) -> String {
    secrets.iter().fold(value.to_string(), |value, secret| {
        value.replace(secret, "[REDACTED]")
    })
}

pub(crate) fn contains_sensitive_marker(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "password",
        "api_key",
        "api-key",
        "token",
        "secret",
        "credential",
        "authorization",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

pub(crate) fn redact_json_values(value: &mut Value, secrets: &[String]) {
    redact_value(value, &sorted_secret_refs(secrets));
}

pub(crate) fn redact_text_values(value: &str, secrets: &[String]) -> String {
    redact_text(value, &sorted_secret_refs(secrets))
}

fn sorted_secret_refs(secrets: &[String]) -> Vec<&str> {
    let mut secrets = secrets
        .iter()
        .map(String::as_str)
        .filter(|secret| !secret.is_empty())
        .collect::<Vec<_>>();
    secrets
        .sort_unstable_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    secrets.dedup();
    secrets
}

pub(crate) fn is_sensitive_field_name(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase().replace('-', "_");
    let compact = normalized.replace('_', "");
    normalized.contains("password")
        || normalized.contains("passphrase")
        || normalized.contains("api_key")
        || compact.contains("apikey")
        || matches!(
            normalized.as_str(),
            "secret" | "credential" | "credentials" | "authorization"
        )
        || normalized.ends_with("_secret")
        || normalized.ends_with("_credential")
        || normalized.ends_with("_credentials")
        || matches!(
            compact.as_str(),
            "token"
                | "accesstoken"
                | "refreshtoken"
                | "mediatoken"
                | "authorizationtoken"
                | "privatekey"
        )
}

pub(crate) fn visit_sensitive_values(value: &Value, visitor: &mut impl FnMut(&str, &Value)) {
    visit_sensitive_values_at(value, None, visitor);
}

fn visit_sensitive_values_at(
    value: &Value,
    field: Option<&str>,
    visitor: &mut impl FnMut(&str, &Value),
) {
    match (field, value) {
        (Some(field), Value::Array(items)) if !items.is_empty() => {
            for item in items {
                visit_sensitive_values_at(item, Some(field), visitor);
            }
        }
        (Some(field), Value::Object(object)) if !object.is_empty() => {
            for value in object.values() {
                visit_sensitive_values_at(value, Some(field), visitor);
            }
        }
        (Some(field), _) => visitor(field, value),
        (None, Value::Array(items)) => {
            for item in items {
                visit_sensitive_values_at(item, None, visitor);
            }
        }
        (None, Value::Object(object)) => {
            for (key, value) in object {
                let field = is_sensitive_field_name(key).then_some(key.as_str());
                visit_sensitive_values_at(value, field, visitor);
            }
        }
        (None, _) => {}
    }
}

fn redact_value(value: &mut Value, secrets: &[&str]) {
    match value {
        Value::Array(items) => {
            for item in items {
                redact_value(item, secrets);
            }
        }
        Value::Object(object) => {
            let entries = std::mem::take(object);
            for (key, mut value) in entries {
                redact_value(&mut value, secrets);
                object.insert(redact_text(&key, secrets), value);
            }
        }
        Value::String(text) => *text = redact_text(text, secrets),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use reqwest::Method;
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use uuid::Uuid;

    use super::{
        ApiBody, ApiClient, JsonRequest, api_request_error, is_sensitive_field_name,
        redact_json_values,
    };
    use crate::config::StoredConfig;

    #[test]
    fn response_bodies_decode_json_text_empty_and_reject_invalid_bytes() {
        assert_eq!(
            ApiBody {
                content_type: Some("application/json".into()),
                bytes: br#"{"ok":true}"#.to_vec(),
            }
            .into_value()
            .unwrap(),
            json!({"ok": true})
        );
        assert_eq!(
            ApiBody {
                content_type: None,
                bytes: b"[1,2]".to_vec(),
            }
            .into_value()
            .unwrap(),
            json!([1, 2])
        );
        assert_eq!(
            ApiBody {
                content_type: Some("text/plain".into()),
                bytes: b"plain text".to_vec(),
            }
            .into_value()
            .unwrap(),
            json!("plain text")
        );
        assert_eq!(
            ApiBody {
                content_type: None,
                bytes: Vec::new(),
            }
            .into_value()
            .unwrap(),
            serde_json::Value::Null
        );
        assert!(
            ApiBody {
                content_type: Some("application/json".into()),
                bytes: b"not-json".to_vec(),
            }
            .into_value()
            .is_err()
        );
        assert!(
            ApiBody {
                content_type: None,
                bytes: vec![0xff],
            }
            .into_value()
            .is_err()
        );
    }

    #[test]
    fn extracts_api_error_contract() {
        let error = api_request_error(
            reqwest::StatusCode::CONFLICT,
            br#"{"code":"state_conflict","message":"failed","retryable":false,"trace_id":"trace-1","details":{"state":"busy"}}"#,
        );
        assert_eq!(error.code, "state_conflict");
        assert_eq!(error.message, "failed");
        assert_eq!(error.trace_id.as_deref(), Some("trace-1"));
        assert!(!error.retryable);
        assert_eq!(error.details["state"], "busy");
    }

    #[test]
    fn extracts_generation_api_error_contract() {
        let error = api_request_error(
            reqwest::StatusCode::NOT_FOUND,
            br#"{"code":"generation_task_not_found","message":"missing","retryable":false,"trace_id":null,"details":{"task_id":"missing"}}"#,
        );
        assert_eq!(error.code, "generation_task_not_found");
        assert_eq!(error.message, "missing");
        assert_eq!(error.details["task_id"], "missing");
    }

    #[test]
    fn rejects_noncanonical_api_error_contract_without_guessing_fields() {
        let error = api_request_error(
            reqwest::StatusCode::BAD_GATEWAY,
            br#"{"code":"upstream_error","message":"upstream unavailable","retryable":true,"details":{}}"#,
        );
        assert_eq!(error.code, "invalid_server_error_contract");
        assert!(!error.retryable);
        assert_eq!(error.trace_id, None);
        assert_eq!(error.details, json!({}));
    }

    #[test]
    fn redacts_known_secrets_from_api_errors() {
        let mut error = api_request_error(
            reqwest::StatusCode::BAD_REQUEST,
            br#"{"code":"invalid-top-secret","message":"rejected top-secret","retryable":false,"trace_id":"trace-top-secret","details":{"key-top-secret":["top-secret"]}}"#,
        );
        error.redact(&["top-secret".into()]);

        assert_eq!(error.code, "invalid-[REDACTED]");
        assert_eq!(error.message, "rejected [REDACTED]");
        assert_eq!(error.trace_id.as_deref(), Some("trace-[REDACTED]"));
        assert_eq!(error.details["key-[REDACTED]"][0], "[REDACTED]");
    }

    #[test]
    fn redacts_overlapping_secrets_longest_first() {
        let mut error = api_request_error(
            reqwest::StatusCode::BAD_REQUEST,
            br#"{"code":"invalid_request","message":"rejected token-extended","retryable":false,"trace_id":null,"details":{}}"#,
        );
        error.redact(&["token".into(), "token-extended".into()]);

        assert_eq!(error.message, "rejected [REDACTED]");
    }

    #[test]
    fn sensitive_field_detection_does_not_hide_token_metrics() {
        assert!(is_sensitive_field_name("access_token"));
        assert!(is_sensitive_field_name("apiKey"));
        assert!(!is_sensitive_field_name("token_count"));
        assert!(!is_sensitive_field_name("max_tokens"));
    }

    #[test]
    fn successful_json_responses_are_redacted_with_request_secrets() {
        let mut value = json!({"echo": "token-extended", "nested": ["token"]});
        redact_json_values(&mut value, &["token".into(), "token-extended".into()]);
        assert_eq!(
            value,
            json!({"echo": "[REDACTED]", "nested": ["[REDACTED]"]})
        );
    }

    #[test]
    fn failed_config_writes_do_not_mutate_client_state() {
        let directory = tempfile::TempDir::new().unwrap();
        let blocked_parent = directory.path().join("not-a-directory");
        std::fs::write(&blocked_parent, b"blocked").unwrap();
        let config_path = blocked_parent.join("cli.json");
        let stored = StoredConfig {
            server: Some("http://127.0.0.1:8080".into()),
            access_token: Some("access".into()),
            refresh_token: Some("refresh".into()),
            session_id: Some("session".into()),
            agent_id: None,
            agent_token: None,
            ui_session_id: None,
            ui_session_token: None,
            runtime_config_snapshot: None,
        };
        let mut client = ApiClient::new(config_path, stored, None).unwrap();

        assert!(client.set_server("http://127.0.0.1:18080").is_err());
        assert_eq!(client.api_base, "http://127.0.0.1:8080/api");
        assert_eq!(client.stored.access_token.as_deref(), Some("access"));

        assert!(client.clear_auth().is_err());
        assert_eq!(client.stored.access_token.as_deref(), Some("access"));
        assert_eq!(client.stored.refresh_token.as_deref(), Some("refresh"));
        assert_eq!(client.stored.session_id.as_deref(), Some("session"));
    }

    #[test]
    fn api_paths_cannot_escape_the_configured_api_root() {
        let client = ApiClient::new(
            test_config_path(),
            StoredConfig::default(),
            Some("http://127.0.0.1:8080/a4n"),
        )
        .unwrap();

        assert_eq!(
            client.url("/meta?detail=1").unwrap(),
            "http://127.0.0.1:8080/a4n/api/meta?detail=1"
        );
        assert!(client.url("/../admin").is_err());
        assert!(client.url("/%2e%2e/admin").is_err());
        assert!(client.url("/meta#credentials").is_err());
    }

    #[test]
    fn stale_clients_do_not_restore_cleared_tokens() {
        let config_path = test_config_path();
        let stored = StoredConfig {
            server: Some("http://127.0.0.1:8080".into()),
            access_token: Some("access".into()),
            refresh_token: Some("refresh".into()),
            session_id: Some("session".into()),
            agent_id: None,
            agent_token: None,
            ui_session_id: None,
            ui_session_token: None,
            runtime_config_snapshot: None,
        };
        stored.save(&config_path).unwrap();
        let mut stale = ApiClient::new(config_path.clone(), stored.clone(), None).unwrap();
        let mut current = ApiClient::new(config_path.clone(), stored, None).unwrap();

        current.clear_auth().unwrap();
        stale.set_server("http://127.0.0.1:8080").unwrap();

        let saved = StoredConfig::load(&config_path).unwrap();
        assert!(saved.access_token.is_none());
        assert!(saved.refresh_token.is_none());
        assert!(saved.session_id.is_none());
        remove_test_config(&config_path);
    }

    #[test]
    fn reload_rejects_tokens_without_a_server_owner() {
        let config_path = test_config_path();
        let stored = StoredConfig {
            server: None,
            access_token: Some("access".into()),
            refresh_token: Some("refresh".into()),
            session_id: Some("session".into()),
            agent_id: None,
            agent_token: None,
            ui_session_id: None,
            ui_session_token: None,
            runtime_config_snapshot: None,
        };
        stored.save(&config_path).unwrap();
        let mut client =
            ApiClient::new(config_path.clone(), stored, Some("http://127.0.0.1:8080")).unwrap();

        assert!(!client.is_authenticated());
        assert!(client.reload_config_locked().is_err());
        assert!(!client.is_authenticated());
        remove_test_config(&config_path);
    }

    #[tokio::test]
    async fn refreshes_once_retries_and_persists_tokens() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server = format!("http://{}", listener.local_addr().unwrap());
        let server_task = tokio::spawn(async move {
            let (mut stream, first) = accept_request(&listener).await;
            assert!(first.starts_with("GET /api/protected "));
            assert!(
                first
                    .to_ascii_lowercase()
                    .contains("authorization: bearer old")
            );
            write_response(&mut stream, "401 Unauthorized", r#"{"message":"expired"}"#).await;

            let (mut stream, refresh) = accept_request(&listener).await;
            assert!(refresh.starts_with("POST /api/commands/refresh_token "));
            assert!(refresh.contains(r#""refresh_token":"refresh""#));
            write_response(
                &mut stream,
                "200 OK",
                r#"{"access_token":"new","refresh_token":"next","expires_in":3600,"token_type":"Bearer","session_id":"session"}"#,
            )
            .await;

            let (mut stream, retried) = accept_request(&listener).await;
            assert!(retried.starts_with("GET /api/protected "));
            assert!(
                retried
                    .to_ascii_lowercase()
                    .contains("authorization: bearer new")
            );
            write_response(&mut stream, "200 OK", r#"{"ok":true}"#).await;
        });

        let config_path = test_config_path();
        let stored = StoredConfig {
            server: Some(server),
            access_token: Some("old".into()),
            refresh_token: Some("refresh".into()),
            session_id: Some("session".into()),
            agent_id: None,
            agent_token: None,
            ui_session_id: None,
            ui_session_token: None,
            runtime_config_snapshot: None,
        };
        stored.save(&config_path).unwrap();
        let mut client = ApiClient::new(config_path.clone(), stored, None).unwrap();
        let result = client
            .send_json(JsonRequest {
                method: Method::GET,
                path: "/protected".into(),
                body: None,
                authenticated: true,
            })
            .await
            .unwrap();
        assert_eq!(result, json!({ "ok": true }));
        server_task.await.unwrap();

        let saved = StoredConfig::load(&config_path).unwrap();
        assert_eq!(saved.access_token.as_deref(), Some("new"));
        assert_eq!(saved.refresh_token.as_deref(), Some("next"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&config_path)
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        remove_test_config(&config_path);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_clients_reuse_the_single_rotated_token_pair() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server = format!("http://{}", listener.local_addr().unwrap());
        let server_task = tokio::spawn(async move {
            let (mut first_stream, first) = accept_request(&listener).await;
            let (mut second_stream, second) = accept_request(&listener).await;
            for request in [first, second] {
                assert!(request.starts_with("GET /api/protected "));
                assert!(
                    request
                        .to_ascii_lowercase()
                        .contains("authorization: bearer old")
                );
            }
            tokio::join!(
                write_response(
                    &mut first_stream,
                    "401 Unauthorized",
                    r#"{"message":"expired"}"#,
                ),
                write_response(
                    &mut second_stream,
                    "401 Unauthorized",
                    r#"{"message":"expired"}"#,
                )
            );

            let (mut refresh_stream, refresh) = accept_request(&listener).await;
            assert!(refresh.starts_with("POST /api/commands/refresh_token "));
            assert!(refresh.contains(r#""refresh_token":"refresh""#));
            write_response(
                &mut refresh_stream,
                "200 OK",
                r#"{"access_token":"new","refresh_token":"next","expires_in":3600,"token_type":"Bearer","session_id":"session"}"#,
            )
            .await;

            for _ in 0..2 {
                let (mut stream, retried) = accept_request(&listener).await;
                assert!(retried.starts_with("GET /api/protected "));
                assert!(
                    retried
                        .to_ascii_lowercase()
                        .contains("authorization: bearer new")
                );
                write_response(&mut stream, "200 OK", r#"{"ok":true}"#).await;
            }
        });

        let config_path = test_config_path();
        let stored = StoredConfig {
            server: Some(server),
            access_token: Some("old".into()),
            refresh_token: Some("refresh".into()),
            session_id: Some("session".into()),
            agent_id: None,
            agent_token: None,
            ui_session_id: None,
            ui_session_token: None,
            runtime_config_snapshot: None,
        };
        stored.save(&config_path).unwrap();
        let mut first_client = ApiClient::new(
            config_path.clone(),
            StoredConfig::load(&config_path).unwrap(),
            None,
        )
        .unwrap();
        let mut second_client = ApiClient::new(
            config_path.clone(),
            StoredConfig::load(&config_path).unwrap(),
            None,
        )
        .unwrap();
        let request = JsonRequest {
            method: Method::GET,
            path: "/protected".into(),
            body: None,
            authenticated: true,
        };
        let second_request = request.clone();
        let first_task = tokio::spawn(async move { first_client.send_json(request).await });
        let second_task =
            tokio::spawn(async move { second_client.send_json(second_request).await });
        let (first_result, second_result) = tokio::join!(first_task, second_task);
        assert_eq!(first_result.unwrap().unwrap(), json!({ "ok": true }));
        assert_eq!(second_result.unwrap().unwrap(), json!({ "ok": true }));
        server_task.await.unwrap();

        let saved = StoredConfig::load(&config_path).unwrap();
        assert_eq!(saved.access_token.as_deref(), Some("new"));
        assert_eq!(saved.refresh_token.as_deref(), Some("next"));
        remove_test_config(&config_path);
    }

    #[tokio::test]
    async fn logout_clears_local_tokens_even_when_server_rejects_it() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server = format!("http://{}", listener.local_addr().unwrap());
        let server_task = tokio::spawn(async move {
            let (mut stream, request) = accept_request(&listener).await;
            assert!(request.starts_with("POST /api/commands/revoke_session "));
            write_response(
                &mut stream,
                "500 Internal Server Error",
                r#"{"message":"failed"}"#,
            )
            .await;
        });

        let config_path = test_config_path();
        let stored = StoredConfig {
            server: Some(server),
            access_token: Some("old".into()),
            refresh_token: Some("refresh".into()),
            session_id: Some("session".into()),
            agent_id: None,
            agent_token: None,
            ui_session_id: None,
            ui_session_token: None,
            runtime_config_snapshot: None,
        };
        stored.save(&config_path).unwrap();
        let mut client = ApiClient::new(config_path.clone(), stored, None).unwrap();
        assert!(client.logout().await.is_err());
        server_task.await.unwrap();
        let saved = StoredConfig::load(&config_path).unwrap();
        assert!(saved.access_token.is_none());
        assert!(saved.refresh_token.is_none());
        assert!(saved.session_id.is_none());
        remove_test_config(&config_path);
    }

    #[tokio::test]
    async fn environment_access_tokens_cannot_report_a_fake_logout() {
        let config_path = test_config_path();
        let stored = StoredConfig {
            server: Some("http://127.0.0.1:8080".into()),
            access_token: Some("stored-access".into()),
            refresh_token: Some("stored-refresh".into()),
            session_id: Some("stored-session".into()),
            agent_id: None,
            agent_token: None,
            ui_session_id: None,
            ui_session_token: None,
            runtime_config_snapshot: None,
        };
        stored.save(&config_path).unwrap();
        let mut client = ApiClient::new(config_path.clone(), stored, None).unwrap();
        client.access_token_override = Some("environment-access".into());

        assert!(client.logout().await.is_err());
        assert!(client.session_id().is_none());
        client.clear_auth().unwrap();
        assert!(client.is_authenticated());
        assert_eq!(client.auth_source(), Some("environment"));

        let saved = StoredConfig::load(&config_path).unwrap();
        assert!(saved.access_token.is_none());
        assert!(saved.refresh_token.is_none());
        assert!(saved.session_id.is_none());
        remove_test_config(&config_path);
    }

    fn test_config_path() -> PathBuf {
        std::env::temp_dir().join(format!("a4n-cli-test-{}.json", Uuid::new_v4()))
    }

    fn remove_test_config(path: &std::path::Path) {
        std::fs::remove_file(path).unwrap();
        let mut lock_path = path.as_os_str().to_os_string();
        lock_path.push(".lock");
        std::fs::remove_file(lock_path).unwrap();
    }

    async fn accept_request(listener: &TcpListener) -> (TcpStream, String) {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_request(&mut stream).await;
        (stream, request)
    }

    async fn read_request(stream: &mut TcpStream) -> String {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let read = stream.read(&mut buffer).await.unwrap();
            assert!(read > 0, "HTTP request ended before headers");
            bytes.extend_from_slice(&buffer[..read]);
            let Some(header_end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") else {
                continue;
            };
            let header_end = header_end + 4;
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            while bytes.len() < header_end + content_length {
                let read = stream.read(&mut buffer).await.unwrap();
                assert!(read > 0, "HTTP request ended before body");
                bytes.extend_from_slice(&buffer[..read]);
            }
            return String::from_utf8(bytes).unwrap();
        }
    }

    async fn write_response(stream: &mut TcpStream, status: &str, body: &str) {
        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    }
}
