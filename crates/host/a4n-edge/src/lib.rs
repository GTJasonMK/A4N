use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderName, Request, StatusCode, header},
    response::{IntoResponse, Response},
    routing::any,
};
use futures::StreamExt;
use reqwest::Client;
use std::{
    env,
    net::SocketAddr,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

const EDGE_THREAD_STACK_SIZE: usize = 8 * 1024 * 1024;
const DEFAULT_WEB_ADDR: &str = "127.0.0.1:8080";
const DEFAULT_CONTROL_ADDR: &str = "127.0.0.1:18080";

#[derive(Clone)]
struct EdgeState {
    client: Client,
    control_base_url: String,
}

pub fn run() -> anyhow::Result<()> {
    run_with_control_addr(None)
}

pub fn run_with_control_addr(control_addr: Option<SocketAddr>) -> anyhow::Result<()> {
    run_with_control_addr_until(control_addr, Arc::new(AtomicBool::new(false)))
}

pub fn run_with_control_addr_until(
    control_addr: Option<SocketAddr>,
    stop_requested: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(EDGE_THREAD_STACK_SIZE)
        .enable_all()
        .build()?
        .block_on(async_main(control_addr, stop_requested))
}

async fn async_main(
    control_addr: Option<SocketAddr>,
    stop_requested: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    init_tracing();
    let bind_addr = parse_addr("A4N_WEB_ADDR", DEFAULT_WEB_ADDR)?;
    let control_addr = match control_addr {
        Some(control_addr) => control_addr,
        None => parse_optional_addr("A4N_CONTROL_ADDR")?.unwrap_or_else(|| {
            parse_addr_value(DEFAULT_CONTROL_ADDR).expect("valid default address")
        }),
    };
    let frontend_dist = env::var_os("A4N_FRONTEND_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("frontend/dist"));
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .build()?;
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    let app = router(
        EdgeState {
            client,
            control_base_url: format!("http://{control_addr}"),
        },
        frontend_dist,
    );

    tracing::info!(
        "A4N Edge listening on http://{bind_addr}, forwarding API to http://{control_addr}"
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(stop_requested))
        .await?;
    Ok(())
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

fn router(state: EdgeState, frontend_dist: PathBuf) -> Router {
    let index_file = frontend_dist.join("index.html");
    let static_files = ServeDir::new(frontend_dist)
        .append_index_html_on_directories(true)
        .fallback(ServeFile::new(index_file));

    Router::new()
        .route("/api", any(proxy))
        .route("/api/{*path}", any(proxy))
        .fallback_service(static_files)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn proxy(State(state): State<EdgeState>, request: Request<Body>) -> Response {
    let path_and_query = request
        .uri()
        .path_and_query()
        .map(ToString::to_string)
        .unwrap_or_else(|| "/api".to_string());
    let target_url = format!("{}{path_and_query}", state.control_base_url);
    let (parts, body) = request.into_parts();
    let mut outgoing = state.client.request(parts.method, target_url);
    for (name, value) in &parts.headers {
        if !is_hop_by_hop(name) {
            outgoing = outgoing.header(name, value);
        }
    }

    let response = match outgoing
        .body(reqwest::Body::wrap_stream(body.into_data_stream()))
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => return control_unavailable(error),
    };

    let status = StatusCode::from_u16(response.status().as_u16())
        .expect("reqwest status code must be a valid HTTP status");
    let headers = response.headers().clone();
    let body = response.bytes_stream().map(|chunk| {
        chunk.map_err(|error| {
            std::io::Error::other(format!("Control response stream failed: {error}"))
        })
    });
    let mut output = Response::new(Body::from_stream(body));
    *output.status_mut() = status;
    for (name, value) in &headers {
        if !is_hop_by_hop(name) {
            output.headers_mut().append(name.clone(), value.clone());
        }
    }
    output
}

fn control_unavailable(error: reqwest::Error) -> Response {
    tracing::warn!(%error, "A4N Control request failed");
    (
        StatusCode::BAD_GATEWAY,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        "A4N Control is unavailable",
    )
        .into_response()
}

fn is_hop_by_hop(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "host"
    )
}

fn parse_addr(name: &str, default: &str) -> anyhow::Result<SocketAddr> {
    parse_optional_addr(name)?.map_or_else(|| parse_addr_value(default), Ok)
}

fn parse_optional_addr(name: &str) -> anyhow::Result<Option<SocketAddr>> {
    let Some(value) = env::var_os(name) else {
        return Ok(None);
    };
    let value = value
        .into_string()
        .map_err(|_| anyhow::anyhow!("{name} 必须是有效 UTF-8 地址"))?;
    if value.trim().is_empty() {
        return Err(anyhow::anyhow!("{name} 不能为空"));
    }
    parse_addr_value(&value).map(Some)
}

fn parse_addr_value(value: &str) -> anyhow::Result<SocketAddr> {
    value
        .parse()
        .map_err(|error| anyhow::anyhow!("无效监听地址 {value}: {error}"))
}

async fn shutdown_signal(stop_requested: Arc<AtomicBool>) {
    let stop_flag = wait_for_stop_request(stop_requested);
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
            _ = stop_flag => {},
        }
    }
    #[cfg(not(unix))]
    {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = stop_flag => {},
        }
    }
    tracing::info!("A4N Edge shutting down");
}

async fn wait_for_stop_request(stop_requested: Arc<AtomicBool>) {
    while !stop_requested.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::{EdgeState, is_hop_by_hop, parse_addr_value, router};
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{HeaderName, Request, StatusCode, header},
        response::Response,
        routing::post,
    };
    use reqwest::Client;
    use std::path::PathBuf;

    #[test]
    fn parses_control_addresses_and_rejects_invalid_values() {
        assert_eq!(parse_addr_value("127.0.0.1:18080").unwrap().port(), 18080);
        assert!(parse_addr_value("not-an-address").is_err());
    }

    #[test]
    fn filters_hop_by_hop_headers_before_forwarding() {
        assert!(is_hop_by_hop(&HeaderName::from_static("connection")));
        assert!(is_hop_by_hop(&HeaderName::from_static("host")));
        assert!(!is_hop_by_hop(&HeaderName::from_static("authorization")));
    }

    #[tokio::test]
    async fn forwards_api_requests_and_streams_the_control_response() {
        async fn echo(request: Request<Body>) -> Response {
            let authorization = request
                .headers()
                .get(header::AUTHORIZATION)
                .cloned()
                .expect("proxy must forward authorization");
            let body = to_bytes(request.into_body(), 1024)
                .await
                .expect("echo body should be readable");
            Response::builder()
                .status(StatusCode::CREATED)
                .header(header::AUTHORIZATION, authorization)
                .body(Body::from(body))
                .unwrap()
        }

        let control_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let control_addr = control_listener.local_addr().unwrap();
        let control_task = tokio::spawn(async move {
            axum::serve(
                control_listener,
                Router::new().route("/api/echo", post(echo)),
            )
            .await
            .unwrap();
        });

        let edge_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let edge_addr = edge_listener.local_addr().unwrap();
        let edge_task = tokio::spawn(async move {
            axum::serve(
                edge_listener,
                router(
                    EdgeState {
                        client: Client::new(),
                        control_base_url: format!("http://{control_addr}"),
                    },
                    PathBuf::from("target/no-edge-assets"),
                ),
            )
            .await
            .unwrap();
        });

        let response = Client::new()
            .post(format!("http://{edge_addr}/api/echo"))
            .header(header::AUTHORIZATION, "Bearer test")
            .body("payload")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers()[header::AUTHORIZATION], "Bearer test");
        assert_eq!(response.text().await.unwrap(), "payload");

        edge_task.abort();
        control_task.abort();
        let _ = edge_task.await;
        let _ = control_task.await;
    }
}
