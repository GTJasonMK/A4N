use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Output};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tempfile::TempDir;

use a4n_api_contract::command_manifest::{API_COMMANDS, COMMAND_MANIFEST_VERSION};

fn cli(temp: &TempDir, config: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_a4n"));
    command
        .current_dir(temp.path())
        .args(["--config"])
        .arg(config)
        .arg("--agent");
    command
}

fn write_json(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

fn write_config(path: &Path, server: &str) {
    write_json(
        path,
        &json!({
            "server": server,
            "access_token": "test-token"
        }),
    );
}

fn agent_error(output: &Output) -> Value {
    serde_json::from_slice(&output.stderr).unwrap_or_else(|error| {
        panic!(
            "invalid agent stderr: {error}; stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn spawn_reply(
    status: &'static str,
    body: impl Into<Vec<u8>>,
    declared_length: Option<usize>,
) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let body = body.into();
    let handle = thread::spawn(move || {
        reply_once(
            &listener,
            status,
            "application/json",
            &body,
            declared_length,
        )
    });
    (format!("http://{address}"), handle)
}

fn spawn_json_replies(bodies: Vec<Value>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        bodies
            .into_iter()
            .map(|body| {
                let body = serde_json::to_vec(&body).unwrap();
                reply_once(&listener, "200 OK", "application/json", &body, None)
            })
            .collect()
    });
    (format!("http://{address}"), handle)
}

fn reply_once(
    listener: &TcpListener,
    status: &str,
    content_type: &str,
    body: &[u8],
    declared_length: Option<usize>,
) -> String {
    let mut stream = accept_with_timeout(listener);
    let request = read_request(&mut stream);
    let length = declared_length.unwrap_or(body.len());
    write!(
        stream,
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {length}\r\nconnection: close\r\n\r\n"
    )
    .unwrap();
    stream.write_all(body).unwrap();
    stream.shutdown(Shutdown::Both).unwrap();
    request
}

fn accept_with_timeout(listener: &TcpListener) -> TcpStream {
    listener.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((stream, _)) => return stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "mock Server 未收到请求");
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("mock Server accept 失败: {error}"),
        }
    }
}

fn read_request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let read = stream.read(&mut buffer).unwrap();
        assert!(read > 0, "request ended before headers");
        bytes.extend_from_slice(&buffer[..read]);
        let Some(header_end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") else {
            continue;
        };
        let header_end = header_end + 4;
        let headers = String::from_utf8_lossy(&bytes[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .unwrap_or_default();
        while bytes.len() < header_end + content_length {
            let read = stream.read(&mut buffer).unwrap();
            assert!(read > 0, "request ended before body");
            bytes.extend_from_slice(&buffer[..read]);
        }
        return String::from_utf8(bytes).unwrap();
    }
}

#[test]
fn agent_capabilities_and_safety_gates_work_outside_repository() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    write_config(&config, "http://127.0.0.1:9");

    let output = cli(&temp, &config)
        .args(["capabilities", "export"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["data"]["total_commands"], API_COMMANDS.len());

    let output = cli(&temp, &config).arg("shell").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(agent_error(&output)["error"]["code"], "invalid_input");

    let output = cli(&temp, &config)
        .args(["call", "delete_project", "--args", r#"{"projectId":1}"#])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(10));
    assert_eq!(
        agent_error(&output)["error"]["code"],
        "destructive_confirmation_required"
    );

    for args in [
        ["task", "cancel", "task-1", "--attempt", "1"].as_slice(),
        ["auth", "revoke-session", "session-1"].as_slice(),
    ] {
        let output = cli(&temp, &config).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(10));
        assert_eq!(
            agent_error(&output)["error"]["code"],
            "destructive_confirmation_required"
        );
    }

    let output = cli(&temp, &config)
        .args(["call", "future_unknown_command", "--args", "{}"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        agent_error(&output)["error"]["message"]
            .as_str()
            .unwrap()
            .contains("未知 Host API 命令")
    );

    let (server, server_thread) = spawn_reply(
        "200 OK",
        serde_json::to_vec(&json!({
            "manifest_version": COMMAND_MANIFEST_VERSION,
            "total_commands": API_COMMANDS.len(),
            "matched_commands": API_COMMANDS.len(),
            "commands": [],
            "runtime_equivalents": [],
        }))
        .unwrap(),
        None,
    );
    write_config(&config, &server);
    let output = cli(&temp, &config)
        .args(["capabilities", "check"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(11));
    assert_eq!(
        agent_error(&output)["error"]["code"],
        "contract_incompatible"
    );
    assert!(
        server_thread
            .join()
            .unwrap()
            .starts_with("GET /api/meta/commands ")
    );
}

#[test]
fn connection_failures_use_the_network_exit_contract() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let server = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    write_config(&config, &server);

    let output = cli(&temp, &config)
        .args(["call", "list_projects", "--args", "{}"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(agent_error(&output)["error"]["code"], "network_connect");
}

#[test]
fn health_rejects_a_non_a4n_server() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_reply(
        "200 OK",
        serde_json::to_vec(&json!({ "host": "other-service" })).unwrap(),
        None,
    );
    write_config(&config, &server);

    let output = cli(&temp, &config).arg("health").output().unwrap();
    assert_eq!(output.status.code(), Some(8));
    assert_eq!(
        agent_error(&output)["error"]["code"],
        "server_identity_invalid"
    );
    assert!(server_thread.join().unwrap().starts_with("GET /api/meta "));
}

#[test]
fn flow_runs_captures_and_rejects_changed_source_on_resume() {
    let temp = TempDir::new().unwrap();
    let (server, server_thread) = spawn_reply(
        "200 OK",
        br#"{"projects":[{"id":42,"title":"demo"}]}"#,
        None,
    );
    let config = temp.path().join("cli.json");
    let flow = temp.path().join("flow.json");
    let journal = temp.path().join("journal.json");
    write_config(&config, &server);
    write_json(
        &flow,
        &json!({
            "version": 1,
            "name": "project-list",
            "steps": [{
                "id": "list",
                "type": "call",
                "command": "list_projects",
                "args": {},
                "capture": {"project_id": "/projects/0/id"}
            }]
        }),
    );

    let output = cli(&temp, &config)
        .args(["flow", "run"])
        .arg(&flow)
        .args(["--journal"])
        .arg(&journal)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let _: Value = serde_json::from_str(line).unwrap();
    }
    let request = server_thread.join().unwrap();
    assert!(request.starts_with("GET /api/projects HTTP/1.1"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer test-token")
    );

    let saved: Value = serde_json::from_slice(&fs::read(&journal).unwrap()).unwrap();
    assert_eq!(saved["status"], "completed");
    assert_eq!(saved["steps"][0]["result"]["project_id"], 42);
    assert!(!String::from_utf8_lossy(&fs::read(&journal).unwrap()).contains("test-token"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&journal).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    let output = cli(&temp, &config)
        .args(["flow", "resume"])
        .arg(&journal)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("already_completed"));

    let mut changed: Value = serde_json::from_slice(&fs::read(&flow).unwrap()).unwrap();
    changed["description"] = json!("changed");
    write_json(&flow, &changed);
    let output = cli(&temp, &config)
        .args(["flow", "resume"])
        .arg(&journal)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        agent_error(&output)["error"]["message"]
            .as_str()
            .unwrap()
            .contains("flow 文件在 journal 创建后已变化")
    );
}

#[test]
fn flow_redacts_server_echoed_secrets_and_blocks_unsafe_resume() {
    let temp = TempDir::new().unwrap();
    let secret = "agent-secret-value";
    let response = format!(
        r#"{{"code":"invalid_config","message":"rejected {secret}","details":{{"api_key":"{secret}"}}}}"#
    );
    let (server, server_thread) = spawn_reply("400 Bad Request", response.into_bytes(), None);
    let config = temp.path().join("cli.json");
    let flow = temp.path().join("secret-flow.json");
    let journal = temp.path().join("secret-journal.json");
    write_config(&config, &server);
    write_json(
        &flow,
        &json!({
            "version": 1,
            "name": "secret-error",
            "steps": [{
                "id": "config",
                "type": "call",
                "command": "create_llm_config",
                "args": {"input": {"api_key": "${env.A4N_E2E_SECRET}"}}
            }]
        }),
    );

    let output = cli(&temp, &config)
        .env("A4N_E2E_SECRET", secret)
        .args(["flow", "run"])
        .arg(&flow)
        .args(["--journal"])
        .arg(&journal)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(8));
    let request = server_thread.join().unwrap();
    assert!(request.contains(secret));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let journal_text = String::from_utf8_lossy(&fs::read(&journal).unwrap()).into_owned();
    assert!(!stderr.contains(secret));
    assert!(stderr.contains("[REDACTED]"));
    assert!(!journal_text.contains(secret));
    assert!(journal_text.contains("查看本次命令的结构化错误"));

    let dangerous_flow = temp.path().join("dangerous-flow.json");
    let dangerous_journal = temp.path().join("dangerous-journal.json");
    write_json(
        &dangerous_flow,
        &json!({
            "version": 1,
            "name": "dangerous",
            "steps": [{
                "id": "delete",
                "type": "call",
                "command": "delete_project",
                "args": {"projectId": 1},
                "allow_destructive": true
            }]
        }),
    );
    let output = cli(&temp, &config)
        .args(["flow", "run"])
        .arg(&dangerous_flow)
        .args(["--journal"])
        .arg(&dangerous_journal)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(10));

    let output = cli(&temp, &config)
        .args(["flow", "resume"])
        .arg(&dangerous_journal)
        .arg("--allow-destructive")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(10));
    assert!(
        agent_error(&output)["error"]["message"]
            .as_str()
            .unwrap()
            .contains("--rerun-incomplete")
    );

    let (server, server_thread) = spawn_reply("200 OK", br#"{"deleted":true}"#, None);
    write_config(&config, &server);
    let output = cli(&temp, &config)
        .args(["flow", "resume"])
        .arg(&dangerous_journal)
        .args(["--allow-destructive", "--rerun-incomplete"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        server_thread
            .join()
            .unwrap()
            .starts_with("POST /api/commands/delete_project ")
    );
    let saved: Value = serde_json::from_slice(&fs::read(&dangerous_journal).unwrap()).unwrap();
    assert_eq!(saved["status"], "completed");
}

#[test]
fn failed_forced_download_preserves_existing_file() {
    let temp = TempDir::new().unwrap();
    let (server, server_thread) = spawn_reply("200 OK", b"new", Some(100));
    let config = temp.path().join("cli.json");
    let output_path = temp.path().join("artifact.bin");
    write_config(&config, &server);
    fs::write(&output_path, b"old").unwrap();

    let output = cli(&temp, &config)
        .args(["media", "download", "artifact.bin", "--output"])
        .arg(&output_path)
        .arg("--force")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let request = server_thread.join().unwrap();
    assert!(request.starts_with("GET /api/media?path=artifact.bin HTTP/1.1"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("\r\nauthorization: bearer test-token\r\n")
    );
    assert!(!request.contains("access_token"));
    assert_eq!(fs::read(&output_path).unwrap(), b"old");
}

#[test]
fn task_and_workflow_terminal_states_use_stable_exit_codes() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");

    for (kind, expected_code, expected_error) in [
        ("failed", 8, "task_failed"),
        ("cancelled", 9, "task_cancelled"),
    ] {
        let body = format!("data: {{\"sequence\":1,\"event\":{{\"kind\":\"{kind}\"}}}}\n\n");
        let (server, server_thread) = spawn_reply("200 OK", body.into_bytes(), None);
        write_config(&config, &server);
        let output = cli(&temp, &config)
            .args(["task", "watch", "task-1"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(expected_code));
        assert_eq!(agent_error(&output)["error"]["code"], expected_error);
        assert!(
            server_thread
                .join()
                .unwrap()
                .starts_with("GET /api/generation-tasks/task-1/events?attempt=1 ")
        );
    }

    let user = json!({
        "id": 7,
        "username": "agent",
        "email": "agent@example.test",
        "role": "user",
        "oauth_provider": null,
        "oauth_subject": null,
        "avatar_url": null,
        "is_active": true,
        "created_at": null,
        "updated_at": null,
        "last_login_at": null
    });
    for (status, expected_code, expected_error) in [
        ("failed", 8, "workflow_failed"),
        ("cancelled", 9, "workflow_cancelled"),
        ("interrupted", 9, "workflow_interrupted"),
        ("superseded", 9, "workflow_superseded"),
        ("mystery", 8, "workflow_contract_invalid"),
    ] {
        let (server, server_thread) = spawn_json_replies(vec![
            user.clone(),
            json!([]),
            json!({"run": {"status": status}}),
        ]);
        write_config(&config, &server);
        let output = cli(&temp, &config)
            .args(["workflow", "watch", "31", "--interval-ms", "200"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(expected_code));
        assert_eq!(agent_error(&output)["error"]["code"], expected_error);
        let requests = server_thread.join().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[0].starts_with("GET /api/auth/me "));
        assert!(requests[1].starts_with("POST /api/commands/get_workflow_progress_events "));
        assert!(requests[2].starts_with("POST /api/commands/get_workflow_run_detail "));
    }

    let (server, server_thread) = spawn_json_replies(vec![user, json!({"unexpected": "object"})]);
    write_config(&config, &server);
    let output = cli(&temp, &config)
        .args(["workflow", "watch", "31", "--interval-ms", "200"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(8));
    assert_eq!(
        agent_error(&output)["error"]["code"],
        "workflow_contract_invalid"
    );
    assert_eq!(server_thread.join().unwrap().len(), 2);
}
