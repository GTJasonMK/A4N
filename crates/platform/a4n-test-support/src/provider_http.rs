//! Real loopback HTTP transport used by adapter tests, including multipart bytes.
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::Response,
    routing::any,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub struct CapturedRequest {
    pub method: String,
    pub uri: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Clone)]
pub struct ProviderResponse {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
    pub stream_error: bool,
    pub delay_ms: u64,
}

impl ProviderResponse {
    pub fn json(status: u16, body: serde_json::Value) -> Self {
        Self {
            status,
            content_type: "application/json",
            body: body.to_string().into_bytes(),
            stream_error: false,
            delay_ms: 0,
        }
    }
    pub fn bytes(content_type: &'static str, body: Vec<u8>) -> Self {
        Self {
            status: 200,
            content_type,
            body,
            stream_error: false,
            delay_ms: 0,
        }
    }
}

type Handler = dyn Fn(&CapturedRequest) -> ProviderResponse + Send + Sync;

#[derive(Clone)]
struct ProviderState {
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    handler: Arc<Handler>,
}

pub struct LocalProvider {
    pub base_url: String,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    task: tokio::task::JoinHandle<()>,
}

impl LocalProvider {
    pub async fn start(
        handler: impl Fn(&CapturedRequest) -> ProviderResponse + Send + Sync + 'static,
    ) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let state = ProviderState {
            requests: requests.clone(),
            handler: Arc::new(handler),
        };
        let app = Router::new()
            .route("/", any(serve))
            .route("/{*path}", any(serve))
            .with_state(state);
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            base_url,
            requests,
            task,
        }
    }
    pub fn requests(&self) -> Vec<CapturedRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for LocalProvider {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve(State(state): State<ProviderState>, request: Request) -> Response<Body> {
    let (parts, body) = request.into_parts();
    let request = CapturedRequest {
        method: parts.method.to_string(),
        uri: parts.uri.to_string(),
        headers: parts
            .headers
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_str().unwrap().to_string()))
            .collect(),
        body: to_bytes(body, 32 * 1024 * 1024).await.unwrap().to_vec(),
    };
    state.requests.lock().unwrap().push(request.clone());
    let response = (state.handler)(&request);
    if response.delay_ms > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(response.delay_ms)).await;
    }
    let body = if response.stream_error {
        Body::from_stream(futures::stream::iter([
            Ok(response.body),
            Err(std::io::Error::other("injected disconnect")),
        ]))
    } else {
        Body::from(response.body)
    };
    Response::builder()
        .status(response.status)
        .header("content-type", response.content_type)
        .body(body)
        .unwrap()
}
