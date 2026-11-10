use std::{
    io::{BufRead, BufReader, Read, Write},
    net::Shutdown,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use a4n_domain::llm::{LLM_PROVIDER_CAPABILITIES, LlmProviderCapabilities};
use a4n_llm_provider_contract::{ChatRequest, LlmProvider, ModelCapabilities, collect_chat_stream};
use a4n_test_support::model_contract::assert_case;
use openssl::{
    asn1::Asn1Time,
    hash::MessageDigest,
    pkey::PKey,
    rsa::Rsa,
    ssl::{SslAcceptor, SslMethod},
    x509::{
        X509, X509NameBuilder,
        extension::{BasicConstraints, SubjectAlternativeName},
    },
};
use reqwest::Client;
use serde_json::{Value, json};

use super::{LlmProtocolKind, events};

static TLS: LazyLock<(SslAcceptor, Vec<u8>)> = LazyLock::new(test_tls_identity);

#[cfg(test)]
fn test_tls_identity() -> (SslAcceptor, Vec<u8>) {
    let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", "localhost").unwrap();
    let name = name.build();
    let mut certificate = X509::builder().unwrap();
    certificate.set_version(2).unwrap();
    certificate.set_subject_name(&name).unwrap();
    certificate.set_issuer_name(&name).unwrap();
    certificate.set_pubkey(&key).unwrap();
    certificate
        .set_not_before(&Asn1Time::days_from_now(0).unwrap())
        .unwrap();
    certificate
        .set_not_after(&Asn1Time::days_from_now(1).unwrap())
        .unwrap();
    certificate
        .append_extension(BasicConstraints::new().critical().ca().build().unwrap())
        .unwrap();
    let san = SubjectAlternativeName::new()
        .ip("127.0.0.1")
        .dns("localhost")
        .build(&certificate.x509v3_context(None, None))
        .unwrap();
    certificate.append_extension(san).unwrap();
    certificate.sign(&key, MessageDigest::sha256()).unwrap();
    let certificate = certificate.build();
    let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    acceptor.set_private_key(&key).unwrap();
    acceptor.set_certificate(&certificate).unwrap();
    acceptor.check_private_key().unwrap();
    (acceptor.build(), certificate.to_pem().unwrap())
}

struct TlsProvider {
    base_url: String,
    client: Client,
    connections: Arc<AtomicUsize>,
    requests: Arc<Mutex<Vec<Vec<u8>>>>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<()>,
}

impl TlsProvider {
    async fn start(failed_handshakes: usize, response: Option<Vec<u8>>) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("https://{}", listener.local_addr().unwrap());
        let client = Client::builder()
            .no_proxy()
            .add_root_certificate(reqwest::Certificate::from_pem(&TLS.1).unwrap())
            .connect_timeout(Duration::from_secs(1))
            .read_timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let connections = Arc::new(AtomicUsize::new(0));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (stop, mut stopped) = tokio::sync::oneshot::channel();
        let count = connections.clone();
        let captured = requests.clone();
        let task = tokio::spawn(async move {
            let mut workers = tokio::task::JoinSet::new();
            loop {
                let socket = tokio::select! {
                    _ = &mut stopped => break,
                    result = listener.accept() => result.unwrap().0,
                };
                let attempt = count.fetch_add(1, Ordering::SeqCst) + 1;
                let captured = captured.clone();
                let response = response.clone();
                workers.spawn_blocking(move || {
                    let mut socket = socket.into_std().unwrap();
                    socket.set_nonblocking(false).unwrap();
                    socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                    socket.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
                    if attempt <= failed_handshakes {
                        // Consume ClientHello then close the write half, producing TLS EOF before HTTP.
                        let mut hello = [0; 16_384];
                        let _ = socket.read(&mut hello);
                        socket.shutdown(Shutdown::Write).unwrap();
                        let _ = std::io::copy(&mut socket, &mut std::io::sink());
                        return;
                    }
                    let Ok(mut tls) = TLS.0.accept(socket) else { return; };
                    let mut reader = BufReader::new(&mut tls);
                    let mut request = Vec::new();
                    let mut length = None;
                    loop {
                        let mut line = String::new();
                        assert!(reader.read_line(&mut line).unwrap() > 0);
                        request.extend_from_slice(line.as_bytes());
                        if line == "\r\n" { break; }
                        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                            length = Some(value.trim().parse::<usize>().unwrap());
                        }
                        assert!(request.len() <= 16_384);
                    }
                    let length = length.expect("JSON request has a known body length");
                    assert!(length <= 1_048_576);
                    let mut body = vec![0; length];
                    reader.read_exact(&mut body).unwrap();
                    request.extend(body);
                    captured.lock().unwrap().push(request);
                    if let Some(body) = response {
                        write!(tls, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
                        tls.write_all(&body).unwrap();
                        tls.flush().unwrap();
                    }
                    let _ = tls.shutdown();
                });
            }
            while let Some(result) = workers.join_next().await {
                result.unwrap();
            }
        });
        Self {
            base_url,
            client,
            connections,
            requests,
            stop: Some(stop),
            task,
        }
    }

    async fn stop(mut self) {
        self.stop.take().unwrap().send(()).unwrap();
        (&mut self.task).await.unwrap();
    }
}

impl Drop for TlsProvider {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

fn provider(preset: &LlmProviderCapabilities, server: &TlsProvider) -> Box<dyn LlmProvider> {
    let caps = ModelCapabilities {
        max_context_tokens: 500_000,
        max_output_tokens: 20_000,
    };
    if preset.protocol == LlmProtocolKind::AnthropicMessages {
        Box::new(
            crate::AnthropicLlmProvider::new(
                &server.base_url,
                "connect-regression",
                Some("matrix-key".into()),
                2,
                None,
                caps,
            )
            .unwrap()
            .with_test_client(server.client.clone()),
        )
    } else {
        Box::new(
            crate::HttpLlmProvider::new_with_timeout_options_and_capabilities(
                &server.base_url,
                "connect-regression",
                Some("matrix-key".into()),
                2,
                None,
                caps,
            )
            .unwrap()
            .with_protocol(preset.protocol)
            .with_test_client(server.client.clone()),
        )
    }
}

fn request() -> ChatRequest {
    ChatRequest::system_user("original rules", "original source", None, Some(20_000))
}

#[tokio::test]
async fn model_contract_matrix_connect_recovery_preserves_request_and_budget() {
    for preset in LLM_PROVIDER_CAPABILITIES {
        let server = TlsProvider::start(2, Some(events(preset.protocol, "valid"))).await;
        let response = collect_chat_stream(provider(preset, &server).as_ref(), request())
            .await
            .unwrap();
        let captured = server.requests.lock().unwrap().clone();
        let connections = server.connections.load(Ordering::SeqCst);
        server.stop().await;
        assert_eq!(captured.len(), 1);
        let wire = String::from_utf8(captured[0].clone()).unwrap();
        let (_, body) = wire.split_once("\r\n\r\n").unwrap();
        let body: Value = serde_json::from_str(body).unwrap();
        let budget = match preset.protocol {
            LlmProtocolKind::OpenAiChatCompletions | LlmProtocolKind::AnthropicMessages => {
                &body["max_tokens"]
            }
            LlmProtocolKind::OpenAiResponses => &body["max_output_tokens"],
            LlmProtocolKind::GeminiGenerateContent => &body["generationConfig"]["maxOutputTokens"],
        };
        assert_eq!(budget, 20_000);
        assert!(
            wire.contains("original rules")
                && wire.contains("original source")
                && wire.contains("matrix-key")
        );
        assert_case(
            "llm.transport",
            preset.id,
            "json",
            "connect-recovery",
            json!({"connections":3,"http_requests":1,"content":{"id":1,"exit_state":{}}}),
            json!({"connections":connections,"http_requests":captured.len(),"content":serde_json::from_str::<Value>(&response.message.content).unwrap()}),
            json!({"max_output_tokens":budget}),
        );
    }
}

#[tokio::test]
async fn model_contract_matrix_connect_exhaustion_is_bounded_and_explicit() {
    for preset in LLM_PROVIDER_CAPABILITIES {
        let server = TlsProvider::start(usize::MAX, None).await;
        let error = provider(preset, &server)
            .chat_stream(request())
            .await
            .err()
            .expect("persistent EOF must fail");
        let connections = server.connections.load(Ordering::SeqCst);
        let requests = server.requests.lock().unwrap().len();
        server.stop().await;
        assert!(error.to_string().contains("unexpected EOF"), "{error}");
        assert_case(
            "llm.transport",
            preset.id,
            "json",
            "connect-exhausted",
            json!({"connections":3,"http_requests":0}),
            json!({"connections":connections,"http_requests":requests}),
            json!({"error":error.to_string()}),
        );
    }
}

#[tokio::test]
async fn model_contract_matrix_accepted_request_eof_is_never_replayed() {
    for preset in LLM_PROVIDER_CAPABILITIES {
        let server = TlsProvider::start(0, None).await;
        let error = provider(preset, &server)
            .chat_stream(request())
            .await
            .err()
            .expect("accepted request EOF must fail");
        let connections = server.connections.load(Ordering::SeqCst);
        let requests = server.requests.lock().unwrap().len();
        server.stop().await;
        assert_case(
            "llm.transport",
            preset.id,
            "json",
            "accepted-request-eof",
            json!({"connections":1,"http_requests":1}),
            json!({"connections":connections,"http_requests":requests}),
            json!({"error":error.to_string()}),
        );
    }
}

#[tokio::test]
async fn model_contract_matrix_cancellation_stops_pending_connection_retries() {
    for preset in LLM_PROVIDER_CAPABILITIES {
        let server = TlsProvider::start(usize::MAX, None).await;
        let provider = provider(preset, &server);
        let task = tokio::spawn(async move { provider.chat_stream(request()).await });
        tokio::time::timeout(Duration::from_secs(1), async {
            while server.connections.load(Ordering::SeqCst) == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        task.abort();
        assert!(task.await.err().unwrap().is_cancelled());
        tokio::time::sleep(Duration::from_millis(150)).await;
        let connections = server.connections.load(Ordering::SeqCst);
        let requests = server.requests.lock().unwrap().len();
        server.stop().await;
        assert_case(
            "llm.transport",
            preset.id,
            "json",
            "connect-cancellation",
            json!({"connections":1,"http_requests":0}),
            json!({"connections":connections,"http_requests":requests}),
            json!({"caller_task_cancelled":true,"backoff_window_observed_ms":150}),
        );
    }
}

#[tokio::test]
async fn model_contract_matrix_connection_retry_keeps_tls_verification_and_connect_deadline() {
    let server = TlsProvider::start(0, None).await;
    let client = Client::builder().no_proxy().build().unwrap();
    assert!(
        crate::transport::send(client.post(&server.base_url).json(&json!({})))
            .await
            .is_err()
    );
    assert!(server.requests.lock().unwrap().is_empty());
    server.stop().await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("https://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (_socket, _) = listener.accept().await.unwrap();
        std::future::pending::<()>().await;
    });
    let client = Client::builder()
        .no_proxy()
        .connect_timeout(Duration::from_millis(50))
        .build()
        .unwrap();
    let started = std::time::Instant::now();
    let error = crate::transport::send(client.post(url).json(&json!({})))
        .await
        .unwrap_err();
    assert!(error.is_timeout() && error.is_connect(), "{error}");
    assert!(started.elapsed() < Duration::from_millis(300));
    server.abort();
    assert!(server.await.err().unwrap().is_cancelled());
}
