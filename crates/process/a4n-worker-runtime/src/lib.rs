use std::future::Future;

use a4n_worker_contract::{WorkerEnvelope, WorkerFailure, WorkerResponse};
use axum::{
    Json,
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use subtle::ConstantTimeEq;

#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;

pub fn worker_token_authorized(headers: &HeaderMap, header_name: &str, token: &[u8]) -> bool {
    headers
        .get(header_name)
        .is_some_and(|value| bool::from(value.as_bytes().ct_eq(token)))
}

pub async fn worker_command_response<T, O, F, Fut, U>(
    authorized: bool,
    request: WorkerEnvelope<T>,
    command: F,
    unauthorized: U,
) -> Response
where
    T: Serialize + Clone,
    O: Serialize,
    F: FnOnce(WorkerEnvelope<T>) -> Fut,
    Fut: Future<Output = Result<O, WorkerFailure>>,
    U: FnOnce() -> Response,
{
    if !authorized {
        return unauthorized();
    }

    let response = match command(request.clone()).await {
        Ok(payload) => WorkerResponse::completed_for(&request, payload),
        Err(error) => WorkerResponse::<O>::failed_for(&request, error.into()),
    };
    Json(response).into_response()
}

pub fn init_worker_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

/// Wait for Ctrl-C or SIGTERM before beginning graceful process shutdown.
pub fn wait_for_shutdown_signal() -> impl Future<Output = ()> + Send {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        async move {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = terminate.recv() => {},
            }
        }
    }
    #[cfg(not(unix))]
    {
        async {
            let _ = tokio::signal::ctrl_c().await;
        }
    }
}

#[cfg(target_os = "linux")]
/// Deliver SIGTERM to a child process when its parent exits.
pub fn configure_parent_death_signal(command: &mut std::process::Command) {
    unsafe {
        command.pre_exec(|| {
            unsafe extern "C" {
                fn prctl(option: i32, ...) -> i32;
            }
            const PR_SET_PDEATHSIG: i32 = 1;
            const SIGTERM: i32 = 15;
            if prctl(PR_SET_PDEATHSIG, SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(target_os = "linux"))]
pub fn configure_parent_death_signal(_: &mut std::process::Command) {}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::io::{BufRead, BufReader};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    #[cfg(unix)]
    use std::{
        process::{Command, Stdio},
        thread,
        time::{Duration, Instant},
    };

    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerErrorCode, WorkerLease, WorkerOperationStatus,
        WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
    };
    use axum::{
        body::to_bytes,
        http::{HeaderMap, HeaderValue, StatusCode},
    };
    use serde_json::Value;

    use super::*;

    #[cfg(unix)]
    const SHUTDOWN_SIGNAL_CHILD: &str = "A4N_WORKER_RUNTIME_SIGTERM_TEST_CHILD";

    #[cfg(unix)]
    #[test]
    fn wait_for_shutdown_signal_handles_sigterm() {
        let executable = std::env::current_exe().expect("test executable path");
        let mut child = Command::new(executable)
            .args([
                "--exact",
                "tests::wait_for_shutdown_signal_child",
                "--nocapture",
            ])
            .env(SHUTDOWN_SIGNAL_CHILD, "1")
            .stdout(Stdio::piped())
            .spawn()
            .expect("shutdown signal child starts");
        let stdout = child.stdout.take().expect("child stdout is piped");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = reader.read_line(&mut line).expect("read child readiness");
            assert_ne!(bytes, 0, "shutdown signal child exited before readiness");
            if line.trim() == "READY" {
                break;
            }
        }

        let signal = Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status()
            .expect("send SIGTERM to shutdown signal child");
        if !signal.success() {
            let _ = child.kill();
            let _ = child.wait();
            panic!("SIGTERM delivery failed: {signal}");
        }

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = child.try_wait().expect("check shutdown child status") {
                assert!(status.success(), "shutdown child failed: {status}");
                return;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("shutdown signal waiter did not exit after SIGTERM");
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wait_for_shutdown_signal_child() {
        if std::env::var_os(SHUTDOWN_SIGNAL_CHILD).is_none() {
            return;
        }
        let shutdown = wait_for_shutdown_signal();
        println!("READY");
        shutdown.await;
    }

    #[test]
    fn worker_token_authorization_compares_raw_header_bytes() {
        let header_name = "x-a4n-worker-token";
        let token = "密钥".as_bytes();
        let mut headers = HeaderMap::new();

        assert!(!worker_token_authorized(&headers, header_name, token));
        headers.insert(
            header_name,
            HeaderValue::from_bytes(token).expect("header accepts opaque token bytes"),
        );
        assert!(worker_token_authorized(&headers, header_name, token));
        assert!(!worker_token_authorized(&headers, header_name, b"other"));
    }

    #[tokio::test]
    async fn worker_command_response_preserves_auth_and_wire_semantics() {
        let request = valid_request();
        let called = Arc::new(AtomicBool::new(false));
        let command_called = Arc::clone(&called);
        let unauthorized_response = worker_command_response(
            false,
            request.clone(),
            move |_| async move {
                command_called.store(true, Ordering::SeqCst);
                Ok::<_, WorkerFailure>(serde_json::json!({"ok": true}))
            },
            || StatusCode::UNAUTHORIZED.into_response(),
        )
        .await;
        assert_eq!(unauthorized_response.status(), StatusCode::UNAUTHORIZED);
        assert!(!called.load(Ordering::SeqCst));

        let completed = worker_command_response(
            true,
            request.clone(),
            |_| async { Ok::<_, WorkerFailure>(serde_json::json!({"ok": true})) },
            || StatusCode::UNAUTHORIZED.into_response(),
        )
        .await;
        assert_eq!(completed.status(), StatusCode::OK);
        let completed: WorkerResponse<Value> = serde_json::from_slice(
            &to_bytes(completed.into_body(), usize::MAX)
                .await
                .expect("completed body"),
        )
        .expect("completed response JSON");
        assert_eq!(completed.request_id, request.request_id);
        assert_eq!(completed.trace_id, request.trace_id);
        assert_eq!(completed.operation.operation_id, request.operation_id);
        assert_eq!(completed.status, WorkerOperationStatus::Completed);
        assert_eq!(completed.payload, Some(serde_json::json!({"ok": true})));

        let failed = worker_command_response::<_, Value, _, _, _>(
            true,
            request.clone(),
            |_| async { Err(WorkerFailure::unavailable("database unavailable")) },
            || StatusCode::UNAUTHORIZED.into_response(),
        )
        .await;
        assert_eq!(failed.status(), StatusCode::OK);
        let failed: WorkerResponse<Value> = serde_json::from_slice(
            &to_bytes(failed.into_body(), usize::MAX)
                .await
                .expect("failed body"),
        )
        .expect("failed response JSON");
        assert_eq!(failed.request_id, request.request_id);
        assert_eq!(failed.trace_id, request.trace_id);
        assert_eq!(failed.operation.operation_id, request.operation_id);
        assert_eq!(failed.status, WorkerOperationStatus::Failed);
        assert_eq!(failed.payload, None);
        assert_eq!(
            failed.error.as_ref().map(|error| error.code),
            Some(WorkerErrorCode::Unavailable)
        );
    }

    fn valid_request() -> WorkerEnvelope<Value> {
        WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request-1".into(),
            trace_id: "trace-1".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: "worker.read".into(),
            idempotency_key: "idem-1".into(),
            project_id: Some(7),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease::control_request("test-worker"),
            payload: Value::Null,
        }
    }
}
