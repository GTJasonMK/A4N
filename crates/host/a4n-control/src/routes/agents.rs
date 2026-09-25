use super::auth::authenticated_user;
use super::error::ApiError;
use super::validate_positive_id;
use crate::state::ServerState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::{Stream, stream};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const AGENT_LEASE_SECONDS: i64 = 30;

#[derive(Clone)]
pub(crate) struct AgentRegistry {
    records: Arc<RwLock<HashMap<String, AgentRecord>>>,
    events: tokio::sync::broadcast::Sender<AgentEvent>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct AgentView {
    pub agent_id: String,
    pub name: String,
    pub kind: String,
    pub client: String,
    pub status: String,
    pub phase: Option<String>,
    pub summary: Option<String>,
    pub progress: Option<f32>,
    pub current_task_id: Option<String>,
    pub project_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub started_at: String,
    pub updated_at: String,
    pub last_heartbeat_at: String,
    pub expires_at: String,
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize)]
struct AgentEvent {
    kind: &'static str,
    agent_id: String,
    agent: Option<AgentView>,
    #[serde(skip)]
    owner_user_id: i64,
}

#[derive(Clone, Debug)]
struct AgentRecord {
    view: AgentView,
    owner_user_id: i64,
    token: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterAgentInput {
    name: String,
    #[serde(default = "default_agent_kind")]
    kind: String,
    #[serde(default = "default_agent_client")]
    client: String,
    project_id: Option<i64>,
    chapter_id: Option<i64>,
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateAgentInput {
    status: Option<String>,
    phase: Option<String>,
    summary: Option<String>,
    progress: Option<f32>,
    current_task_id: Option<String>,
    project_id: Option<i64>,
    chapter_id: Option<i64>,
    metadata: Option<Value>,
}

#[derive(Debug, Serialize)]
pub(super) struct RegisterAgentResponse {
    agent: AgentView,
    agent_token: String,
    lease_seconds: i64,
}

impl AgentRegistry {
    pub(crate) fn new() -> Self {
        let (events, _) = tokio::sync::broadcast::channel(128);
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            events,
        }
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<AgentEvent> {
        self.events.subscribe()
    }

    fn expire(&self) {
        let now = chrono::Utc::now().to_rfc3339();
        let mut expired = Vec::new();
        {
            let mut records = self.records.write();
            for record in records.values_mut() {
                if record.view.status != "offline" && record.view.expires_at <= now {
                    record.view.status = "offline".into();
                    record.view.updated_at = now.clone();
                    expired.push(AgentEvent {
                        kind: "updated",
                        agent_id: record.view.agent_id.clone(),
                        agent: Some(record.view.clone()),
                        owner_user_id: record.owner_user_id,
                    });
                }
            }
        }
        for event in expired {
            let _ = self.events.send(event);
        }
    }

    fn list(&self, owner_user_id: i64) -> Vec<AgentView> {
        self.expire();
        let mut agents: Vec<_> = self
            .records
            .read()
            .values()
            .filter(|record| record.owner_user_id == owner_user_id)
            .map(|record| record.view.clone())
            .collect();
        agents.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        agents
    }

    pub(crate) fn authorize(
        &self,
        owner_user_id: i64,
        agent_id: &str,
        token: &str,
    ) -> Result<(), ApiError> {
        self.expire();
        let records = self.records.read();
        let record = records
            .get(agent_id)
            .ok_or_else(|| ApiError::not_found("Agent 不存在"))?;
        authorize_record(record, owner_user_id, token)
    }

    fn register(
        &self,
        owner_user_id: i64,
        input: RegisterAgentInput,
    ) -> Result<RegisterAgentResponse, ApiError> {
        let name = required_text(input.name, "name")?;
        let kind = required_text(input.kind, "kind")?;
        let client = required_text(input.client, "client")?;
        validate_positive_id(input.project_id, "project_id")?;
        validate_positive_id(input.chapter_id, "chapter_id")?;
        if let Some(metadata) = &input.metadata {
            validate_metadata(metadata)?;
        }
        let now = chrono::Utc::now().to_rfc3339();
        let agent_id = Uuid::new_v4().to_string();
        let token = Uuid::new_v4().to_string();
        let view = AgentView {
            agent_id: agent_id.clone(),
            name,
            kind,
            client,
            status: "idle".into(),
            phase: None,
            summary: None,
            progress: None,
            current_task_id: None,
            project_id: input.project_id,
            chapter_id: input.chapter_id,
            started_at: now.clone(),
            updated_at: now.clone(),
            last_heartbeat_at: now.clone(),
            expires_at: lease_expiry(),
            metadata: input.metadata.unwrap_or(Value::Null),
        };
        let event = AgentEvent {
            kind: "registered",
            agent_id: agent_id.clone(),
            agent: Some(view.clone()),
            owner_user_id,
        };
        self.records.write().insert(
            agent_id,
            AgentRecord {
                view: view.clone(),
                owner_user_id,
                token: token.clone(),
            },
        );
        let _ = self.events.send(event);
        Ok(RegisterAgentResponse {
            agent: view,
            agent_token: token,
            lease_seconds: AGENT_LEASE_SECONDS,
        })
    }

    #[cfg(test)]
    pub(crate) fn register_for_test(&self, owner_user_id: i64, name: &str) -> (String, String) {
        let response = self
            .register(
                owner_user_id,
                RegisterAgentInput {
                    name: name.into(),
                    kind: "test".into(),
                    client: "test".into(),
                    project_id: None,
                    chapter_id: None,
                    metadata: None,
                },
            )
            .expect("test agent registration");
        (response.agent.agent_id, response.agent_token)
    }

    fn update(
        &self,
        owner_user_id: i64,
        agent_id: &str,
        token: &str,
        input: UpdateAgentInput,
    ) -> Result<AgentView, ApiError> {
        self.expire();
        let status_omitted = input.status.is_none();
        let view = {
            let mut records = self.records.write();
            let record = records
                .get_mut(agent_id)
                .ok_or_else(|| ApiError::not_found("Agent 不存在"))?;
            authorize_record(record, owner_user_id, token)?;
            let mut view = record.view.clone();
            apply_update(&mut view, input)?;
            if status_omitted && view.status == "offline" {
                view.status = "idle".into();
            }
            view.updated_at = chrono::Utc::now().to_rfc3339();
            view.last_heartbeat_at = view.updated_at.clone();
            view.expires_at = lease_expiry();
            record.view = view.clone();
            view
        };
        let _ = self.events.send(AgentEvent {
            kind: "updated",
            agent_id: agent_id.to_string(),
            agent: Some(view.clone()),
            owner_user_id,
        });
        Ok(view)
    }

    fn unregister(&self, owner_user_id: i64, agent_id: &str, token: &str) -> Result<(), ApiError> {
        let removed = {
            let mut records = self.records.write();
            let record = records
                .get(agent_id)
                .ok_or_else(|| ApiError::not_found("Agent 不存在"))?;
            authorize_record(record, owner_user_id, token)?;
            records.remove(agent_id).expect("agent checked above").view
        };
        let _ = self.events.send(AgentEvent {
            kind: "removed",
            agent_id: agent_id.to_string(),
            agent: Some(removed),
            owner_user_id,
        });
        Ok(())
    }
}

pub(super) async fn list(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AgentView>>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.agents().list(user.id)))
}

pub(super) async fn register(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<RegisterAgentInput>,
) -> Result<(StatusCode, Json<RegisterAgentResponse>), ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok((
        StatusCode::CREATED,
        Json(state.agents().register(user.id, input)?),
    ))
}

pub(super) async fn update(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
    Json(input): Json<UpdateAgentInput>,
) -> Result<Json<AgentView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let token = agent_token(&headers)?;
    Ok(Json(
        state.agents().update(user.id, &agent_id, token, input)?,
    ))
}

pub(super) async fn unregister(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    state
        .agents()
        .unregister(user.id, &agent_id, agent_token(&headers)?)?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn events(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let registry = state.agents().clone();
    let stream = agent_sse_stream(registry, user.id);
    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keep-alive"),
    ))
}

fn agent_sse_stream(
    registry: AgentRegistry,
    owner_user_id: i64,
) -> impl Stream<Item = Result<Event, Infallible>> + Send + 'static {
    let receiver = registry.subscribe();
    let interval = tokio::time::interval(Duration::from_secs(5));
    stream::unfold(
        (registry, receiver, interval),
        move |(registry, mut receiver, mut interval)| async move {
            loop {
                tokio::select! {
                    result = receiver.recv() => match result {
                        Ok(event) if event.owner_user_id == owner_user_id => {
                            let data = serde_json::to_string(&event).expect("agent event is serializable");
                            return Some((Ok(Event::default().event("agent_update").data(data)), (registry, receiver, interval)));
                        }
                        Ok(_) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
                    },
                    _ = interval.tick() => registry.expire(),
                }
            }
        },
    )
}

fn apply_update(view: &mut AgentView, input: UpdateAgentInput) -> Result<(), ApiError> {
    if let Some(status) = input.status {
        if ![
            "idle",
            "running",
            "paused",
            "completed",
            "failed",
            "offline",
        ]
        .contains(&status.as_str())
        {
            return Err(ApiError::bad_request("Agent status 无效"));
        }
        view.status = status;
    }
    if let Some(progress) = input.progress {
        if !progress.is_finite() || !(0.0..=100.0).contains(&progress) {
            return Err(ApiError::bad_request("progress 必须在 0 到 100 之间"));
        }
        view.progress = Some(progress);
    }
    if let Some(value) = input.phase {
        view.phase = Some(required_text(value, "phase")?);
    }
    if let Some(value) = input.summary {
        view.summary = Some(required_text(value, "summary")?);
    }
    if let Some(value) = input.current_task_id {
        view.current_task_id = Some(required_text(value, "current_task_id")?);
    }
    if let Some(value) = input.project_id {
        validate_positive_id(Some(value), "project_id")?;
        view.project_id = Some(value);
    }
    if let Some(value) = input.chapter_id {
        validate_positive_id(Some(value), "chapter_id")?;
        view.chapter_id = Some(value);
    }
    if let Some(value) = input.metadata {
        validate_metadata(&value)?;
        view.metadata = value;
    }
    Ok(())
}

fn authorize_record(record: &AgentRecord, owner_user_id: i64, token: &str) -> Result<(), ApiError> {
    if record.owner_user_id != owner_user_id || record.token != token {
        return Err(ApiError::unauthorized("Agent token 无效"));
    }
    Ok(())
}

fn agent_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get("x-agent-token")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::unauthorized("缺少 X-Agent-Token"))
}

fn required_text(value: String, field: &str) -> Result<String, ApiError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(ApiError::bad_request(format!("{field} 不能为空")));
    }
    if value.chars().count() > 200 {
        return Err(ApiError::bad_request(format!("{field} 过长")));
    }
    Ok(value)
}

fn validate_metadata(value: &Value) -> Result<(), ApiError> {
    let bytes = serde_json::to_vec(value).map_err(|_| ApiError::bad_request("metadata 无效"))?;
    if bytes.len() > 64 * 1024 {
        return Err(ApiError::bad_request("metadata 不能超过 64 KiB"));
    }
    Ok(())
}

fn default_agent_kind() -> String {
    "general".into()
}
fn default_agent_client() -> String {
    "cli".into()
}
fn lease_expiry() -> String {
    (chrono::Utc::now() + chrono::Duration::seconds(AGENT_LEASE_SECONDS)).to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_updates_and_scopes_agents_by_user() {
        let registry = AgentRegistry::new();
        let registered = registry
            .register(
                7,
                RegisterAgentInput {
                    name: "漫画转换器".into(),
                    kind: "media".into(),
                    client: "cli".into(),
                    project_id: Some(42),
                    chapter_id: Some(3),
                    metadata: None,
                },
            )
            .expect("register agent");

        assert_eq!(registry.list(7).len(), 1);
        assert!(registry.list(8).is_empty());
        let updated = registry
            .update(
                7,
                &registered.agent.agent_id,
                &registered.agent_token,
                UpdateAgentInput {
                    status: Some("running".into()),
                    phase: Some("生成分镜".into()),
                    summary: Some("正在处理上一章".into()),
                    progress: Some(32.0),
                    current_task_id: None,
                    project_id: None,
                    chapter_id: None,
                    metadata: None,
                },
            )
            .expect("update agent");
        assert_eq!(updated.status, "running");
        assert_eq!(updated.progress, Some(32.0));
        assert!(
            registry
                .update(
                    8,
                    &registered.agent.agent_id,
                    &registered.agent_token,
                    UpdateAgentInput {
                        status: None,
                        phase: None,
                        summary: None,
                        progress: None,
                        current_task_id: None,
                        project_id: None,
                        chapter_id: None,
                        metadata: None,
                    },
                )
                .is_err()
        );
    }

    #[test]
    fn invalid_update_does_not_partially_change_agent() {
        let registry = AgentRegistry::new();
        let (agent_id, token) = registry.register_for_test(7, "test agent");
        let before = registry.list(7).pop().expect("registered agent");

        assert!(
            registry
                .update(
                    7,
                    &agent_id,
                    &token,
                    UpdateAgentInput {
                        status: Some("running".into()),
                        phase: None,
                        summary: None,
                        progress: Some(f32::NAN),
                        current_task_id: None,
                        project_id: None,
                        chapter_id: None,
                        metadata: None,
                    },
                )
                .is_err()
        );

        let after = registry.list(7).pop().expect("registered agent");
        assert_eq!(after.status, before.status);
        assert_eq!(after.updated_at, before.updated_at);
        assert_eq!(after.last_heartbeat_at, before.last_heartbeat_at);
        assert_eq!(after.expires_at, before.expires_at);
    }

    #[test]
    fn statusless_heartbeat_revives_expired_agent() {
        let registry = AgentRegistry::new();
        let (agent_id, token) = registry.register_for_test(7, "test agent");
        {
            let mut records = registry.records.write();
            let record = records.get_mut(&agent_id).expect("registered agent");
            record.view.status = "running".into();
            record.view.expires_at = "2000-01-01T00:00:00+00:00".into();
        }

        let updated = registry
            .update(
                7,
                &agent_id,
                &token,
                UpdateAgentInput {
                    status: None,
                    phase: None,
                    summary: None,
                    progress: None,
                    current_task_id: None,
                    project_id: None,
                    chapter_id: None,
                    metadata: None,
                },
            )
            .expect("heartbeat must renew the expired lease");

        assert_eq!(updated.status, "idle");
        assert!(updated.expires_at > updated.last_heartbeat_at);
    }
}
