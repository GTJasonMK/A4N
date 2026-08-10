use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use a4n_service_protocol::{
    SIDECAR_LAST_ERROR_ENV, SIDECAR_REQUEST_ID_HEADER, SIDECAR_RESTART_COUNT_ENV,
    SIDECAR_TRACE_ID_HEADER, ServiceOperationMetrics, ServiceRuntimeMetrics,
    redact_sensitive_message,
};
use anyhow::{Context, anyhow};
use axum::{
    Json, Router,
    extract::{Request, State},
    middleware::{self, Next},
    response::Response,
    routing::get,
};

use crate::require_auth;

const REQUEST_LOG_FIELD_MAX_CHARS: usize = 128;

#[derive(Clone)]
struct SidecarRuntimeMetrics {
    inner: Arc<MetricsInner>,
}

struct MetricsInner {
    service: String,
    pid: u32,
    started_at_unix_ms: u64,
    started: Instant,
    restart_count: u32,
    last_error: Option<String>,
    requests_total: AtomicU64,
    errors_total: AtomicU64,
    cancelled_total: AtomicU64,
    in_flight: AtomicU64,
    duration_us_total: AtomicU64,
    duration_us_max: AtomicU64,
    operations: Mutex<BTreeMap<String, ServiceOperationMetrics>>,
}

impl SidecarRuntimeMetrics {
    fn from_env(service: &str) -> anyhow::Result<Self> {
        if service.trim().is_empty() {
            return Err(anyhow!("sidecar metrics service name 不能为空"));
        }
        let started_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("系统时间早于 Unix epoch")?
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let restart_count = optional_env_u32(SIDECAR_RESTART_COUNT_ENV)?.unwrap_or(0);
        let last_error = optional_env(SIDECAR_LAST_ERROR_ENV)?;
        Ok(Self::new(
            service,
            started_at_unix_ms,
            restart_count,
            last_error,
        ))
    }

    fn new(
        service: &str,
        started_at_unix_ms: u64,
        restart_count: u32,
        last_error: Option<String>,
    ) -> Self {
        Self {
            inner: Arc::new(MetricsInner {
                service: service.to_string(),
                pid: std::process::id(),
                started_at_unix_ms,
                started: Instant::now(),
                restart_count,
                last_error: last_error.map(|value| sanitize_last_error(&value)),
                requests_total: AtomicU64::new(0),
                errors_total: AtomicU64::new(0),
                cancelled_total: AtomicU64::new(0),
                in_flight: AtomicU64::new(0),
                duration_us_total: AtomicU64::new(0),
                duration_us_max: AtomicU64::new(0),
                operations: Mutex::new(BTreeMap::new()),
            }),
        }
    }

    fn begin(&self, operation: String) -> RequestMetricsGuard {
        self.inner.requests_total.fetch_add(1, Ordering::Relaxed);
        self.inner.in_flight.fetch_add(1, Ordering::Relaxed);
        let mut operations = self.inner.operations.lock().expect("metrics lock poisoned");
        let metrics = operations.entry(operation.clone()).or_default();
        metrics.requests_total += 1;
        metrics.in_flight += 1;
        drop(operations);
        RequestMetricsGuard {
            metrics: self.clone(),
            operation,
            started: Instant::now(),
            finished: false,
        }
    }

    fn snapshot(&self) -> ServiceRuntimeMetrics {
        ServiceRuntimeMetrics {
            service: self.inner.service.clone(),
            pid: self.inner.pid,
            started_at_unix_ms: self.inner.started_at_unix_ms,
            uptime_ms: self
                .inner
                .started
                .elapsed()
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
            restart_count: self.inner.restart_count,
            requests_total: self.inner.requests_total.load(Ordering::Relaxed),
            errors_total: self.inner.errors_total.load(Ordering::Relaxed),
            cancelled_total: self.inner.cancelled_total.load(Ordering::Relaxed),
            in_flight: self.inner.in_flight.load(Ordering::Relaxed),
            duration_us_total: self.inner.duration_us_total.load(Ordering::Relaxed),
            duration_us_max: self.inner.duration_us_max.load(Ordering::Relaxed),
            last_error: self.inner.last_error.clone(),
            operations: self
                .inner
                .operations
                .lock()
                .expect("metrics lock poisoned")
                .clone(),
        }
    }
}

struct RequestMetricsGuard {
    metrics: SidecarRuntimeMetrics,
    operation: String,
    started: Instant,
    finished: bool,
}

impl RequestMetricsGuard {
    fn finish(mut self, failed: bool) -> u64 {
        let duration_us = self
            .started
            .elapsed()
            .as_micros()
            .try_into()
            .unwrap_or(u64::MAX);
        self.metrics.inner.in_flight.fetch_sub(1, Ordering::Relaxed);
        self.metrics
            .inner
            .duration_us_total
            .fetch_add(duration_us, Ordering::Relaxed);
        self.metrics
            .inner
            .duration_us_max
            .fetch_max(duration_us, Ordering::Relaxed);
        if failed {
            self.metrics
                .inner
                .errors_total
                .fetch_add(1, Ordering::Relaxed);
        }
        let mut operations = self
            .metrics
            .inner
            .operations
            .lock()
            .expect("metrics lock poisoned");
        let metrics = operations
            .get_mut(&self.operation)
            .expect("started operation metrics must exist");
        metrics.in_flight -= 1;
        metrics.duration_us_total += duration_us;
        metrics.duration_us_max = metrics.duration_us_max.max(duration_us);
        if failed {
            metrics.errors_total += 1;
        }
        self.finished = true;
        duration_us
    }
}

impl Drop for RequestMetricsGuard {
    fn drop(&mut self) {
        if !self.finished {
            self.metrics.inner.in_flight.fetch_sub(1, Ordering::Relaxed);
            self.metrics
                .inner
                .cancelled_total
                .fetch_add(1, Ordering::Relaxed);
            let mut operations = self
                .metrics
                .inner
                .operations
                .lock()
                .expect("metrics lock poisoned");
            let metrics = operations
                .get_mut(&self.operation)
                .expect("started operation metrics must exist");
            metrics.in_flight -= 1;
            metrics.cancelled_total += 1;
        }
    }
}

pub(super) fn with_runtime_metrics(router: Router, service: &str) -> anyhow::Result<Router> {
    let metrics = SidecarRuntimeMetrics::from_env(service)?;
    let metrics_router = Router::new()
        .route("/metrics", get(runtime_metrics))
        .route_layer(middleware::from_fn(require_auth))
        .with_state(metrics.clone());
    Ok(Router::new()
        .merge(router)
        .merge(metrics_router)
        .layer(middleware::from_fn_with_state(metrics, observe_request)))
}

async fn runtime_metrics(
    State(metrics): State<SidecarRuntimeMetrics>,
) -> Json<ServiceRuntimeMetrics> {
    Json(metrics.snapshot())
}

async fn observe_request(
    State(metrics): State<SidecarRuntimeMetrics>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let path = sanitize_log_field(request.uri().path());
    let request_id = header_text(&request, SIDECAR_REQUEST_ID_HEADER);
    let trace_id = header_text(&request, SIDECAR_TRACE_ID_HEADER);
    let guard = metrics.begin(path.clone());
    let response = next.run(request).await;
    let status = response.status();
    let duration_us = guard.finish(status.is_client_error() || status.is_server_error());
    tracing::info!(
        service = %metrics.inner.service,
        %method,
        %path,
        status = status.as_u16(),
        duration_us,
        request_id = request_id.as_deref().unwrap_or(""),
        trace_id = trace_id.as_deref().unwrap_or(""),
        "sidecar request completed"
    );
    response
}

fn header_text(request: &Request, name: &'static str) -> Option<String> {
    request
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(sanitize_log_field)
}

fn sanitize_log_field(value: &str) -> String {
    redact_sensitive_message(value)
        .chars()
        .take(REQUEST_LOG_FIELD_MAX_CHARS)
        .collect()
}

fn optional_env(key: &str) -> anyhow::Result<Option<String>> {
    match std::env::var(key) {
        Ok(value) => {
            let value = value.trim();
            if value.is_empty() {
                Err(anyhow!("{key} 不能为空"))
            } else {
                Ok(Some(value.to_string()))
            }
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(anyhow!("读取 {key} 失败: {error}")),
    }
}

fn optional_env_u32(key: &str) -> anyhow::Result<Option<u32>> {
    optional_env(key)?
        .map(|value| {
            value
                .parse::<u32>()
                .with_context(|| format!("{key} 必须是非负整数"))
        })
        .transpose()
}

fn sanitize_last_error(value: &str) -> String {
    redact_sensitive_message(value).chars().take(512).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_completion_and_cancellation_update_one_shared_snapshot() {
        let metrics = SidecarRuntimeMetrics::new("demo", 1, 2, Some("previous crash".into()));
        metrics.begin("/rag/search".into()).finish(true);
        drop(metrics.begin("/rag/ingest/events".into()));

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.restart_count, 2);
        assert_eq!(snapshot.requests_total, 2);
        assert_eq!(snapshot.errors_total, 1);
        assert_eq!(snapshot.cancelled_total, 1);
        assert_eq!(snapshot.in_flight, 0);
        assert_eq!(snapshot.last_error.as_deref(), Some("previous crash"));
        assert_eq!(snapshot.operations["/rag/search"].errors_total, 1);
        assert_eq!(snapshot.operations["/rag/ingest/events"].cancelled_total, 1);
    }

    #[test]
    fn last_error_redacts_urls_tokens_and_sensitive_json_fields() {
        for (message, secret) in [
            (
                "request failed at https://user:secret@example.test/path",
                "user:secret@example.test",
            ),
            ("provider failed: token=abc", "token=abc"),
            ("provider failed: authorization: bearer abc", "bearer abc"),
            (
                r#"{"endpoint":"https://provider.example/v1","api_key":"key","password":"pw","visible":"ok"}"#,
                "\"api_key\":\"key\"",
            ),
        ] {
            let redacted = sanitize_last_error(message);
            assert!(
                !redacted.contains(secret),
                "redacted message leaked {secret:?}: {redacted}"
            );
        }

        let redacted = sanitize_last_error(
            r#"{"endpoint":"https://provider.example/v1","api_key":"key","password":"pw","visible":"ok"}"#,
        );
        assert!(!redacted.contains("\"password\":\"pw\""));
        assert!(!redacted.contains("provider.example"));
        assert!(redacted.contains("visible"));
    }

    #[test]
    fn request_log_fields_are_redacted_and_bounded() {
        assert_eq!(sanitize_log_field("token=secret"), "token=[REDACTED]");
        assert_eq!(sanitize_log_field("/token=secret"), "/token=[REDACTED]");
        assert_eq!(
            sanitize_log_field("https://user:secret@provider.example/v1"),
            "[REDACTED_URL]"
        );
        assert_eq!(
            sanitize_log_field(&"a".repeat(REQUEST_LOG_FIELD_MAX_CHARS + 1))
                .chars()
                .count(),
            REQUEST_LOG_FIELD_MAX_CHARS
        );
    }
}
