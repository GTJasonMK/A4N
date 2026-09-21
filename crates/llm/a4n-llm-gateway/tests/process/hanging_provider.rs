use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, header},
    response::IntoResponse,
    routing::post,
};
use futures::{future::pending, stream};
use serde_json::Value;
use tokio::{sync::oneshot, task::JoinHandle, time::timeout};

#[derive(Debug, Clone)]
pub(crate) struct HangingProviderRequest {
    pub(crate) authorization: Option<String>,
    pub(crate) body: Value,
}

type HangingProviderObservation = Arc<Mutex<Option<HangingProviderRequest>>>;

#[derive(Clone)]
struct HangingProviderState {
    observation: HangingProviderObservation,
    body_drop_signal: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

struct ProviderBodyDropSignal(Option<oneshot::Sender<()>>);

impl Drop for ProviderBodyDropSignal {
    fn drop(&mut self) {
        if let Some(signal) = self.0.take() {
            let _ = signal.send(());
        }
    }
}

pub(crate) struct HangingProviderServer {
    addr: SocketAddr,
    observation: HangingProviderObservation,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<Result<(), std::io::Error>>,
}

impl HangingProviderServer {
    pub(crate) async fn start() -> (Self, oneshot::Receiver<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("hanging provider should bind a loopback port");
        let addr = listener
            .local_addr()
            .expect("hanging provider should expose its local address");
        let observation = Arc::new(Mutex::new(None));
        let (body_drop_signal, body_drop_receiver) = oneshot::channel();
        let router = Router::new()
            .route("/chat/completions", post(hanging_chat_completions))
            .with_state(HangingProviderState {
                observation: observation.clone(),
                body_drop_signal: Arc::new(Mutex::new(Some(body_drop_signal))),
            });
        let (shutdown, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
        });

        (
            Self {
                addr,
                observation,
                shutdown,
                task,
            },
            body_drop_receiver,
        )
    }

    pub(crate) fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub(crate) fn request(&self) -> HangingProviderRequest {
        self.observation
            .lock()
            .expect("hanging provider observation lock should not be poisoned")
            .clone()
            .expect("gateway should start the hanging provider stream")
    }

    pub(crate) async fn stop(self) {
        let _ = self.shutdown.send(());
        timeout(Duration::from_secs(5), self.task)
            .await
            .expect("hanging provider should stop before timeout")
            .expect("hanging provider task should join")
            .expect("hanging provider should stop cleanly");
    }
}

async fn hanging_chat_completions(
    State(state): State<HangingProviderState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    *state
        .observation
        .lock()
        .expect("hanging provider observation lock should not be poisoned") =
        Some(HangingProviderRequest {
            authorization,
            body,
        });
    let body_drop_signal = state
        .body_drop_signal
        .lock()
        .expect("provider body drop signal lock should not be poisoned")
        .take()
        .expect("hanging provider should be called once");
    let body_stream = stream::unfold(
        (false, ProviderBodyDropSignal(Some(body_drop_signal))),
        |(first_sent, drop_signal)| async move {
            if first_sent {
                return pending::<
                    Option<(Result<Bytes, Infallible>, (bool, ProviderBodyDropSignal))>,
                >()
                .await;
            }
            Some((
                Ok(Bytes::from_static(
                    b"data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"first\"},\"finish_reason\":null}]}\n\n",
                )),
                (true, drop_signal),
            ))
        },
    );

    (
        [(header::CONTENT_TYPE, "text/event-stream")],
        Body::from_stream(body_stream),
    )
}
