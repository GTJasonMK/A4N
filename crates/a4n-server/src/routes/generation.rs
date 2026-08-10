use super::auth::authenticated_user;
use super::error::GenerationApiError;
use crate::state::ServerState;
use a4n_api_contract::generation_tasks::{
    CancelGenerationTaskInput, GenerationTaskHistoryPage, GenerationTaskHistoryQuery,
    GenerationTaskSummary,
};
use a4n_api_contract::{
    blueprint::{StartBlueprintGenerationInput, StartBlueprintRefinementInput},
    chapter::StartChapterGenerationInput,
    image_generation::StartImageGenerationInput,
    inspiration::StartInspirationGenerationInput,
    optimize::{
        StartOptimizeAnalysisGenerationInput, StartOptimizeChatGenerationInput,
        StartOptimizePatchGenerationInput,
    },
    outline::StartOutlineGenerationInput,
    portrait::StartPortraitGenerationInput,
    video::StartVideoGenerationInput,
};
use a4n_runtime::host_api::generation_tasks as generation_actions;
use a4n_runtime::host_api::generation_tasks::{GenerationReplay, GenerationSubscription};
use a4n_service_protocol::{
    GenerationTaskEvent, GenerationTaskSnapshot, MAX_GENERATION_SNAPSHOT_BYTES,
};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::{Stream, stream};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub(super) struct AttemptQuery {
    attempt: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProjectListQuery {
    project_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HistoryListQuery {
    project_id: Option<String>,
    limit: Option<String>,
    cursor: Option<String>,
}

fn generation_start_input<T>(
    mut body: Value,
) -> Result<
    (
        T,
        Option<a4n_runtime::runtime_config::RuntimeConfigSnapshot>,
    ),
    GenerationApiError,
>
where
    T: DeserializeOwned,
{
    let runtime_config_snapshot =
        a4n_runtime::runtime_config::snapshot_from_args(&body).map_err(|message| {
            GenerationApiError::bad_request("generation_request_invalid", message)
        })?;
    let fields = body.as_object_mut().ok_or_else(|| {
        GenerationApiError::bad_request(
            "generation_request_invalid",
            "generation 请求必须是 JSON object",
        )
    })?;
    fields.remove("runtime_config_snapshot");
    let input = serde_json::from_value(body).map_err(|error| {
        GenerationApiError::bad_request(
            "generation_request_invalid",
            format!("generation 请求无效: {error}"),
        )
    })?;
    Ok((input, runtime_config_snapshot))
}

async fn authenticated_generation_user_id(
    state: &ServerState,
    headers: &HeaderMap,
) -> Result<i64, GenerationApiError> {
    authenticated_user(state, headers)
        .await
        .map(|user| user.id)
        .map_err(|_| {
            GenerationApiError::unauthorized(
                "generation_task_unauthenticated",
                "需要有效的 Bearer token",
            )
        })
}

macro_rules! generation_start_route {
    ($name:ident, $input:ty, $action:path) => {
        pub(super) async fn $name(
            State(state): State<ServerState>,
            headers: HeaderMap,
            Json(body): Json<Value>,
        ) -> Result<
            (
                StatusCode,
                Json<a4n_api_contract::generation_tasks::GenerationTaskHandle>,
            ),
            GenerationApiError,
        > {
            let user_id = authenticated_generation_user_id(&state, &headers).await?;
            let (input, runtime_config_snapshot) = generation_start_input::<$input>(body)?;
            let handle = a4n_runtime::runtime_config::request_scope(
                user_id,
                runtime_config_snapshot,
                ($action)(state.runtime(), user_id, input),
            )
            .await
            .map_err(map_access_error)?;
            let status = if handle.status.is_terminal() {
                StatusCode::OK
            } else {
                StatusCode::ACCEPTED
            };
            Ok((status, Json(handle)))
        }
    };
}

generation_start_route!(
    start_chapter,
    StartChapterGenerationInput,
    a4n_runtime::host_api::chapter_generation::start_chapter_generation
);
generation_start_route!(
    start_inspiration,
    StartInspirationGenerationInput,
    a4n_runtime::host_api::inspiration_records::start_inspiration_generation
);
generation_start_route!(
    start_blueprint,
    StartBlueprintGenerationInput,
    a4n_runtime::host_api::blueprint_records::start_blueprint_generation
);
generation_start_route!(
    start_blueprint_refinement,
    StartBlueprintRefinementInput,
    a4n_runtime::host_api::blueprint_records::start_blueprint_refinement
);
generation_start_route!(
    start_outline,
    StartOutlineGenerationInput,
    a4n_runtime::host_api::outline_records::start_outline_generation
);
generation_start_route!(
    start_portrait,
    StartPortraitGenerationInput,
    a4n_runtime::host_api::portrait_generation::start_portrait_generation
);
generation_start_route!(
    start_image,
    StartImageGenerationInput,
    a4n_runtime::host_api::image_generation::start_image_generation
);
generation_start_route!(
    start_video,
    StartVideoGenerationInput,
    a4n_runtime::host_api::video_generation::start_video_generation
);
generation_start_route!(
    start_optimize_chat,
    StartOptimizeChatGenerationInput,
    a4n_runtime::host_api::optimize::start_optimize_chat_generation
);
generation_start_route!(
    start_optimize_analysis,
    StartOptimizeAnalysisGenerationInput,
    a4n_runtime::host_api::optimize::start_optimize_analysis_generation
);
generation_start_route!(
    start_optimize_patch,
    StartOptimizePatchGenerationInput,
    a4n_runtime::host_api::optimize::start_optimize_patch_generation
);

pub(super) async fn task(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Query(query): Query<AttemptQuery>,
) -> Result<Json<GenerationTaskSummary>, GenerationApiError> {
    let user_id = authenticated_generation_user_id(&state, &headers).await?;
    let attempt = parse_attempt(query.attempt.as_deref())?;
    let summary =
        generation_actions::generation_task_summary(state.runtime(), user_id, &task_id, attempt)
            .await
            .map_err(map_access_error)?;
    Ok(Json(summary))
}

pub(super) async fn active(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<ProjectListQuery>,
) -> Result<Json<Vec<GenerationTaskSummary>>, GenerationApiError> {
    let user_id = authenticated_generation_user_id(&state, &headers).await?;
    let project_id = parse_project_id(query.project_id.as_deref())?;
    let items =
        generation_actions::list_active_generation_tasks(state.runtime(), user_id, project_id)
            .await
            .map_err(map_access_error)?;
    Ok(Json(items))
}

pub(super) async fn history(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<HistoryListQuery>,
) -> Result<Json<GenerationTaskHistoryPage>, GenerationApiError> {
    let user_id = authenticated_generation_user_id(&state, &headers).await?;
    let project_id = parse_project_id(query.project_id.as_deref())?;
    let limit = parse_history_limit(query.limit.as_deref())?;
    let cursor = query
        .cursor
        .as_deref()
        .map(generation_actions::decode_generation_history_cursor)
        .transpose()
        .map_err(|error| {
            GenerationApiError::bad_request("generation_cursor_invalid", error.to_string())
        })?;
    let input = GenerationTaskHistoryQuery {
        project_id,
        limit,
        cursor: query.cursor,
    };
    let page =
        generation_actions::list_generation_task_history(state.runtime(), user_id, input, cursor)
            .await
            .map_err(map_access_error)?;
    Ok(Json(page))
}

pub(super) async fn snapshot(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Query(query): Query<AttemptQuery>,
) -> Result<Json<GenerationTaskSnapshot>, GenerationApiError> {
    let user_id = authenticated_generation_user_id(&state, &headers).await?;
    let attempt = parse_attempt(query.attempt.as_deref())?;
    let task =
        generation_actions::authorize_generation_task(state.runtime(), user_id, &task_id, attempt)
            .await
            .map_err(map_access_error)?;
    let snapshot = task.public_snapshot.ok_or_else(|| {
        if task
            .checkpoint
            .expires_at
            .is_some_and(|expires_at| expires_at <= chrono::Utc::now().naive_utc())
        {
            GenerationApiError::gone("generation_snapshot_expired", "generation snapshot 已过期")
        } else {
            GenerationApiError::internal(
                "generation_checkpoint_unavailable",
                "generation snapshot 当前不可用",
                true,
            )
        }
    })?;
    let encoded = serde_json::to_vec(&snapshot).map_err(|error| {
        GenerationApiError::internal(
            "generation_checkpoint_corrupt",
            format!("generation snapshot 序列化失败: {error}"),
            false,
        )
    })?;
    if encoded.len() > MAX_GENERATION_SNAPSHOT_BYTES {
        return Err(GenerationApiError::payload_too_large(
            "generation_snapshot_too_large",
            "generation snapshot 超过固定大小上限",
        ));
    }
    Ok(Json(snapshot))
}

pub(super) async fn events(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Query(query): Query<AttemptQuery>,
) -> Result<impl IntoResponse, GenerationApiError> {
    let user_id = authenticated_generation_user_id(&state, &headers).await?;
    let attempt = parse_attempt(query.attempt.as_deref())?;
    let task =
        generation_actions::authorize_generation_task(state.runtime(), user_id, &task_id, attempt)
            .await
            .map_err(map_access_error)?;
    let cursor = parse_last_event_id(&headers, attempt)?;
    let mut subscription =
        generation_actions::subscribe_generation_task(state.runtime(), &task, cursor)
            .map_err(map_access_error)?;
    let initial = subscription.take_initial().ok_or_else(|| {
        GenerationApiError::internal(
            "generation_checkpoint_unavailable",
            "generation subscription 缺少初始 replay",
            true,
        )
    })?;
    let (initial_events, latest, terminal) = match initial {
        GenerationReplay::Events {
            events,
            latest_sequence,
            ..
        } => {
            let terminal = task
                .public_snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.status.is_terminal());
            (events, latest_sequence, terminal)
        }
        GenerationReplay::SnapshotRequired { .. } => {
            return Err(GenerationApiError::conflict(
                "generation_snapshot_required",
                "replay cursor 已超出保留范围，请先读取 snapshot",
            ));
        }
    };
    let cursor = cursor.unwrap_or(0);
    if initial_events.is_empty() && terminal && latest <= cursor {
        return Err(GenerationApiError::conflict(
            "generation_snapshot_required",
            "终态任务没有可继续回放的事件，请先读取 snapshot",
        ));
    }

    let terminal_pending = initial_events.iter().any(|event| event.event.is_terminal());
    let mut pending = VecDeque::with_capacity(initial_events.len());
    let mut next_cursor = cursor;
    for event in initial_events {
        next_cursor = event.sequence;
        pending.push_back(to_sse_event(event, attempt)?);
    }
    let stream = generation_sse_stream(GenerationSseState {
        subscription,
        pending,
        cursor: next_cursor,
        attempt,
        terminal_pending,
    });
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-transform"),
    );
    response_headers.insert("x-accel-buffering", HeaderValue::from_static("no"));
    Ok((
        response_headers,
        Sse::new(stream).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(5))
                .text("keep-alive"),
        ),
    ))
}

pub(super) async fn cancel(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(input): Json<CancelGenerationTaskInput>,
) -> Result<
    (
        StatusCode,
        Json<a4n_api_contract::generation_tasks::GenerationTaskHandle>,
    ),
    GenerationApiError,
> {
    if input.task_id != task_id {
        return Err(GenerationApiError::bad_request(
            "generation_request_invalid",
            "body.task_id 必须与 path task_id 一致",
        ));
    }
    let user_id = authenticated_generation_user_id(&state, &headers).await?;
    let handle = generation_actions::cancel_generation_task(state.runtime(), user_id, input)
        .await
        .map_err(map_terminal_error)?;
    Ok((StatusCode::ACCEPTED, Json(handle)))
}

struct GenerationSseState {
    subscription: GenerationSubscription,
    pending: VecDeque<Event>,
    cursor: u64,
    attempt: u32,
    terminal_pending: bool,
}

fn generation_sse_stream(
    state: GenerationSseState,
) -> impl Stream<Item = Result<Event, Infallible>> + Send + 'static {
    stream::unfold(state, |mut state| async move {
        loop {
            if let Some(event) = state.pending.pop_front() {
                return Some((Ok(event), state));
            }
            if state.terminal_pending {
                return None;
            }
            if state.subscription.receiver().changed().await.is_err() {
                return None;
            }
            let replay = state.subscription.replay_after(Some(state.cursor));
            match replay {
                GenerationReplay::Events {
                    events,
                    latest_sequence,
                    ..
                } => {
                    if events.is_empty() {
                        if latest_sequence <= state.cursor
                            && state.subscription.snapshot().status.is_terminal()
                        {
                            return None;
                        }
                        continue;
                    }
                    for event in events {
                        state.cursor = event.sequence;
                        if event.event.is_terminal() {
                            state.terminal_pending = true;
                        }
                        match to_sse_event(event, state.attempt) {
                            Ok(event) => state.pending.push_back(event),
                            Err(error) => {
                                tracing::error!(error = ?error, "generation SSE event serialization failed");
                                return None;
                            }
                        }
                    }
                }
                GenerationReplay::SnapshotRequired { .. } => {
                    tracing::warn!(
                        attempt = state.attempt,
                        cursor = state.cursor,
                        "generation SSE replay floor passed; closing for snapshot recovery"
                    );
                    return None;
                }
            }
        }
    })
}

fn to_sse_event(event: GenerationTaskEvent, attempt: u32) -> Result<Event, GenerationApiError> {
    let sequence = event.sequence;
    let data = serde_json::to_string(&event).map_err(|error| {
        GenerationApiError::internal(
            "generation_checkpoint_corrupt",
            format!("generation event 序列化失败: {error}"),
            false,
        )
    })?;
    Ok(Event::default()
        .event("generation_task_event")
        .id(format!("{attempt}:{sequence}"))
        .data(data))
}

fn parse_attempt(raw: Option<&str>) -> Result<u32, GenerationApiError> {
    let Some(raw) = raw else {
        return Err(GenerationApiError::bad_request(
            "generation_request_invalid",
            "attempt 必须提供",
        ));
    };
    let attempt = raw.parse::<u32>().map_err(|_| {
        GenerationApiError::bad_request("generation_request_invalid", "attempt 必须是正整数")
    })?;
    if attempt == 0 {
        return Err(GenerationApiError::bad_request(
            "generation_request_invalid",
            "attempt 必须大于 0",
        ));
    }
    Ok(attempt)
}

fn parse_project_id(raw: Option<&str>) -> Result<Option<i64>, GenerationApiError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let project_id = raw.parse::<i64>().map_err(|_| {
        GenerationApiError::bad_request(
            "generation_request_invalid",
            "project_id 必须是正的 JavaScript safe integer",
        )
    })?;
    if !(1..=a4n_service_protocol::JS_SAFE_INTEGER_I64).contains(&project_id) {
        return Err(GenerationApiError::bad_request(
            "generation_request_invalid",
            "project_id 必须是正的 JavaScript safe integer",
        ));
    }
    Ok(Some(project_id))
}

fn parse_history_limit(raw: Option<&str>) -> Result<u32, GenerationApiError> {
    let limit = raw
        .map(|raw| raw.parse::<u32>())
        .transpose()
        .map_err(|_| {
            GenerationApiError::bad_request(
                "generation_request_invalid",
                "limit 必须是 1..=100 的整数",
            )
        })?
        .unwrap_or(generation_actions::DEFAULT_GENERATION_HISTORY_LIMIT);
    if !(1..=generation_actions::MAX_GENERATION_HISTORY_LIMIT).contains(&limit) {
        return Err(GenerationApiError::bad_request(
            "generation_request_invalid",
            "limit 必须是 1..=100 的整数",
        ));
    }
    Ok(limit)
}

fn parse_last_event_id(
    headers: &HeaderMap,
    attempt: u32,
) -> Result<Option<u64>, GenerationApiError> {
    let Some(value) = headers.get(HeaderName::from_static("last-event-id")) else {
        return Ok(None);
    };
    let raw = value.to_str().map_err(|_| {
        GenerationApiError::bad_request(
            "generation_request_invalid",
            "Last-Event-ID 必须是有效的 ASCII 字符串",
        )
    })?;
    let (raw_attempt, raw_sequence) = raw.split_once(':').ok_or_else(|| {
        GenerationApiError::bad_request(
            "generation_request_invalid",
            "Last-Event-ID 必须使用 {attempt}:{sequence}",
        )
    })?;
    let id_attempt = raw_attempt.parse::<u32>().map_err(|_| {
        GenerationApiError::bad_request("generation_request_invalid", "Last-Event-ID attempt 无效")
    })?;
    if id_attempt != attempt {
        return Err(GenerationApiError::conflict(
            "generation_attempt_mismatch",
            format!("Last-Event-ID attempt={id_attempt} 与请求 attempt={attempt} 不一致"),
        ));
    }
    let sequence = raw_sequence.parse::<u64>().map_err(|_| {
        GenerationApiError::bad_request("generation_request_invalid", "Last-Event-ID sequence 无效")
    })?;
    Ok(Some(sequence))
}

fn map_access_error(error: generation_actions::GenerationTaskAccessError) -> GenerationApiError {
    let code = error.code();
    match code {
        "generation_task_not_found" => GenerationApiError::not_found(code, error.to_string()),
        "generation_task_forbidden" => GenerationApiError::forbidden(code, error.to_string()),
        "generation_attempt_mismatch" | "generation_snapshot_required" => {
            GenerationApiError::conflict(code, error.to_string())
        }
        "generation_request_invalid" => GenerationApiError::bad_request(code, error.to_string()),
        "generation_subscription_capacity_exceeded" => {
            GenerationApiError::new(StatusCode::TOO_MANY_REQUESTS, code, error.to_string(), true)
        }
        "generation_session_capacity_exceeded" => GenerationApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            code,
            error.to_string(),
            true,
        ),
        _ => GenerationApiError::internal(code, error.to_string(), true),
    }
}

fn map_terminal_error(error: generation_actions::GenerationTerminalError) -> GenerationApiError {
    let code = error.code();
    let status = match code {
        "generation_attempt_mismatch"
        | "generation_cancel_too_late"
        | "generation_terminal_already_set" => StatusCode::CONFLICT,
        "generation_terminal_persistence_unavailable" => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    GenerationApiError::new(
        status,
        code,
        error.to_string(),
        status == StatusCode::SERVICE_UNAVAILABLE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_start_separates_runtime_snapshot_from_task_input() {
        let secret = "sk-request-only";
        let (input, snapshot) =
            generation_start_input::<StartImageGenerationInput>(serde_json::json!({
                "meta": {
                    "start_request_id": "00000000-0000-4000-8000-000000000001",
                    "retry_of_task_id": null
                },
                "project_id": 5,
                "operation": {
                    "kind": "scene",
                    "prompt": "雨夜车站",
                    "config_id": null,
                    "chapter_id": 9
                },
                "runtime_config_snapshot": {
                    "schema_version": 1,
                    "image": {
                        "id": 1,
                        "name": "request image config",
                        "backend": "openai-compatible-image",
                        "base_url": "https://example.test/v1",
                        "api_key_encrypted": secret,
                        "model": "image-test",
                        "max_concurrent": 1,
                        "default_width": 512,
                        "default_height": 512,
                        "is_default": true
                    }
                }
            }))
            .unwrap();

        assert_eq!(input.project_id, 5);
        assert_eq!(
            snapshot.unwrap().image.unwrap().name,
            "request image config"
        );
        assert!(!serde_json::to_string(&input).unwrap().contains(secret));
    }

    #[test]
    fn parses_generation_attempt_and_last_event_id() {
        assert_eq!(parse_attempt(Some("2")).unwrap(), 2);
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("last-event-id"),
            HeaderValue::from_static("2:17"),
        );
        assert_eq!(parse_last_event_id(&headers, 2).unwrap(), Some(17));
    }

    #[test]
    fn rejects_malformed_or_cross_attempt_cursor() {
        assert!(parse_attempt(Some("0")).is_err());
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("last-event-id"),
            HeaderValue::from_static("1:17"),
        );
        let error = parse_last_event_id(&headers, 2).unwrap_err();
        assert_eq!(error.into_response().status(), StatusCode::CONFLICT);
        assert_eq!(parse_history_limit(None).unwrap(), 50);
        assert!(parse_history_limit(Some("101")).is_err());
        assert_eq!(parse_project_id(Some("42")).unwrap(), Some(42));
        assert!(parse_project_id(Some("0")).is_err());
    }
}
