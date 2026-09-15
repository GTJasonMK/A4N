use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll},
};

use a4n_export_contract::{
    EXPORT_SERVICE_ADDR_ENV, EXPORT_SERVICE_DEFAULT_ADDR, EXPORT_SERVICE_NAME,
    ExportRenderJsonInput, ExportRenderZipInput, ExportRenderZipOutput,
};
use a4n_export_core::{render_json, render_zip};
use a4n_service_protocol::{
    HealthStatus, SIDECAR_HEALTH_PATH, SIDECAR_TRACE_ID_HEADER, ServiceError, ServiceErrorCode,
    ServiceHealth, ServiceRequest, ServiceResponse, known_service_health, redact_sensitive_message,
};
use axum::{
    Json, Router,
    body::{Body, BodyDataStream},
    extract::{DefaultBodyLimit, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use futures::Stream;

mod render_gate;
use render_gate::RenderAdmission;

#[derive(Clone)]
struct RenderAdmissionState {
    admission: RenderAdmission,
    metrics: ExportRenderMetrics,
}

#[derive(Clone, Default)]
struct ExportRenderMetrics {
    inner: Arc<ExportRenderMetricsInner>,
}

#[derive(Default)]
struct ExportRenderMetricsInner {
    active_renders: AtomicU64,
    in_flight_requests: AtomicU64,
    peak_active_renders: AtomicU64,
    peak_queue_depth: AtomicU64,
    queue_rejections: AtomicU64,
    max_concurrent_renders: usize,
    queue_capacity: usize,
}

impl ExportRenderMetrics {
    fn with_policy(max_concurrent_renders: usize, queue_capacity: usize) -> Self {
        Self {
            inner: Arc::new(ExportRenderMetricsInner {
                max_concurrent_renders,
                queue_capacity,
                ..Default::default()
            }),
        }
    }

    fn observe_request(&self) -> RenderRequestGuard {
        let in_flight = self
            .inner
            .in_flight_requests
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        let active = self.inner.active_renders.load(Ordering::Relaxed);
        if active > 0 {
            self.inner
                .peak_queue_depth
                .fetch_max(in_flight.saturating_sub(active), Ordering::Relaxed);
        }
        RenderRequestGuard {
            metrics: self.clone(),
        }
    }

    fn begin(&self) -> ActiveRenderGuard {
        let active = self.inner.active_renders.fetch_add(1, Ordering::Relaxed) + 1;
        self.inner
            .peak_active_renders
            .fetch_max(active, Ordering::Relaxed);
        self.inner
            .peak_queue_depth
            .fetch_max(self.queue_depth(), Ordering::Relaxed);
        ActiveRenderGuard {
            metrics: self.clone(),
        }
    }

    fn queue_depth(&self) -> u64 {
        self.inner
            .in_flight_requests
            .load(Ordering::Relaxed)
            .saturating_sub(self.inner.active_renders.load(Ordering::Relaxed))
    }

    fn record_queue_rejection(&self) {
        self.inner.queue_rejections.fetch_add(1, Ordering::Relaxed);
    }
}

struct RenderRequestGuard {
    metrics: ExportRenderMetrics,
}

impl Drop for RenderRequestGuard {
    fn drop(&mut self) {
        self.metrics
            .inner
            .in_flight_requests
            .fetch_sub(1, Ordering::Relaxed);
    }
}

struct ActiveRenderGuard {
    metrics: ExportRenderMetrics,
}

impl Drop for ActiveRenderGuard {
    fn drop(&mut self) {
        self.metrics
            .inner
            .active_renders
            .fetch_sub(1, Ordering::Relaxed);
    }
}

struct RenderAdmissionHold {
    _slot: tokio::sync::OwnedSemaphorePermit,
    _active: tokio::sync::OwnedSemaphorePermit,
}

struct HeldResponseBody {
    body: BodyDataStream,
    admission: Option<RenderAdmissionHold>,
    request: Option<RenderRequestGuard>,
}

impl Stream for HeldResponseBody {
    type Item = Result<axum::body::Bytes, axum::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let result = Pin::new(&mut self.body).poll_next(cx);
        if matches!(result, Poll::Ready(None)) {
            self.admission.take();
            self.request.take();
        }
        result
    }
}

fn hold_response_body(
    response: Response,
    admission: Option<RenderAdmissionHold>,
    request: Option<RenderRequestGuard>,
) -> Response {
    let (parts, body) = response.into_parts();
    Response::from_parts(
        parts,
        Body::from_stream(HeldResponseBody {
            body: body.into_data_stream(),
            admission,
            request,
        }),
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_EXPORT_SERVICE_LOG",
        "a4n_export_service=info,tower_http=warn",
    )?;
    let addr =
        a4n_sidecar_bootstrap::socket_addr(EXPORT_SERVICE_ADDR_ENV, EXPORT_SERVICE_DEFAULT_ADDR)?;
    let policy = a4n_service_protocol::sidecar_resource_policy(EXPORT_SERVICE_NAME)
        .expect("export sidecar must have a resource policy");
    let render_metrics = ExportRenderMetrics::with_policy(
        policy.max_concurrent_requests,
        render_gate::RENDER_QUEUE_CAPACITY,
    );
    let render_admission = RenderAdmissionState {
        admission: RenderAdmission::new(policy.max_concurrent_requests),
        metrics: render_metrics.clone(),
    };
    let render_routes = Router::new()
        .route(SIDECAR_HEALTH_PATH, get(healthz))
        .route(
            "/export/render_json",
            post(render_json_route).layer(DefaultBodyLimit::max(
                policy.body_limit(a4n_service_protocol::BodyLimitKind::Json),
            )),
        )
        .route(
            "/export/render_zip",
            post(render_zip_route).layer(DefaultBodyLimit::max(
                policy.body_limit(a4n_service_protocol::BodyLimitKind::Media),
            )),
        )
        .layer(a4n_sidecar_bootstrap::concurrency_limit(
            policy.max_concurrent_requests,
        ))
        .layer(middleware::from_fn_with_state(
            render_admission,
            admit_render_request,
        ))
        .with_state(render_metrics.clone());
    let app = render_routes.merge(
        Router::new()
            .route("/export/metrics", get(render_metrics_route))
            .with_state(render_metrics),
    );
    a4n_sidecar_bootstrap::serve_sidecar(app, addr, EXPORT_SERVICE_NAME).await
}

async fn healthz() -> Json<ServiceHealth> {
    Json(known_service_health(
        EXPORT_SERVICE_NAME,
        env!("CARGO_PKG_VERSION"),
        HealthStatus::Healthy,
    ))
}

async fn render_json_route(
    State(metrics): State<ExportRenderMetrics>,
    Json(request): Json<ServiceRequest<ExportRenderJsonInput>>,
) -> Json<ServiceResponse<String>> {
    Json(request.execute_known_service(
        EXPORT_SERVICE_NAME,
        |input| {
            let _guard = metrics.begin();
            render_json(input)
        },
        operation_error,
    ))
}

async fn render_zip_route(
    State(metrics): State<ExportRenderMetrics>,
    Json(request): Json<ServiceRequest<ExportRenderZipInput>>,
) -> Json<ServiceResponse<ExportRenderZipOutput>> {
    Json(request.execute_known_service(
        EXPORT_SERVICE_NAME,
        |input| {
            let _guard = metrics.begin();
            render_zip(input)
        },
        operation_error,
    ))
}

async fn render_metrics_route(
    State(metrics): State<ExportRenderMetrics>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "service": EXPORT_SERVICE_NAME,
        "active_renders": metrics.inner.active_renders.load(Ordering::Relaxed),
        "peak_active_renders": metrics.inner.peak_active_renders.load(Ordering::Relaxed),
        "queue_depth": metrics.queue_depth(),
        "peak_queue_depth": metrics.inner.peak_queue_depth.load(Ordering::Relaxed),
        "queue_rejections": metrics.inner.queue_rejections.load(Ordering::Relaxed),
        "max_concurrent_renders": metrics.inner.max_concurrent_renders,
        "queue_capacity": metrics.inner.queue_capacity,
    }))
}

async fn admit_render_request(
    State(admission): State<RenderAdmissionState>,
    request: Request,
    next: Next,
) -> Response {
    if !matches!(
        request.uri().path(),
        "/export/render_json" | "/export/render_zip"
    ) {
        return next.run(request).await;
    }

    let trace_id = request
        .headers()
        .get(SIDECAR_TRACE_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .unwrap_or_default();
    let request_guard = admission.metrics.observe_request();
    let Some(slot) = admission.admission.try_reserve_slot() else {
        admission.metrics.record_queue_rejection();
        let body = Json(ServiceResponse::<serde_json::Value>::error(
            trace_id,
            ServiceError::new(
                ServiceErrorCode::ServiceUnavailable,
                "export render queue is full",
                true,
            ),
        ));
        return hold_response_body(
            (StatusCode::TOO_MANY_REQUESTS, body).into_response(),
            None,
            Some(request_guard),
        );
    };
    let active = admission.admission.acquire_active().await;
    hold_response_body(
        next.run(request).await,
        Some(RenderAdmissionHold {
            _slot: slot,
            _active: active,
        }),
        Some(request_guard),
    )
}

fn operation_error(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::non_retryable(
        ServiceErrorCode::ConfigurationError,
        redact_sensitive_message(error.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, response::Response};

    use super::{ExportRenderMetrics, hold_response_body};

    #[test]
    fn active_render_observation_is_scoped_to_guard_lifetime() {
        let metrics = ExportRenderMetrics::default();
        let first = metrics.begin();
        let second = metrics.begin();
        assert_eq!(
            metrics
                .inner
                .active_renders
                .load(std::sync::atomic::Ordering::Relaxed),
            2
        );
        assert_eq!(
            metrics
                .inner
                .peak_active_renders
                .load(std::sync::atomic::Ordering::Relaxed),
            2
        );
        drop(first);
        assert_eq!(
            metrics
                .inner
                .active_renders
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        drop(second);
        assert_eq!(
            metrics
                .inner
                .active_renders
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    #[test]
    fn queue_depth_excludes_active_render_and_returns_to_zero() {
        let metrics = ExportRenderMetrics::with_policy(4, 4);
        let request = metrics.observe_request();
        assert_eq!(metrics.queue_depth(), 1);

        let active = metrics.begin();
        assert_eq!(metrics.queue_depth(), 0);
        assert_eq!(
            metrics
                .inner
                .peak_queue_depth
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );

        drop(active);
        drop(request);
        assert_eq!(metrics.queue_depth(), 0);
    }

    #[test]
    fn queued_request_updates_peak_depth_without_rejecting_it() {
        let metrics = ExportRenderMetrics::with_policy(4, 4);
        let active_request = metrics.observe_request();
        let active = metrics.begin();
        let queued_request = metrics.observe_request();

        assert_eq!(metrics.queue_depth(), 1);
        assert_eq!(
            metrics
                .inner
                .peak_queue_depth
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );

        drop(queued_request);
        drop(active);
        drop(active_request);
    }

    #[tokio::test]
    async fn request_observation_lasts_until_response_body_is_drained() {
        let metrics = ExportRenderMetrics::with_policy(1, 1);
        let request = metrics.observe_request();
        let response =
            hold_response_body(Response::new(Body::from("response")), None, Some(request));

        assert_eq!(metrics.queue_depth(), 1);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .expect("response body should drain");
        assert_eq!(&body[..], b"response");
        assert_eq!(metrics.queue_depth(), 0);
    }
}
