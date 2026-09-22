use anyhow::Context;
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
    path::{Path, PathBuf},
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
    let configured_addr = parse_optional_addr("A4N_WEB_ADDR")?;
    let address_file = env::var_os("A4N_WEB_LISTEN_FILE").map(PathBuf::from);
    let bind_addr = configured_addr
        .unwrap_or_else(|| parse_addr_value(DEFAULT_WEB_ADDR).expect("valid default address"));
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
    let listener = bind_web_listener(
        bind_addr,
        configured_addr.is_none() && address_file.is_some(),
        address_file.as_deref(),
    )
    .await?;
    let bind_addr = listener.local_addr()?;
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

async fn bind_web_listener(
    preferred: SocketAddr,
    automatic: bool,
    address_file: Option<&Path>,
) -> anyhow::Result<tokio::net::TcpListener> {
    let saved_address = match address_file.map(std::fs::read_to_string) {
        Some(Ok(value)) => {
            let address = parse_addr_value(value.trim())?;
            anyhow::ensure!(
                address.port() != 0
                    && address.ip() == preferred.ip()
                    && (automatic || preferred.port() == 0 || address == preferred),
                "已发布的 HTTP 监听地址与启动配置不一致: {address}"
            );
            Some(address)
        }
        Some(Err(error)) if error.kind() == std::io::ErrorKind::NotFound => None,
        Some(Err(error)) => return Err(error).context("读取 HTTP 监听地址失败"),
        None => None,
    };
    // A restarted Edge must keep the endpoint already handed to its clients.
    let address = saved_address.unwrap_or(preferred);
    let listener = match tokio::net::TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(error)
            if automatic
                && saved_address.is_none()
                && error.kind() == std::io::ErrorKind::AddrInUse =>
        {
            let listener =
                tokio::net::TcpListener::bind(SocketAddr::new(preferred.ip(), 0)).await?;
            tracing::info!(
                requested = %preferred,
                selected = %listener.local_addr()?,
                "默认 HTTP 端口被占用，已选择空闲端口"
            );
            listener
        }
        Err(error) => return Err(error).with_context(|| format!("监听 HTTP 地址 {address} 失败")),
    };
    if let Some(path) = address_file {
        // Publish only after binding, atomically, so readers never see a partial address.
        let pending = path.with_extension("pending");
        std::fs::write(&pending, listener.local_addr()?.to_string())
            .context("写入 HTTP 监听地址失败")?;
        std::fs::rename(&pending, path).context("发布 HTTP 监听地址失败")?;
    }
    Ok(listener)
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
    use super::{EdgeState, bind_web_listener, is_hop_by_hop, parse_addr_value, router};
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

    #[tokio::test]
    async fn managed_listener_publishes_a_bound_port_and_preserves_it_on_restart() {
        let directory = tempfile::tempdir().unwrap();
        let address_file = directory.path().join("http.addr");
        let occupied = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let preferred = occupied.local_addr().unwrap();
        let listener = bind_web_listener(preferred, true, Some(&address_file))
            .await
            .unwrap();
        let selected = listener.local_addr().unwrap();
        assert_ne!(selected, preferred);
        assert_ne!(selected.port(), 0);
        assert_eq!(
            std::fs::read_to_string(&address_file).unwrap(),
            selected.to_string()
        );
        assert!(tokio::net::TcpListener::bind(selected).await.is_err());

        drop(listener);
        drop(occupied);
        let restarted = bind_web_listener(preferred, true, Some(&address_file))
            .await
            .unwrap();
        assert_eq!(restarted.local_addr().unwrap(), selected);
        assert!(
            bind_web_listener(preferred, true, Some(&address_file))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn fixed_ports_and_invalid_address_records_fail_without_selecting_another_port() {
        let directory = tempfile::tempdir().unwrap();
        let address_file = directory.path().join("http.addr");
        let occupied = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let preferred = occupied.local_addr().unwrap();
        assert!(
            bind_web_listener(preferred, false, Some(&address_file))
                .await
                .is_err()
        );
        assert!(!address_file.exists());
        for invalid in ["not-an-address", "127.0.0.1:0", "0.0.0.0:8080"] {
            std::fs::write(&address_file, invalid).unwrap();
            assert!(
                bind_web_listener(preferred, true, Some(&address_file))
                    .await
                    .is_err()
            );
            assert_eq!(std::fs::read_to_string(&address_file).unwrap(), invalid);
        }
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
