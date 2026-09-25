use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Output};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tempfile::TempDir;

use a4n_api_contract::command_manifest::{
    API_COMMANDS, COMMAND_MANIFEST_VERSION, HOST_IDENTITY, command_descriptor, is_destructive,
};

fn cli(temp: &TempDir, config: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_a4n"));
    command
        .current_dir(temp.path())
        .args(["--config"])
        .arg(config)
        .args(["--output", "agent-json", "--no-input"]);
    command
}

fn agent_cli(temp: &TempDir, config: &Path) -> Command {
    let mut command = cli(temp, config);
    command.arg("--agent");
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

fn spawn_optional_reply(body: Value) -> (String, thread::JoinHandle<Option<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let body = serde_json::to_vec(&body).unwrap();
    let handle = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return None;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("mock Server accept 失败: {error}"),
            }
        };
        let request = read_request(&mut stream);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(&body).unwrap();
        stream.shutdown(Shutdown::Both).unwrap();
        Some(request)
    });
    (format!("http://{address}"), handle)
}

fn spawn_tcp_accept() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let _stream = accept_with_timeout(&listener);
    });
    (address.to_string(), handle)
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

struct MockResponse {
    status: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
    declared_length: Option<usize>,
}

fn json_response(value: Value) -> MockResponse {
    MockResponse {
        status: "200 OK",
        content_type: "application/json",
        body: serde_json::to_vec(&value).unwrap(),
        declared_length: None,
    }
}

fn sse_response(body: impl Into<Vec<u8>>) -> MockResponse {
    MockResponse {
        status: "200 OK",
        content_type: "text/event-stream",
        body: body.into(),
        declared_length: None,
    }
}

fn binary_response(body: impl Into<Vec<u8>>) -> MockResponse {
    MockResponse {
        status: "200 OK",
        content_type: "application/octet-stream",
        body: body.into(),
        declared_length: None,
    }
}

fn spawn_scripted_replies(replies: Vec<MockResponse>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        replies
            .into_iter()
            .map(|reply| {
                reply_once_with_content_type(
                    &listener,
                    reply.status,
                    reply.content_type,
                    &reply.body,
                    reply.declared_length,
                )
            })
            .collect()
    });
    (format!("http://{address}"), handle)
}

fn spawn_chunked_response(
    content_type: &'static str,
    chunks: Vec<Vec<u8>>,
) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let body_length = chunks.iter().map(Vec::len).sum::<usize>();
    let handle = thread::spawn(move || {
        let mut stream = accept_with_timeout(&listener);
        let request = read_request(&mut stream);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\ncontent-length: {body_length}\r\nconnection: close\r\n\r\n"
        )
        .unwrap();
        for chunk in chunks {
            stream.write_all(&chunk).unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(5));
        }
        stream.shutdown(Shutdown::Both).unwrap();
        request
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
    reply_once_with_content_type(listener, status, content_type, body, declared_length)
}

fn reply_once_with_content_type(
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

fn request_json_body(request: &str) -> Value {
    let (_, body) = request.split_once("\r\n\r\n").unwrap();
    serde_json::from_str(body).unwrap()
}

fn success_data(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "CLI 失败: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "invalid agent stdout: {error}; stdout={}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    assert_eq!(value["ok"], true);
    value["data"].clone()
}

fn output_documents(output: &Output) -> Vec<Value> {
    output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect()
}

fn user_value() -> Value {
    json!({
        "id": 42,
        "username": "tester",
        "email": "tester@example.test",
        "role": "user",
        "oauth_provider": null,
        "oauth_subject": null,
        "avatar_url": null,
        "page_cli_enabled": false,
        "is_active": true,
        "created_at": null,
        "updated_at": null,
        "last_login_at": null
    })
}

fn auth_value(access_token: &str, refresh_token: &str, session_id: &str) -> Value {
    json!({
        "user": user_value(),
        "tokens": {
            "access_token": access_token,
            "refresh_token": refresh_token,
            "expires_in": 3600,
            "token_type": "Bearer",
            "session_id": session_id
        },
        "session": {
            "id": session_id,
            "user_agent": "a4n-cli-test",
            "created_at": "2026-08-22T00:00:00Z",
            "expires_at": "2026-08-23T00:00:00Z",
            "last_used_at": null
        }
    })
}

fn page_session_value(
    status: &str,
    agent_id: Option<&str>,
    operation_id: Option<&str>,
    context_revision: u64,
) -> Value {
    json!({
        "protocol_version": 1,
        "ui_session_id": "ui-session-1",
        "mode": "page",
        "status": status,
        "client": "a4n-web",
        "tab_id": "tab-1",
        "page_kind": "novel-detail",
        "route": "/novel/42",
        "project_id": 42,
        "chapter_id": 7,
        "active_tab": "overview",
        "selected_ids": {},
        "dirty_state": false,
        "local_state": {},
        "context_revision": context_revision,
        "agent_id": agent_id,
        "operation_id": operation_id,
        "last_operation": null,
        "created_at": "2026-08-23T00:00:00Z",
        "updated_at": "2026-08-23T00:00:01Z",
        "last_seen_at": "2026-08-23T00:00:01Z",
        "expires_at": "2026-08-23T01:00:00Z"
    })
}

fn page_operation_value(status: &str, target: Value, result: Value) -> Value {
    json!({
        "operation_id": "operation-1",
        "ui_session_id": "ui-session-1",
        "agent_id": "agent-1",
        "command": "list_projects",
        "page_kind": "novel-detail",
        "phase": "Page CLI 调用 list_projects",
        "target": target,
        "status": status,
        "progress": (status == "succeeded").then_some(100),
        "summary": null,
        "result": result,
        "error": null,
        "started_at": "2026-08-23T00:00:01Z",
        "updated_at": "2026-08-23T00:00:02Z"
    })
}

fn page_manifest_value(command: &str, available: bool, reason: Option<&str>) -> Value {
    json!({
        "protocol_version": 1,
        "ui_session_id": "ui-session-1",
        "page_kind": "novel-detail",
        "route": "/novel/42",
        "commands": [{
            "descriptor": serde_json::to_value(command_descriptor(command).unwrap()).unwrap(),
            "available": available,
            "reason": reason
        }]
    })
}

fn api_error(status: &str, code: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "code": code,
        "message": format!("mock {status}"),
        "retryable": false,
        "trace_id": "trace-test",
        "details": {}
    }))
    .unwrap()
}

fn cli_with_output(temp: &TempDir, config: &Path, mode: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_a4n"));
    command
        .current_dir(temp.path())
        .args(["--config"])
        .arg(config)
        .args(["--output", mode, "--no-input"]);
    command
}

fn bare_cli(temp: &TempDir) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_a4n"));
    command
        .current_dir(temp.path())
        .args(["--output", "agent-json", "--no-input"]);
    command
}

fn run_with_stdin(command: &mut Command, input: &str) -> Output {
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn cli_capabilities_and_safety_gates_work_outside_repository() {
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

    let output = cli(&temp, &config).arg("local").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(agent_error(&output)["error"]["code"], "invalid_input");

    let output = cli(&temp, &config)
        .args(["capabilities", "describe", "list_projects"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["data"]["name"],
        "list_projects"
    );

    let output = cli(&temp, &config)
        .args(["capabilities", "list", "--domain", "project"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        !serde_json::from_slice::<Value>(&output.stdout).unwrap()["data"]["commands"]
            .as_array()
            .unwrap()
            .is_empty()
    );

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
fn exported_manifest_covers_every_descriptor_and_gates_all_destructive_calls() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    write_config(&config, "http://127.0.0.1:9");

    let output = cli(&temp, &config)
        .args(["capabilities", "export"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let manifest: Value = serde_json::from_slice(&output.stdout).unwrap();
    let descriptors = manifest["data"]["commands"].as_array().unwrap();
    assert_eq!(descriptors.len(), API_COMMANDS.len());
    assert_eq!(
        manifest["data"]["manifest_version"],
        COMMAND_MANIFEST_VERSION
    );
    for (descriptor, expected_name) in descriptors.iter().zip(API_COMMANDS) {
        assert_eq!(descriptor["name"], *expected_name);
        assert!(
            descriptor["args_schema"].is_object(),
            "{expected_name} args schema"
        );
        assert!(
            descriptor["result_schema"].is_object(),
            "{expected_name} result schema"
        );
        assert!(
            descriptor["invoke"]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
            "{expected_name} invoke"
        );
    }

    let destructive_commands: Vec<_> = API_COMMANDS
        .iter()
        .copied()
        .filter(|command| is_destructive(command))
        .collect();
    assert!(!destructive_commands.is_empty());
    for command in destructive_commands {
        let output = cli(&temp, &config)
            .args(["call", command, "--args", "{}"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(10), "{command}");
        assert_eq!(
            agent_error(&output)["error"]["code"],
            "destructive_confirmation_required"
        );
    }
}

#[test]
fn page_attach_reads_code_from_stdin_and_never_prints_the_session_token() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(json!([page_session_value(
            "attached",
            Some("agent-1"),
            None,
            3
        )])),
        json_response(json!({
            "session": page_session_value("attached", Some("agent-1"), None, 3),
            "ui_session_token": "ui-session-secret-1"
        })),
        json_response(json!({
            "session": page_session_value("attached", Some("agent-1"), None, 3),
            "ui_session_token": "ui-session-secret-2"
        })),
    ]);
    write_json(
        &config,
        &json!({
            "server": server,
            "access_token": "access-token",
            "agent_id": "agent-1",
            "agent_token": "agent-secret"
        }),
    );

    let sessions = success_data(
        &cli(&temp, &config)
            .args(["page", "sessions"])
            .output()
            .unwrap(),
    );
    assert_eq!(sessions[0]["ui_session_id"], "ui-session-1");

    let direct_output = cli(&temp, &config)
        .args(["page", "attach", "--code", "AUTH-CODE"])
        .output()
        .unwrap();
    let direct_data = success_data(&direct_output);
    assert_eq!(direct_data["attached"], true);
    assert!(!String::from_utf8_lossy(&direct_output.stdout).contains("ui-session-secret-1"));
    assert!(!String::from_utf8_lossy(&direct_output.stdout).contains("AUTH-CODE"));

    let output = run_with_stdin(
        cli(&temp, &config).args(["page", "attach", "--code-stdin"]),
        "AUTH-CODE-STDIN\n",
    );
    let data = success_data(&output);
    assert_eq!(data["attached"], true);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("ui-session-secret-2"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("AUTH-CODE-STDIN"));

    let stored: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    assert_eq!(stored["ui_session_id"], "ui-session-1");
    assert_eq!(stored["ui_session_token"], "ui-session-secret-2");

    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 3);
    assert!(requests[0].starts_with("GET /api/ui-sessions "));
    let (headers, _) = requests[1].split_once("\r\n\r\n").unwrap();
    assert!(headers.to_ascii_lowercase().contains("x-agent-id: agent-1"));
    assert!(
        headers
            .to_ascii_lowercase()
            .contains("x-agent-token: agent-secret")
    );
    assert!(!headers.contains("AUTH-CODE"));
    assert_eq!(
        request_json_body(&requests[1])["authorization_code"],
        "AUTH-CODE"
    );
    assert_eq!(
        request_json_body(&requests[2])["authorization_code"],
        "AUTH-CODE-STDIN"
    );
}

#[test]
fn page_context_navigation_and_non_destructive_controls_use_the_expected_headers() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(page_session_value("attached", Some("agent-1"), None, 5)),
        json_response(page_session_value("attached", Some("agent-1"), None, 5)),
        json_response(page_manifest_value("list_projects", true, None)),
        json_response(page_session_value("attached", Some("agent-1"), None, 6)),
        json_response(page_session_value("paused", Some("agent-1"), None, 6)),
        json_response(page_session_value("attached", Some("agent-1"), None, 6)),
        json_response(page_session_value("attached", Some("agent-1"), None, 6)),
    ]);
    write_json(
        &config,
        &json!({
            "server": server,
            "access_token": "access-token",
            "agent_id": "agent-1",
            "agent_token": "agent-secret",
            "ui_session_id": "ui-session-1",
            "ui_session_token": "ui-session-secret"
        }),
    );

    let status = success_data(
        &cli(&temp, &config)
            .args(["page", "status"])
            .output()
            .unwrap(),
    );
    assert_eq!(status["status"], "attached");
    let context = success_data(
        &cli(&temp, &config)
            .args(["page", "context"])
            .output()
            .unwrap(),
    );
    assert_eq!(context["route"], "/novel/42");
    assert!(context.get("agent_id").is_none());
    let commands = success_data(
        &cli(&temp, &config)
            .args(["page", "commands"])
            .output()
            .unwrap(),
    );
    assert_eq!(
        commands["commands"][0]["descriptor"]["name"],
        "list_projects"
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "page",
                "navigate",
                "/settings",
                "--page-kind",
                "settings",
                "--reason",
                "review",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["page", "pause"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["page", "resume"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["page", "cancel"])
            .output()
            .unwrap(),
    );

    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 7);
    assert!(requests[0].starts_with("GET /api/ui-sessions/ui-session-1 "));
    assert!(requests[1].starts_with("GET /api/ui-sessions/ui-session-1 "));
    assert!(requests[2].starts_with("GET /api/ui-sessions/ui-session-1/commands "));
    for request in &requests[..3] {
        let lower = request.to_ascii_lowercase();
        assert!(lower.contains("x-ui-session-token: ui-session-secret"));
        assert!(!lower.contains("x-agent-token:"));
    }
    assert!(requests[3].starts_with("POST /api/ui-sessions/ui-session-1/navigate "));
    assert_eq!(request_json_body(&requests[3])["route"], "/settings");
    assert_eq!(request_json_body(&requests[3])["page_kind"], "settings");
    assert_eq!(request_json_body(&requests[3])["reason"], "review");
    assert!(
        requests[3]
            .to_ascii_lowercase()
            .contains("x-agent-token: agent-secret")
    );
    assert!(
        requests[4].ends_with("/control/pause HTTP/1.1\r\n")
            || requests[4].contains("/control/pause ")
    );
    assert!(requests[5].contains("/control/resume "));
    assert!(requests[6].contains("/control/cancel "));
    for request in &requests[4..] {
        let lower = request.to_ascii_lowercase();
        assert!(lower.contains("x-ui-session-token: ui-session-secret"));
        assert!(!lower.contains("x-agent-token:"));
    }
}

#[test]
fn page_takeover_and_detach_clear_saved_page_credentials() {
    let temp = TempDir::new().unwrap();
    let takeover_config = temp.path().join("takeover.json");
    let detach_config = temp.path().join("detach.json");
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(page_session_value("idle", None, None, 1)),
        json_response(page_session_value("idle", None, None, 1)),
    ]);
    for config in [&takeover_config, &detach_config] {
        write_json(
            config,
            &json!({
                "server": server,
                "access_token": "access-token",
                "ui_session_id": "ui-session-1",
                "ui_session_token": "ui-session-secret"
            }),
        );
    }

    success_data(
        &cli(&temp, &takeover_config)
            .args(["page", "takeover"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &detach_config)
            .args(["page", "detach"])
            .output()
            .unwrap(),
    );

    for config in [takeover_config, detach_config] {
        let stored: Value = serde_json::from_slice(&fs::read(config).unwrap()).unwrap();
        assert!(stored.get("ui_session_id").is_none());
        assert!(stored.get("ui_session_token").is_none());
    }
    let requests = server_thread.join().unwrap();
    assert!(requests[0].contains("/control/takeover "));
    assert!(requests[1].contains("/control/detach "));
}

#[test]
fn page_call_tracks_operation_before_host_call_and_redacts_sensitive_targets() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(page_session_value("attached", Some("agent-1"), None, 5)),
        json_response(page_manifest_value("list_projects", true, None)),
        json_response(page_operation_value(
            "running",
            json!({"api_key": "[REDACTED]"}),
            Value::Null,
        )),
        json_response(json!({"echo": "top-secret", "items": [{"id": 1}]})),
        json_response(page_operation_value(
            "succeeded",
            json!({"api_key": "[REDACTED]"}),
            json!({"echo": "[REDACTED]", "items": [{"id": 1}]}),
        )),
    ]);
    write_json(
        &config,
        &json!({
            "server": server,
            "access_token": "access-token",
            "agent_id": "agent-1",
            "agent_token": "agent-secret",
            "ui_session_id": "ui-session-1",
            "ui_session_token": "ui-session-secret"
        }),
    );

    let output = cli(&temp, &config)
        .args([
            "page",
            "call",
            "list_projects",
            "--args",
            r#"{"api_key":"top-secret"}"#,
        ])
        .output()
        .unwrap();
    let data = success_data(&output);
    assert_eq!(data["result"]["echo"], "[REDACTED]");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("top-secret"));

    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 5);
    assert!(requests[0].contains("GET /api/ui-sessions/ui-session-1 "));
    assert!(requests[1].contains("GET /api/ui-sessions/ui-session-1/commands "));
    assert!(requests[2].contains("POST /api/ui-sessions/ui-session-1/operations "));
    assert_eq!(
        request_json_body(&requests[2])["target"]["api_key"],
        "[REDACTED]"
    );
    assert!(
        requests[2]
            .to_ascii_lowercase()
            .contains("x-agent-token: agent-secret")
    );
    assert!(requests[3].contains("POST /api/commands/list_projects "));
    assert_eq!(request_json_body(&requests[3])["api_key"], "top-secret");
    let lower = requests[3].to_ascii_lowercase();
    assert!(!lower.contains("x-ui-session-token:"));
    assert!(!lower.contains("x-agent-token:"));
    assert!(requests[4].contains("PATCH /api/ui-sessions/ui-session-1/operations/operation-1 "));
    assert_eq!(
        request_json_body(&requests[4])["result"]["echo"],
        "[REDACTED]"
    );
}

#[test]
fn page_call_surfaces_failure_when_operation_status_cannot_be_written() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(page_session_value("attached", Some("agent-1"), None, 5)),
        json_response(page_manifest_value("list_projects", true, None)),
        json_response(page_operation_value("running", json!({}), Value::Null)),
        MockResponse {
            status: "500 Internal Server Error",
            content_type: "application/json",
            body: api_error("500 Internal Server Error", "host_failed"),
            declared_length: None,
        },
        MockResponse {
            status: "500 Internal Server Error",
            content_type: "application/json",
            body: api_error("500 Internal Server Error", "operation_update_failed"),
            declared_length: None,
        },
    ]);
    write_json(
        &config,
        &json!({
            "server": server,
            "access_token": "access-token",
            "agent_id": "agent-1",
            "agent_token": "agent-secret",
            "ui_session_id": "ui-session-1",
            "ui_session_token": "ui-session-secret"
        }),
    );

    let output = cli(&temp, &config)
        .args(["page", "call", "list_projects", "--args", "{}"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(
        agent_error(&output)["error"]["code"],
        "operation_update_failed"
    );
    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 5);
    assert!(requests[3].contains("POST /api/commands/list_projects "));
    assert!(requests[4].contains("PATCH /api/ui-sessions/ui-session-1/operations/operation-1 "));
    assert_eq!(request_json_body(&requests[4])["status"], "failed");
}

#[test]
fn page_call_stops_at_page_manifest_gates_before_operation_requests() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(page_session_value("attached", Some("agent-1"), None, 1)),
        json_response(page_manifest_value("delete_project", true, None)),
    ]);
    write_json(
        &config,
        &json!({
            "server": server,
            "access_token": "access-token",
            "agent_id": "agent-1",
            "agent_token": "agent-secret",
            "ui_session_id": "ui-session-1",
            "ui_session_token": "ui-session-secret"
        }),
    );

    let output = cli(&temp, &config)
        .args(["page", "call", "delete_project", "--args", "{}"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(10));
    assert_eq!(
        agent_error(&output)["error"]["code"],
        "destructive_confirmation_required"
    );
    assert_eq!(server_thread.join().unwrap().len(), 2);
}

#[test]
fn page_call_rejects_a_command_not_available_on_the_current_page() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(page_session_value("attached", Some("agent-1"), None, 1)),
        json_response(page_manifest_value(
            "list_projects",
            false,
            Some("当前页面缺少项目上下文"),
        )),
    ]);
    write_json(
        &config,
        &json!({
            "server": server,
            "access_token": "access-token",
            "agent_id": "agent-1",
            "agent_token": "agent-secret",
            "ui_session_id": "ui-session-1",
            "ui_session_token": "ui-session-secret"
        }),
    );

    let output = cli(&temp, &config)
        .args(["page", "call", "list_projects", "--args", "{}"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(6));
    assert_eq!(
        agent_error(&output)["error"]["code"],
        "page_command_unavailable"
    );
    assert_eq!(server_thread.join().unwrap().len(), 2);
}

#[test]
fn page_events_parse_chunked_crlf_sse_frames() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_chunked_response(
        "text/event-stream",
        vec![
            b": keep-alive\r\nevent: page_session\r\nid: 7\r\ndata: {\"kind\":\"navi".to_vec(),
            b"gate\",\"route\":\"/settings\"}\r\n\r\n".to_vec(),
        ],
    );
    write_config(&config, &server);

    let output = cli(&temp, &config)
        .args(["page", "events"])
        .output()
        .unwrap();
    let data = success_data(&output);
    assert_eq!(data["kind"], "navigate");
    assert_eq!(data["route"], "/settings");
    let request = server_thread.join().unwrap();
    assert!(request.starts_with("GET /api/ui-sessions/events "));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("accept: text/event-stream")
    );
}

#[test]
fn agent_mode_requires_a_saved_lease_for_work_commands() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    write_config(&config, "http://127.0.0.1:9");

    let output = agent_cli(&temp, &config)
        .args(["call", "list_projects", "--args", "{}"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let error = agent_error(&output);
    assert_eq!(error["error"]["code"], "invalid_input");
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("自动 Agent 跟踪缺少 agent_id")
    );
}

#[test]
fn agent_mode_tracks_work_commands_with_the_saved_lease() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_json_replies(vec![
        json!({
            "agent": {"agent_id": "agent-test"},
            "agent_token": "lease-test"
        }),
        json!({}),
        json!({}),
        json!({}),
    ]);
    write_config(&config, &server);

    let output = cli(&temp, &config)
        .args([
            "agent",
            "register",
            "--name",
            "test-agent",
            "--kind",
            "media",
            "--client",
            "cli",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stored: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    assert_eq!(stored["agent_id"], "agent-test");
    assert_eq!(stored["agent_token"], "lease-test");

    let output = agent_cli(&temp, &config)
        .args(["call", "list_projects", "--args", "{}"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["ok"],
        true
    );

    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests[0].starts_with("POST /api/agents "));
    assert_eq!(request_json_body(&requests[0])["name"], "test-agent");
    assert_eq!(request_json_body(&requests[0])["kind"], "media");
    for request in &requests {
        let headers = request.to_ascii_lowercase();
        assert!(headers.contains("\r\nauthorization: bearer test-token\r\n"));
    }
    for request in [&requests[1], &requests[3]] {
        assert!(
            request
                .to_ascii_lowercase()
                .contains("x-agent-token: lease-test")
        );
    }
    assert!(requests[1].starts_with("POST /api/agents/agent-test/heartbeat "));
    assert_eq!(request_json_body(&requests[1])["status"], "running");
    assert_eq!(
        request_json_body(&requests[1])["phase"],
        "CLI 调用 list_projects"
    );
    assert!(requests[2].starts_with("POST /api/commands/list_projects "));
    assert!(requests[3].starts_with("POST /api/agents/agent-test/heartbeat "));
    assert_eq!(request_json_body(&requests[3])["status"], "completed");
    assert_eq!(request_json_body(&requests[3])["summary"], "CLI 命令已完成");
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
fn health_accepts_a_valid_a4n_server_identity() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_reply(
        "200 OK",
        serde_json::to_vec(&json!({ "host": HOST_IDENTITY, "version": "test" })).unwrap(),
        None,
    );
    write_config(&config, &server);

    let data = success_data(&cli(&temp, &config).arg("health").output().unwrap());
    assert_eq!(data["server"], format!("{server}/api"));
    assert_eq!(data["status"], "ok");
    assert_eq!(data["meta"]["host"], HOST_IDENTITY);
    assert!(server_thread.join().unwrap().starts_with("GET /api/meta "));
}

#[test]
fn doctor_and_local_migrate_report_missing_local_prerequisites() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    write_config(&config, "http://127.0.0.1:9");

    let output = cli(&temp, &config)
        .env_remove("A4N_DATABASE_URL")
        .env_remove("A4N_JWT_SECRET")
        .arg("doctor")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(7));
    let error = agent_error(&output);
    assert_eq!(error["error"]["code"], "doctor_failed");
    let checks = error["error"]["details"]["checks"].as_array().unwrap();
    assert!(
        checks
            .iter()
            .any(|check| { check["name"] == "postgres" && check["status"] == "error" })
    );
    assert!(
        checks
            .iter()
            .any(|check| check["name"] == "jwt" && check["status"] == "error")
    );

    let output = cli(&temp, &config)
        .env_remove("A4N_DATABASE_URL")
        .args(["local", "migrate"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(agent_error(&output)["error"]["code"], "invalid_input");
    assert!(
        agent_error(&output)["error"]["message"]
            .as_str()
            .unwrap()
            .contains("A4N_DATABASE_URL")
    );
}

#[test]
fn doctor_reports_a_healthy_local_environment_and_strict_warnings_fail() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");

    let temp = TempDir::new().unwrap();
    let config = temp.path().join("healthy-cli.json");
    let (server, server_thread) = spawn_reply(
        "200 OK",
        serde_json::to_vec(&json!({ "host": HOST_IDENTITY })).unwrap(),
        None,
    );
    let (database, database_thread) = spawn_tcp_accept();
    let (qdrant, qdrant_thread) = spawn_tcp_accept();
    write_config(&config, &server);
    let output = cli(&temp, &config)
        .env(
            "A4N_DATABASE_URL",
            format!("postgres://user:pass@{database}/a4n"),
        )
        .env("A4N_JWT_SECRET", "doctor-test-secret")
        .env("A4N_QDRANT_URL", format!("http://{qdrant}"))
        .env("A4N_SOURCE_ROOT", &source_root)
        .arg("doctor")
        .output()
        .unwrap();
    let report = success_data(&output);
    assert_eq!(report["healthy"], true);
    assert_eq!(report["errors"], 0);
    assert_eq!(report["warnings"], 0);
    assert!(
        report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|check| { check["status"] == "ok" })
    );
    server_thread.join().unwrap();
    database_thread.join().unwrap();
    qdrant_thread.join().unwrap();

    let temp = TempDir::new().unwrap();
    let config = temp.path().join("warning-cli.json");
    let (server, server_thread) = spawn_reply(
        "200 OK",
        serde_json::to_vec(&json!({ "host": HOST_IDENTITY })).unwrap(),
        None,
    );
    let (database, database_thread) = spawn_tcp_accept();
    write_config(&config, &server);
    let output = cli(&temp, &config)
        .env(
            "A4N_DATABASE_URL",
            format!("postgres://user:pass@{database}/a4n"),
        )
        .env("A4N_JWT_SECRET", "doctor-test-secret")
        .env_remove("A4N_QDRANT_URL")
        .env("A4N_SOURCE_ROOT", &source_root)
        .args(["doctor", "--strict"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(7));
    let error = agent_error(&output);
    assert_eq!(error["error"]["code"], "doctor_failed");
    assert_eq!(error["error"]["details"]["errors"], 0);
    assert_eq!(error["error"]["details"]["warnings"], 1);
    assert!(
        error["error"]["details"]["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["name"] == "qdrant" && check["status"] == "warning")
    );
    server_thread.join().unwrap();
    database_thread.join().unwrap();
}

#[test]
fn server_and_config_environment_precedence_is_explicit_and_empty_values_fail() {
    let temp = TempDir::new().unwrap();

    let config = temp.path().join("stored.json");
    write_config(&config, "http://127.0.0.1:9");
    let (env_server, env_thread) = spawn_reply(
        "200 OK",
        serde_json::to_vec(&json!({ "host": HOST_IDENTITY })).unwrap(),
        None,
    );
    let output = cli(&temp, &config)
        .env("A4N_SERVER_URL", &env_server)
        .arg("health")
        .output()
        .unwrap();
    assert!(output.status.success());
    env_thread.join().unwrap();

    let (env_server, env_thread) =
        spawn_optional_reply(json!({ "host": HOST_IDENTITY, "selected": "env" }));
    let (explicit_server, explicit_thread) = spawn_reply(
        "200 OK",
        serde_json::to_vec(&json!({ "host": HOST_IDENTITY, "selected": "explicit" })).unwrap(),
        None,
    );
    write_config(&config, "http://127.0.0.1:9");
    let output = cli(&temp, &config)
        .env("A4N_SERVER_URL", &env_server)
        .args(["--server", &explicit_server, "health"])
        .output()
        .unwrap();
    assert_eq!(success_data(&output)["meta"]["selected"], "explicit");
    assert!(env_thread.join().unwrap().is_none());
    explicit_thread.join().unwrap();

    let env_config = temp.path().join("env-config.json");
    let (env_config_server, env_config_thread) = spawn_reply(
        "200 OK",
        serde_json::to_vec(&json!({ "host": HOST_IDENTITY, "selected": "env-config" })).unwrap(),
        None,
    );
    write_config(&env_config, &env_config_server);
    let output = bare_cli(&temp)
        .env("A4N_CLI_CONFIG", &env_config)
        .env_remove("A4N_SERVER_URL")
        .arg("health")
        .output()
        .unwrap();
    assert_eq!(success_data(&output)["meta"]["selected"], "env-config");
    env_config_thread.join().unwrap();

    let explicit_config = temp.path().join("explicit-config.json");
    let ignored_config = temp.path().join("ignored-config.json");
    let (explicit_config_server, explicit_config_thread) = spawn_reply(
        "200 OK",
        serde_json::to_vec(&json!({ "host": HOST_IDENTITY, "selected": "explicit-config" }))
            .unwrap(),
        None,
    );
    let (ignored_config_server, ignored_config_thread) =
        spawn_optional_reply(json!({ "host": HOST_IDENTITY, "selected": "ignored-config" }));
    write_config(&explicit_config, &explicit_config_server);
    write_config(&ignored_config, &ignored_config_server);
    let output = bare_cli(&temp)
        .args(["--config", explicit_config.to_str().unwrap()])
        .env("A4N_CLI_CONFIG", &ignored_config)
        .env_remove("A4N_SERVER_URL")
        .arg("health")
        .output()
        .unwrap();
    assert_eq!(success_data(&output)["meta"]["selected"], "explicit-config");
    explicit_config_thread.join().unwrap();
    assert!(ignored_config_thread.join().unwrap().is_none());

    write_config(&config, "http://127.0.0.1:9");
    let output = cli(&temp, &config)
        .env("A4N_SERVER_URL", "   ")
        .arg("health")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(agent_error(&output)["error"]["code"], "invalid_input");

    let output = bare_cli(&temp)
        .env("A4N_CLI_CONFIG", "   ")
        .env_remove("A4N_SERVER_URL")
        .arg("health")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(agent_error(&output)["error"]["code"], "invalid_input");
}

#[test]
fn changing_server_clears_stored_auth_and_agent_credentials() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    write_json(
        &config,
        &json!({
            "server": "http://127.0.0.1:8080",
            "access_token": "access-secret",
            "refresh_token": "refresh-secret",
            "session_id": "session-1",
            "agent_id": "agent-1",
            "agent_token": "lease-secret"
        }),
    );
    let data = success_data(
        &cli(&temp, &config)
            .args(["config", "set-server", "https://example.test/base"])
            .output()
            .unwrap(),
    );
    assert_eq!(data["server"], "https://example.test/base/api");
    let stored: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    assert_eq!(stored["server"], "https://example.test/base/api");
    for field in [
        "access_token",
        "refresh_token",
        "session_id",
        "agent_id",
        "agent_token",
    ] {
        assert!(stored.get(field).is_none_or(Value::is_null), "{field}");
    }
}

#[test]
fn runtime_config_is_persisted_and_injected_by_manifest_policy() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let snapshot = temp.path().join("runtime.json");
    write_config(&config, "http://127.0.0.1:9");
    write_json(
        &snapshot,
        &json!({
            "schema_version": 2,
            "llm": {
                "id": 1,
                "name": "local",
                "provider": "openai",
                "base_url": "https://example.test/v1",
                "api_key_encrypted": "runtime-secret",
                "model": "model",
                "max_concurrent": 1,
                "timeout_secs": 30,
                "max_context_tokens": 4096,
                "max_output_tokens": 1024
            }
        }),
    );

    let output = cli(&temp, &config)
        .args(["config", "runtime", "set", "--input"])
        .arg(format!("@{}", snapshot.display()))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("runtime-secret"));
    assert!(
        fs::read_to_string(&config)
            .unwrap()
            .contains("runtime-secret")
    );

    let (server, server_thread) = spawn_reply("200 OK", b"{}", None);
    write_config_with_runtime(&config, &server, &snapshot);
    let output = cli(&temp, &config)
        .args([
            "call",
            "preview_check_character_conflict_request",
            "--args",
            r#"{"input":{"project_id":1,"name":"Alice","start_chapter":null,"end_chapter":null}}"#,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let request = server_thread.join().unwrap();
    let body = request_json_body(&request);
    assert!(
        body["runtime_config_snapshot"]["llm"]
            .get("api_key_encrypted")
            .is_none_or(Value::is_null)
    );

    let (server, server_thread) = spawn_reply("200 OK", b"[]", None);
    write_config_with_runtime(&config, &server, &snapshot);
    let output = cli(&temp, &config)
        .args([
            "call",
            "check_character_conflict",
            "--args",
            r#"{"input":{"project_id":1,"name":"Alice","start_chapter":null,"end_chapter":null},"taskId":"character-conflict-test"}"#,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = request_json_body(&server_thread.join().unwrap());
    assert_eq!(
        body["runtime_config_snapshot"]["llm"]["api_key_encrypted"],
        "runtime-secret"
    );
}

fn write_config_with_runtime(config: &Path, server: &str, snapshot: &Path) {
    let runtime_config_snapshot: Value =
        serde_json::from_slice(&fs::read(snapshot).unwrap()).unwrap();
    write_json(
        config,
        &json!({
            "server": server,
            "access_token": "test-token",
            "runtime_config_snapshot": runtime_config_snapshot,
        }),
    );
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
fn successful_media_download_writes_atomic_bytes_and_reports_content_type() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let output_path = temp.path().join("image.png");
    let (server, server_thread) = spawn_scripted_replies(vec![MockResponse {
        status: "200 OK",
        content_type: "image/png",
        body: b"\x89PNG\r\n".to_vec(),
        declared_length: None,
    }]);
    write_config(&config, &server);

    let data = success_data(
        &cli(&temp, &config)
            .args(["media", "download", "generated/image.png", "--output"])
            .arg(&output_path)
            .output()
            .unwrap(),
    );
    assert_eq!(data["media_path"], "generated/image.png");
    assert_eq!(data["byte_size"], 6);
    assert_eq!(data["content_type"], "image/png");
    assert_eq!(fs::read(&output_path).unwrap(), b"\x89PNG\r\n");

    let request = server_thread.join().unwrap().pop().unwrap();
    assert!(request.starts_with("GET /api/media?path=generated%2Fimage.png "));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer test-token")
    );
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
            .args(["task", "watch", "task-1", "--after-sequence", "0"])
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

    for (status, expected_code, expected_error) in [
        ("failed", 8, "workflow_failed"),
        ("cancelled", 9, "workflow_cancelled"),
        ("interrupted", 9, "workflow_interrupted"),
        ("superseded", 9, "workflow_superseded"),
        ("mystery", 8, "workflow_contract_invalid"),
    ] {
        let (server, server_thread) = spawn_json_replies(vec![
            json!({"events": []}),
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
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("POST /api/commands/get_workflow_progress_events "));
        assert!(requests[1].starts_with("POST /api/commands/get_workflow_run_detail "));
    }

    let (server, server_thread) = spawn_json_replies(vec![json!({"unexpected": "object"})]);
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
    assert_eq!(server_thread.join().unwrap().len(), 1);
}

#[test]
fn semantic_project_and_chapter_commands_route_and_persist_context() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let context = temp.path().join("context.json");
    let chapter_file = temp.path().join("chapter.txt");
    fs::write(&chapter_file, "来自文件的章节正文").unwrap();
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(json!({"items": [{"id": 7, "title": "旧项目"}]})),
        json_response(json!({"id": 7, "title": "旧项目"})),
        json_response(json!({"id": 8, "title": "新项目"})),
        json_response(user_value()),
        json_response(json!({"id": 9, "title": "当前项目"})),
        json_response(user_value()),
        json_response(json!([{"id": 11, "title": "第一章"}])),
        json_response(json!([{"id": 11, "title": "第一章"}])),
        json_response(json!([{"id": 11, "title": "第一章"}])),
        json_response(user_value()),
        json_response(json!({"id": 12, "chapter_number": 1})),
        json_response(json!({"id": 11, "updated": true})),
        json_response(json!({"deleted": true})),
    ]);
    write_config(&config, &server);

    success_data(
        &cli(&temp, &config)
            .args([
                "--context",
                context.to_str().unwrap(),
                "project",
                "list",
                "--page",
                "2",
                "--page-size",
                "10",
                "--kind",
                "novel_long",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "--context",
                context.to_str().unwrap(),
                "project",
                "get",
                "7",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "--context",
                context.to_str().unwrap(),
                "project",
                "create",
                "--title",
                "新项目",
                "--project-type",
                "novel_long",
                "--use-context",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "--context",
                context.to_str().unwrap(),
                "project",
                "use",
                "9",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["--context", context.to_str().unwrap(), "chapter", "list"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "--context",
                context.to_str().unwrap(),
                "chapter",
                "get",
                "11",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "--context",
                context.to_str().unwrap(),
                "chapter",
                "use",
                "11",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "--context",
                context.to_str().unwrap(),
                "chapter",
                "import",
                "--number",
                "1",
                "--content",
                &format!("@{}", chapter_file.display()),
                "--title",
                "第一章",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "--context",
                context.to_str().unwrap(),
                "chapter",
                "update",
                "--content",
                "更新后的正文",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "--context",
                context.to_str().unwrap(),
                "project",
                "delete",
                "--yes",
            ])
            .output()
            .unwrap(),
    );

    let stored_context: Value = serde_json::from_slice(&fs::read(&context).unwrap()).unwrap();
    assert_eq!(stored_context["user_id"], 42);
    assert_eq!(stored_context["project_id"], Value::Null);
    assert_eq!(stored_context["chapter_id"], Value::Null);

    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 13);
    assert!(requests[0].starts_with("POST /api/commands/list_projects "));
    assert_eq!(request_json_body(&requests[0])["page"], 2);
    assert_eq!(request_json_body(&requests[0])["pageSize"], 10);
    assert_eq!(request_json_body(&requests[0])["projectKind"], "novel_long");
    assert!(requests[1].starts_with("POST /api/commands/get_project "));
    assert_eq!(request_json_body(&requests[1])["projectId"], 7);
    assert!(requests[2].starts_with("POST /api/commands/create_project "));
    assert_eq!(request_json_body(&requests[2])["input"]["title"], "新项目");
    assert!(requests[3].starts_with("POST /api/commands/get_current_user "));
    assert!(requests[4].starts_with("POST /api/commands/get_project "));
    assert_eq!(request_json_body(&requests[4])["projectId"], 9);
    assert!(requests[5].starts_with("POST /api/commands/get_current_user "));
    for request in requests.iter().take(9).skip(6) {
        assert!(request.starts_with("POST /api/commands/list_chapters "));
        assert_eq!(request_json_body(request)["projectId"], 9);
    }
    assert!(requests[9].starts_with("POST /api/commands/get_current_user "));
    assert!(requests[10].starts_with("POST /api/commands/import_chapter "));
    assert_eq!(
        request_json_body(&requests[10])["input"]["content"],
        "来自文件的章节正文"
    );
    assert!(requests[11].starts_with("POST /api/commands/update_chapter_content "));
    assert_eq!(request_json_body(&requests[11])["input"]["chapter_id"], 11);
    assert!(requests[12].starts_with("POST /api/commands/delete_project "));
    assert_eq!(request_json_body(&requests[12])["projectId"], 9);
}

#[test]
fn semantic_task_and_workflow_commands_cover_queries_and_mutations() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(json!({"id": "task-1", "status": "running"})),
        json_response(json!({"id": "task-1", "snapshot": {"step": 1}})),
        json_response(json!({"items": []})),
        json_response(json!({"items": [], "next_cursor": null})),
        json_response(json!({"cancelled": true})),
        json_response(json!({"runs": []})),
        json_response(json!({"run": {"id": 31, "status": "paused"}})),
        json_response(json!({"events": [{"id": "event-1"}]})),
        json_response(json!({"id": 31, "status": "running"})),
        json_response(json!({"cancelled": true})),
    ]);
    write_config(&config, &server);

    success_data(
        &cli(&temp, &config)
            .args(["task", "get", "task-1", "--attempt", "2"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["task", "snapshot", "task-1"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["task", "active", "--project-id", "7"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "task",
                "history",
                "--project-id",
                "7",
                "--limit",
                "10",
                "--cursor",
                "next/page",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["task", "cancel", "task-1", "--attempt", "2", "--yes"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["workflow", "list", "--project-id", "7", "--limit", "10"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["workflow", "show", "31", "--no-generation-runs"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["workflow", "events", "31", "--limit", "5"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["workflow", "resume", "31"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["workflow", "cancel", "--workflow-run-id", "31", "--yes"])
            .output()
            .unwrap(),
    );

    let invalid = cli(&temp, &config)
        .args(["task", "get", "task-1", "--attempt", "0"])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
    assert_eq!(agent_error(&invalid)["error"]["code"], "invalid_input");

    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 10);
    assert!(requests[0].starts_with("GET /api/generation-tasks/task-1?attempt=2 "));
    assert!(requests[1].starts_with("GET /api/generation-tasks/task-1/snapshot?attempt=1 "));
    assert!(requests[2].starts_with("GET /api/generation-tasks/active?project_id=7 "));
    assert!(requests[3].starts_with(
        "GET /api/generation-tasks/history?project_id=7&limit=10&cursor=next%2Fpage "
    ));
    assert!(requests[4].starts_with("POST /api/generation-tasks/task-1/cancel "));
    assert_eq!(request_json_body(&requests[4])["attempt"], 2);
    assert!(requests[5].starts_with("POST /api/commands/list_workflow_runs "));
    assert_eq!(request_json_body(&requests[5])["input"]["project_id"], 7);
    assert!(requests[6].starts_with("POST /api/commands/get_workflow_run_detail "));
    assert_eq!(
        request_json_body(&requests[6])["input"]["include_generation_runs"],
        false
    );
    assert!(requests[7].starts_with("POST /api/commands/get_workflow_progress_events "));
    assert_eq!(request_json_body(&requests[7])["input"]["limit"], 5);
    assert!(requests[8].starts_with("POST /api/commands/resume_workflow_run "));
    assert!(requests[9].starts_with("POST /api/commands/cancel_workflow "));
    assert_eq!(
        request_json_body(&requests[9])["input"]["workflow_run_id"],
        31
    );
}

#[test]
fn semantic_filters_and_alternate_cancel_targets_route_all_fields() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(json!({"items": []})),
        json_response(json!({"runs": []})),
        json_response(json!({"run": {"id": 31, "include_generation_runs": true}})),
        json_response(json!({"cancelled": true})),
        json_response(json!({"updated": true})),
        json_response(json!({"updated": true})),
    ]);
    write_config(&config, &server);

    success_data(
        &cli(&temp, &config)
            .args([
                "project",
                "list",
                "--page",
                "3",
                "--page-size",
                "25",
                "--kind",
                "derived",
                "--root-project-id",
                "17",
                "--access-scope",
                "shared",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "workflow",
                "list",
                "--project-id",
                "17",
                "--chapter-id",
                "18",
                "--kind",
                "chapter_generation",
                "--status",
                "paused",
                "--resumable-only",
                "--limit",
                "25",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["workflow", "show", "31"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["workflow", "cancel", "--task-id", "task-1", "--yes"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "auth",
                "update-profile",
                "--avatar-url",
                "https://example.test/avatar.png",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["auth", "update-profile", "--clear-avatar-url"])
            .output()
            .unwrap(),
    );

    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 6);
    assert!(requests[0].starts_with("POST /api/commands/list_projects "));
    assert_eq!(request_json_body(&requests[0])["page"], 3);
    assert_eq!(request_json_body(&requests[0])["pageSize"], 25);
    assert_eq!(request_json_body(&requests[0])["projectKind"], "derived");
    assert_eq!(request_json_body(&requests[0])["rootProjectId"], 17);
    assert_eq!(request_json_body(&requests[0])["accessScope"], "shared");
    assert!(requests[1].starts_with("POST /api/commands/list_workflow_runs "));
    let workflow_list = request_json_body(&requests[1])["input"].clone();
    assert_eq!(workflow_list["project_id"], 17);
    assert_eq!(workflow_list["chapter_id"], 18);
    assert_eq!(workflow_list["workflow_kind"], "chapter_generation");
    assert_eq!(workflow_list["status"], "paused");
    assert_eq!(workflow_list["resumable_only"], true);
    assert_eq!(workflow_list["limit"], 25);
    assert!(requests[2].starts_with("POST /api/commands/get_workflow_run_detail "));
    assert_eq!(
        request_json_body(&requests[2])["input"]["include_generation_runs"],
        true
    );
    assert!(requests[3].starts_with("POST /api/commands/cancel_workflow "));
    assert_eq!(
        request_json_body(&requests[3])["input"]["workflow_run_id"],
        Value::Null
    );
    assert_eq!(
        request_json_body(&requests[3])["input"]["task_id"],
        "task-1"
    );
    assert!(requests[4].starts_with("POST /api/commands/update_user_profile "));
    assert_eq!(
        request_json_body(&requests[4])["input"]["avatar_url"],
        "https://example.test/avatar.png"
    );
    assert!(requests[5].starts_with("POST /api/commands/update_user_profile "));
    assert_eq!(
        request_json_body(&requests[5])["input"]["avatar_url"],
        Value::Null
    );
}

#[test]
fn semantic_cli_rejects_invalid_ids_limits_and_cancel_shapes_before_network() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    write_config(&config, "http://127.0.0.1:9");
    let cases: &[&[&str]] = &[
        &["project", "list", "--page", "0"],
        &["project", "list", "--page-size", "101"],
        &["project", "list", "--root-project-id", "0"],
        &["workflow", "list", "--chapter-id", "0"],
        &["workflow", "list", "--limit", "0"],
        &["workflow", "list", "--limit", "201"],
        &["workflow", "show", "0"],
        &["workflow", "events", "31", "--limit", "201"],
        &["workflow", "watch", "31", "--interval-ms", "199"],
        &["workflow", "watch", "31", "--interval-ms", "60001"],
        &["auth", "update-profile"],
    ];
    for args in cases {
        let output = cli(&temp, &config).args(*args).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert_eq!(
            agent_error(&output)["error"]["code"],
            "invalid_input",
            "{args:?}"
        );
    }

    for args in [
        ["workflow", "cancel", "--yes"].as_slice(),
        [
            "workflow",
            "cancel",
            "--workflow-run-id",
            "31",
            "--task-id",
            "task-1",
            "--yes",
        ]
        .as_slice(),
        ["workflow", "cancel", "--workflow-run-id", "0", "--yes"].as_slice(),
        ["workflow", "cancel", "--task-id", "", "--yes"].as_slice(),
    ] {
        let output = cli(&temp, &config).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        if args.contains(&"--workflow-run-id") && args.contains(&"--task-id") {
            assert!(stderr.contains("cannot be used with"), "{stderr}");
        } else {
            assert_eq!(
                agent_error(&output)["error"]["code"],
                "invalid_input",
                "{args:?}"
            );
        }
    }
}

#[test]
fn agent_commands_use_saved_or_explicit_lease_and_unregister_persisted_credentials() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(json!({})),
        json_response(json!({})),
        json_response(json!({"agents": []})),
        json_response(json!({})),
    ]);
    write_json(
        &config,
        &json!({
            "server": server,
            "access_token": "test-token",
            "agent_id": "saved-agent",
            "agent_token": "lease-token"
        }),
    );

    success_data(
        &cli(&temp, &config)
            .args([
                "agent",
                "update",
                "--agent-id",
                "explicit-agent",
                "--status",
                "running",
                "--phase",
                "phase-1",
                "--summary",
                "working",
                "--progress",
                "25",
                "--current-task-id",
                "task-1",
                "--project-id",
                "3",
                "--chapter-id",
                "4",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "agent",
                "heartbeat",
                "--status",
                "completed",
                "--progress",
                "100",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["agent", "status"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["agent", "unregister"])
            .output()
            .unwrap(),
    );

    let stored: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    assert!(stored.get("agent_id").is_none_or(Value::is_null));
    assert!(stored.get("agent_token").is_none_or(Value::is_null));

    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests[0].starts_with("PATCH /api/agents/explicit-agent "));
    assert_eq!(request_json_body(&requests[0])["status"], "running");
    assert_eq!(request_json_body(&requests[0])["progress"], 25.0);
    assert!(requests[1].starts_with("POST /api/agents/saved-agent/heartbeat "));
    assert_eq!(request_json_body(&requests[1])["status"], "completed");
    assert!(requests[2].starts_with("GET /api/agents "));
    assert!(requests[3].starts_with("DELETE /api/agents/saved-agent "));
    for request in &requests {
        let lower = request.to_ascii_lowercase();
        assert!(lower.contains("authorization: bearer test-token"));
        if !request.starts_with("GET /api/agents ") {
            assert!(lower.contains("x-agent-token: lease-token"));
        }
    }
}

#[test]
fn auth_commands_use_stdin_and_persist_or_clear_sessions() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(auth_value("access-login", "refresh-login", "session-login")),
        json_response(user_value()),
        json_response(json!({
            "access_token": "access-refresh",
            "refresh_token": "refresh-refresh",
            "expires_in": 3600,
            "token_type": "Bearer",
            "session_id": "session-login"
        })),
        json_response(user_value()),
        json_response(json!([{
            "id": "session-login",
            "user_agent": "test",
            "created_at": "2026-08-22T00:00:00Z",
            "expires_at": "2026-08-23T00:00:00Z",
            "last_used_at": null
        }])),
        json_response(json!(
            "https://github.com/login/oauth/authorize?redirect_uri=http%3A%2F%2Flocalhost%3A1420%2Fauth%2Fcallback"
        )),
        json_response(json!({})),
        json_response(auth_value("access-oauth", "refresh-oauth", "session-oauth")),
        json_response(json!({})),
        json_response(json!({})),
        json_response(auth_value(
            "access-register",
            "refresh-register",
            "session-register",
        )),
    ]);
    write_config(&config, &server);

    let output = run_with_stdin(
        cli(&temp, &config).args(["auth", "login", "--username", "tester", "--password-stdin"]),
        "login-password\n",
    );
    assert!(output.status.success());

    success_data(
        &cli(&temp, &config)
            .args(["auth", "status"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["auth", "refresh"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["auth", "update-profile", "--email", "new@example.test"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["auth", "sessions"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["auth", "oauth-authorize", "github"])
            .output()
            .unwrap(),
    );

    let output = run_with_stdin(
        cli(&temp, &config).args(["auth", "change-password", "--passwords-stdin"]),
        "old-password\nnew-password\n",
    );
    assert!(output.status.success());

    let output = run_with_stdin(
        cli(&temp, &config).args(["auth", "oauth-callback", "github", "--code-stdin"]),
        "http://localhost:1420/auth/callback?state=wrong-state&code=oauth-code-secret\n",
    );
    assert!(!output.status.success());
    let pending: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    let nonce = pending["pending_oauth"]["state"].as_str().unwrap();
    let verifier = pending["pending_oauth"]["code_verifier"].as_str().unwrap();
    let output = run_with_stdin(
        cli(&temp, &config).args(["auth", "oauth-callback", "github", "--code-stdin"]),
        &format!("http://localhost:1420/auth/callback?state={nonce}&code=oauth-code-secret\n"),
    );
    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("oauth-code-secret"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("oauth-code-secret"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains(verifier));

    success_data(
        &cli(&temp, &config)
            .args(["auth", "revoke-session", "session-other", "--yes"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["auth", "logout"])
            .output()
            .unwrap(),
    );

    let output = run_with_stdin(
        cli(&temp, &config).args([
            "auth",
            "register",
            "--username",
            "new-user",
            "--email",
            "new@example.test",
            "--invite-code",
            "invite",
            "--password-stdin",
        ]),
        "register-password\n",
    );
    assert!(output.status.success());

    let stored: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    assert_eq!(stored["access_token"], "access-register");
    assert_eq!(stored["session_id"], "session-register");

    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 11);
    assert!(requests[0].starts_with("POST /api/commands/login_user "));
    assert_eq!(
        request_json_body(&requests[0])["input"]["username"],
        "tester"
    );
    assert_eq!(
        request_json_body(&requests[0])["input"]["password"],
        "login-password"
    );
    assert!(requests[1].starts_with("POST /api/commands/get_current_user "));
    assert!(requests[2].starts_with("POST /api/commands/refresh_token "));
    assert_eq!(
        request_json_body(&requests[2])["input"]["refresh_token"],
        "refresh-login"
    );
    assert!(requests[3].starts_with("POST /api/commands/update_user_profile "));
    assert_eq!(
        request_json_body(&requests[3])["input"]["email"],
        "new@example.test"
    );
    assert!(requests[4].starts_with("POST /api/commands/list_sessions "));
    assert!(requests[5].starts_with("POST /api/commands/oauth2_authorize_url "));
    assert_eq!(
        request_json_body(&requests[5])["input"]["provider"],
        "github"
    );
    assert!(requests[6].starts_with("POST /api/commands/change_user_password "));
    assert_eq!(
        request_json_body(&requests[6])["input"]["new_password"],
        "new-password"
    );
    assert!(requests[7].starts_with("POST /api/commands/oauth2_callback "));
    assert_eq!(
        request_json_body(&requests[7])["input"]["code"],
        "oauth-code-secret"
    );
    assert_eq!(
        request_json_body(&requests[7])["input"]["code_verifier"],
        verifier
    );
    {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
        use sha2::{Digest, Sha256};
        assert_eq!(
            request_json_body(&requests[5])["input"]["code_challenge"],
            URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
        );
        assert_eq!(request_json_body(&requests[5])["input"]["state"], nonce);
    }
    assert!(requests[8].starts_with("POST /api/commands/revoke_session "));
    assert_eq!(
        request_json_body(&requests[8])["input"]["session_id"],
        "session-other"
    );
    assert!(requests[9].starts_with("POST /api/commands/revoke_session "));
    assert_eq!(
        request_json_body(&requests[9])["input"]["session_id"],
        "session-oauth"
    );
    assert!(requests[10].starts_with("POST /api/commands/register_user "));
    assert_eq!(
        request_json_body(&requests[10])["input"]["invite_code"],
        "invite"
    );
}

#[test]
fn environment_auth_is_read_only_and_auth_errors_redact_stdin_secrets() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    write_config(&config, "http://127.0.0.1:9");

    let output = cli(&temp, &config)
        .env("A4N_ACCESS_TOKEN", "environment-token")
        .args(["auth", "logout"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let error = agent_error(&output);
    assert_eq!(error["error"]["code"], "invalid_input");
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("只读环境认证")
    );

    let (server, server_thread) = spawn_reply(
        "500 Internal Server Error",
        serde_json::to_vec(&json!({
            "code": "invalid_credentials",
            "message": "密码 rejected-password 无效",
            "retryable": false,
            "trace_id": "trace-test",
            "details": {}
        }))
        .unwrap(),
        None,
    );
    write_config(&config, &server);
    let output = run_with_stdin(
        cli(&temp, &config).args(["auth", "login", "--username", "tester", "--password-stdin"]),
        "rejected-password\n",
    );
    assert_eq!(output.status.code(), Some(7));
    let error = agent_error(&output);
    assert_eq!(error["error"]["code"], "invalid_credentials");
    let message = error["error"]["message"].as_str().unwrap();
    assert!(!message.contains("rejected-password"));
    assert!(message.contains("[REDACTED]"));
    assert!(
        server_thread
            .join()
            .unwrap()
            .starts_with("POST /api/commands/login_user ")
    );
}

#[test]
fn config_and_context_commands_persist_and_clear_hierarchical_state() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let context = temp.path().join("context.json");
    let (server, server_thread) = spawn_scripted_replies(vec![json_response(user_value())]);
    write_config(&config, &server);

    let shown = success_data(
        &cli(&temp, &config)
            .args(["config", "show"])
            .output()
            .unwrap(),
    );
    assert_eq!(shown["authenticated"], true);
    assert!(shown.get("access_token").is_none());

    let snapshot = json!({
        "schema_version": 2,
        "llm": {
            "id": 1,
            "name": "local",
            "provider": "openai",
            "base_url": "https://example.test/v1",
            "api_key_encrypted": "config-secret",
            "model": "model",
            "max_concurrent": 1,
            "timeout_secs": 30,
            "max_context_tokens": 4096,
            "max_output_tokens": 1024
        }
    });
    let snapshot_text = snapshot.to_string();
    success_data(
        &cli(&temp, &config)
            .args(["config", "runtime", "set", "--input"])
            .arg(snapshot_text)
            .output()
            .unwrap(),
    );
    let runtime = success_data(
        &cli(&temp, &config)
            .args(["config", "runtime", "show"])
            .output()
            .unwrap(),
    );
    assert_eq!(runtime["configured"], true);
    assert!(!runtime.to_string().contains("config-secret"));
    success_data(
        &cli(&temp, &config)
            .args(["config", "runtime", "clear"])
            .output()
            .unwrap(),
    );

    success_data(
        &cli(&temp, &config)
            .args([
                "--context",
                context.to_str().unwrap(),
                "context",
                "set",
                "--project-id",
                "7",
                "--chapter-id",
                "8",
                "--snapshot-id",
                "9",
            ])
            .output()
            .unwrap(),
    );
    let shown = success_data(
        &cli(&temp, &config)
            .args(["--context", context.to_str().unwrap(), "context", "show"])
            .output()
            .unwrap(),
    );
    assert_eq!(shown["project_id"], 7);
    assert_eq!(shown["chapter_id"], 8);
    assert_eq!(shown["snapshot_id"], 9);
    success_data(
        &cli(&temp, &config)
            .args([
                "--context",
                context.to_str().unwrap(),
                "context",
                "clear-snapshot",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "--context",
                context.to_str().unwrap(),
                "context",
                "clear-chapter",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "--context",
                context.to_str().unwrap(),
                "context",
                "clear-project",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["--context", context.to_str().unwrap(), "context", "clear"])
            .output()
            .unwrap(),
    );

    success_data(
        &cli(&temp, &config)
            .args(["config", "clear-auth"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["config", "set-server", "https://example.test/api"])
            .output()
            .unwrap(),
    );

    let stored: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    assert_eq!(stored["server"], "https://example.test/api");
    assert!(stored.get("access_token").is_none_or(Value::is_null));
    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("POST /api/commands/get_current_user "));
}

#[test]
fn raw_request_enforces_method_gate_auth_flag_and_atomic_file_output() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let output_path = temp.path().join("raw.bin");
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(json!({"method": "get"})),
        json_response(json!({"method": "post"})),
        json_response(json!({"auth": "omitted"})),
        binary_response(b"raw-bytes"),
    ]);
    write_config(&config, &server);

    let blocked = cli(&temp, &config)
        .args(["request", "post", "/unsafe", "--body", "{}"])
        .output()
        .unwrap();
    assert_eq!(blocked.status.code(), Some(10));
    assert_eq!(
        agent_error(&blocked)["error"]["code"],
        "destructive_confirmation_required"
    );

    success_data(
        &cli(&temp, &config)
            .args(["request", "get", "/health"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "request",
                "post",
                "/unsafe",
                "--body",
                r#"{"value":1}"#,
                "--allow-unsafe-request",
            ])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["request", "get", "/public", "--no-auth"])
            .output()
            .unwrap(),
    );
    let output = cli(&temp, &config)
        .args(["request", "get", "/download", "--output"])
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(fs::read(&output_path).unwrap(), b"raw-bytes");

    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests[0].starts_with("GET /api/health "));
    assert!(
        requests[0]
            .to_ascii_lowercase()
            .contains("authorization: bearer test-token")
    );
    assert!(requests[1].starts_with("POST /api/unsafe "));
    assert_eq!(request_json_body(&requests[1])["value"], 1);
    assert!(requests[2].starts_with("GET /api/public "));
    assert!(!requests[2].to_ascii_lowercase().contains("authorization:"));
    assert!(requests[3].starts_with("GET /api/download "));
}

#[test]
fn call_file_inputs_and_outputs_support_text_base64_force_and_reject_bad_base64() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let input_text = temp.path().join("input.txt");
    let input_binary = temp.path().join("input.bin");
    let base64_output = temp.path().join("artifact.bin");
    let text_output = temp.path().join("name.txt");
    let malformed_output = temp.path().join("malformed.bin");
    fs::write(&input_text, "来自文本文件").unwrap();
    fs::write(&input_binary, b"binary-input").unwrap();
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(json!({"data_base64": "aGVsbG8=", "file_name": "hello.txt"})),
        json_response(json!({"data_base64": "d29ybGQ=", "file_name": "world.txt"})),
        json_response(json!({"data_base64": "not-base64"})),
    ]);
    write_config(&config, &server);

    let mut command = cli(&temp, &config);
    command.args([
        "call",
        "preview_inspiration_request",
        "--args",
        r#"{"input":{"project_id":1,"message":"placeholder","data_base64":"placeholder"}}"#,
    ]);
    command
        .args(["--set-base64"])
        .arg(format!("/input/data_base64={}", input_binary.display()))
        .args(["--set-text"])
        .arg(format!("/input/message={}", input_text.display()))
        .args(["--save-base64"])
        .arg(format!("/data_base64={}", base64_output.display()))
        .args(["--save-text"])
        .arg(format!("/file_name={}", text_output.display()));
    let output = command.output().unwrap();
    let data = success_data(&output);
    assert_eq!(fs::read(&base64_output).unwrap(), b"hello");
    assert_eq!(fs::read_to_string(&text_output).unwrap(), "hello.txt");
    assert_eq!(data["data_base64"]["byte_size"], 5);
    assert_eq!(data["file_name"]["byte_size"], 9);

    let blocked = cli(&temp, &config)
        .args([
            "call",
            "preview_inspiration_request",
            "--args",
            r#"{"input":{}}"#,
            "--save-base64",
        ])
        .arg(format!("/data_base64={}", base64_output.display()))
        .output()
        .unwrap();
    assert_eq!(blocked.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("输出文件已存在"));

    let output = cli(&temp, &config)
        .args([
            "call",
            "preview_inspiration_request",
            "--args",
            r#"{"input":{}}"#,
            "--save-base64",
        ])
        .arg(format!("/data_base64={}", base64_output.display()))
        .args(["--force"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(fs::read(&base64_output).unwrap(), b"world");

    let output = cli(&temp, &config)
        .args([
            "call",
            "preview_inspiration_request",
            "--args",
            r#"{"input":{}}"#,
            "--save-base64",
        ])
        .arg(format!("/data_base64={}", malformed_output.display()))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(!malformed_output.exists());

    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 3);
    let first_body = request_json_body(&requests[0]);
    assert_eq!(first_body["input"]["message"], "来自文本文件");
    assert_eq!(first_body["input"]["data_base64"], "YmluYXJ5LWlucHV0");
}

#[test]
fn output_modes_preserve_raw_json_jsonl_human_and_compact_contracts() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    write_config(&config, "http://127.0.0.1:9");

    let json_output = cli_with_output(&temp, &config, "json")
        .args(["commands", "list_projects"])
        .output()
        .unwrap();
    assert!(json_output.status.success());
    let json_value: Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert!(
        json_value["api_commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("list_projects"))
    );

    let compact = cli_with_output(&temp, &config, "json")
        .args(["--compact", "commands", "list_projects"])
        .output()
        .unwrap();
    assert!(compact.status.success());
    assert_eq!(
        compact.stdout.iter().filter(|byte| **byte == b'\n').count(),
        1
    );
    assert!(!String::from_utf8_lossy(&compact.stdout).contains("\n  \""));

    let human = cli_with_output(&temp, &config, "human")
        .args(["commands", "list_projects"])
        .output()
        .unwrap();
    assert!(human.status.success());
    assert!(String::from_utf8_lossy(&human.stdout).contains("list_projects"));

    let (server, server_thread) = spawn_scripted_replies(vec![json_response(json!([
        {"id": 1},
        {"id": 2}
    ]))]);
    write_config(&config, &server);
    let jsonl = cli_with_output(&temp, &config, "jsonl")
        .args(["call", "list_projects", "--args", "{}"])
        .output()
        .unwrap();
    assert!(jsonl.status.success());
    let documents = output_documents(&jsonl);
    assert_eq!(documents, vec![json!({"id": 1}), json!({"id": 2})]);
    assert_eq!(server_thread.join().unwrap().len(), 1);
}

#[test]
fn http_error_statuses_map_to_documented_cli_exit_codes() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    for (status, code, expected_exit) in [
        ("401 Unauthorized", "unauthorized", 3),
        ("403 Forbidden", "forbidden", 4),
        ("404 Not Found", "not_found", 5),
        ("409 Conflict", "conflict", 6),
        ("429 Too Many Requests", "rate_limited", 7),
        ("500 Internal Server Error", "server_error", 7),
        ("422 Unprocessable Entity", "invalid_request", 8),
    ] {
        let (server, server_thread) = spawn_reply(status, api_error(status, code), None);
        write_config(&config, &server);
        let output = cli(&temp, &config)
            .args(["call", "list_projects", "--args", "{}"]) // no side effects
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(expected_exit), "{status}");
        let error = agent_error(&output);
        assert_eq!(error["error"]["code"], code);
        assert_eq!(
            error["error"]["http_status"],
            status.split(' ').next().unwrap().parse::<u64>().unwrap()
        );
        assert!(
            server_thread
                .join()
                .unwrap()
                .starts_with("POST /api/commands/list_projects ")
        );
    }
}

#[test]
fn task_watch_succeeds_reconnects_with_last_event_id_and_honors_after_sequence() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_scripted_replies(vec![
        sse_response(b"data: {\"sequence\":1,\"event\":{\"kind\":\"running\"}}\n\n"),
        sse_response(b"data: {\"sequence\":2,\"event\":{\"kind\":\"completed\"}}\n\n"),
    ]);
    write_config(&config, &server);
    let output = cli(&temp, &config)
        .args(["task", "watch", "task-1", "--after-sequence", "0"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let documents = output_documents(&output);
    assert_eq!(documents.len(), 2);
    assert_eq!(documents[0]["data"]["sequence"], 1);
    assert_eq!(documents[1]["data"]["event"]["kind"], "completed");
    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with("GET /api/generation-tasks/task-1/events?attempt=1 "));
    assert!(requests[1].starts_with("GET /api/generation-tasks/task-1/events?attempt=1 "));
    assert!(
        requests[1]
            .to_ascii_lowercase()
            .contains("last-event-id: 1:1")
    );

    let (server, server_thread) = spawn_scripted_replies(vec![sse_response(
        b"data: {\"sequence\":8,\"event\":{\"kind\":\"completed\"}}\n\n",
    )]);
    write_config(&config, &server);
    let output = cli(&temp, &config)
        .args(["task", "watch", "task-1", "--after-sequence", "7"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let request = server_thread.join().unwrap().pop().unwrap();
    assert!(request.to_ascii_lowercase().contains("last-event-id: 1:7"));
}

#[test]
fn task_watch_acknowledges_snapshot_and_reports_persisted_terminal_result() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let task_id = "00000000-0000-4000-8000-000000000017";
    let running = json!({
        "schema_version": 1, "task_id": task_id, "attempt": 1,
        "covers_through_sequence": 5, "status": "running",
        "started": {
            "task_kind": "llm_call", "delivery_mode": "stream",
            "project_id": 1, "root_project_id": 1,
            "target": {"kind": "chapter", "id": "1", "revision": null},
            "root_task_id": task_id, "parent_task_id": null, "retry_of_task_id": null,
            "generation_run_id": null, "workflow_run_id": null, "item_key": null,
            "batch_policy": null
        },
        "cancel_reason": null, "progress": null, "streams": [], "artifacts": [],
        "usage": null, "latest_checkpoint": null, "terminal_event": null, "updated_at_ms": 0
    });
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(running.clone()),
        sse_response(b"data: {\"sequence\":6,\"event\":{\"kind\":\"completed\"}}\n\n"),
    ]);
    write_config(&config, &server);
    let output = cli(&temp, &config)
        .args(["task", "watch", task_id])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output_documents(&output)[0]["data"]["covers_through_sequence"],
        5
    );
    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with(&format!(
        "GET /api/generation-tasks/{task_id}/snapshot?attempt=1 "
    )));
    assert!(
        requests[1]
            .to_ascii_lowercase()
            .contains("last-event-id: 1:5")
    );

    for (kind, data, expected_exit) in [
        (
            "completed",
            json!({
                "result_ref": null, "final_text_hashes": [], "final_usage": null,
                "finish_reason": "stop", "summary": "Done", "batch_summary": null
            }),
            0,
        ),
        (
            "failed",
            json!({
                "error": {"code": "provider_job_failed", "message": "Upstream rejected the request",
                    "retryable": false, "stage_code": "generate", "trace_id": null, "source_error": null},
                "draft_disposition": "retained_until_expiry", "batch_summary": null
            }),
            8,
        ),
    ] {
        let mut snapshot = running.clone();
        snapshot["status"] = json!(kind);
        snapshot["covers_through_sequence"] = json!(6);
        snapshot["terminal_event"] = json!({
            "schema_version": 1, "task_id": task_id, "attempt": 1,
            "sequence": 6, "emitted_at_ms": 0, "event": {"kind": kind, "data": data}
        });
        let (server, server_thread) = spawn_scripted_replies(vec![json_response(snapshot)]);
        write_config(&config, &server);
        let output = cli(&temp, &config)
            .args(["task", "watch", task_id])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(expected_exit),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output_documents(&output)[0]["data"]["event"]["kind"], kind);
        if kind == "failed" {
            assert_eq!(agent_error(&output)["error"]["code"], "task_failed");
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("Upstream rejected the request")
            );
        }
        assert_eq!(server_thread.join().unwrap().len(), 1);
    }
}

#[test]
fn workflow_watch_succeeds_and_timeout_is_reported() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(json!({"events": [{"id": "event-1", "message": "started"}]})),
        json_response(json!({"run": {"status": "completed", "id": 31}})),
    ]);
    write_config(&config, &server);
    let output = cli(&temp, &config)
        .args(["workflow", "watch", "31", "--interval-ms", "200"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let documents = output_documents(&output);
    assert_eq!(documents.len(), 2);
    assert_eq!(documents[0]["data"]["id"], "event-1");
    assert_eq!(documents[1]["data"]["run"]["status"], "completed");
    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 2);

    let timeout_replies = vec![
        json_response(json!({"events": []})),
        json_response(json!({"run": {"status": "running"}})),
        json_response(json!({"events": []})),
        json_response(json!({"run": {"status": "running"}})),
    ];
    let (server, server_thread) = spawn_scripted_replies(timeout_replies);
    write_config(&config, &server);
    let output = cli(&temp, &config)
        .args([
            "workflow",
            "watch",
            "31",
            "--interval-ms",
            "1000",
            "--timeout-seconds",
            "1",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(
        agent_error(&output)["error"]["code"],
        "workflow_watch_timeout"
    );
    let requests = server_thread.join().unwrap();
    assert!(requests.len() >= 2);
    assert_eq!(requests.len() % 2, 0);
}

#[test]
fn raw_request_accepts_file_and_stdin_bodies_and_routes_all_unsafe_methods() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let body_file = temp.path().join("request.json");
    fs::write(&body_file, br#"{"source":"file"}"#).unwrap();
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(json!({"method": "get"})),
        json_response(json!({"method": "post"})),
        json_response(json!({"method": "put"})),
        json_response(json!({"method": "patch"})),
        json_response(json!({"method": "delete"})),
    ]);
    write_config(&config, &server);

    success_data(
        &cli(&temp, &config)
            .args(["request", "get", "/get"])
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args(["request", "post", "/post", "--allow-unsafe-request"])
            .args(["--body"])
            .arg(format!("@{}", body_file.display()))
            .output()
            .unwrap(),
    );
    success_data(
        &cli(&temp, &config)
            .args([
                "request",
                "put",
                "/put",
                "--body",
                r#"{"source":"inline"}"#,
                "--allow-unsafe-request",
            ])
            .output()
            .unwrap(),
    );
    success_data(&run_with_stdin(
        cli(&temp, &config).args([
            "request",
            "patch",
            "/patch",
            "--body",
            "-",
            "--allow-unsafe-request",
        ]),
        r#"{"source":"stdin"}
"#,
    ));
    success_data(
        &cli(&temp, &config)
            .args([
                "request",
                "delete",
                "/delete",
                "--body",
                r#"{"source":"delete"}"#,
                "--allow-unsafe-request",
            ])
            .output()
            .unwrap(),
    );

    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 5);
    assert!(requests[0].starts_with("GET /api/get "));
    assert!(requests[1].starts_with("POST /api/post "));
    assert_eq!(request_json_body(&requests[1])["source"], "file");
    assert!(requests[2].starts_with("PUT /api/put "));
    assert_eq!(request_json_body(&requests[2])["source"], "inline");
    assert!(requests[3].starts_with("PATCH /api/patch "));
    assert_eq!(request_json_body(&requests[3])["source"], "stdin");
    assert!(requests[4].starts_with("DELETE /api/delete "));
    assert_eq!(request_json_body(&requests[4])["source"], "delete");
}

#[test]
fn raw_request_refuses_existing_output_before_network_access() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let output_path = temp.path().join("response.json");
    write_config(&config, "http://127.0.0.1:9");
    fs::write(&output_path, b"old-response").unwrap();

    let output = cli(&temp, &config)
        .args(["request", "get", "/response", "--output"])
        .arg(&output_path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("输出文件已存在"));
    assert_eq!(fs::read(&output_path).unwrap(), b"old-response");
}

#[test]
fn task_sse_handles_chunked_multiline_json_and_jsonl_output() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_chunked_response(
        "text/event-stream",
        vec![
            b": keep-alive\r\n\r\n".to_vec(),
            b"data: {\"sequence\":1,\r\n".to_vec(),
            b"data: \"event\":{\"kind\":\"completed\"}}\r\n\r\n".to_vec(),
        ],
    );
    write_config(&config, &server);

    let output = cli_with_output(&temp, &config, "jsonl")
        .args(["task", "watch", "task-1", "--after-sequence", "0"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        output_documents(&output),
        vec![json!({
            "sequence": 1,
            "event": {"kind": "completed"}
        })]
    );
    assert!(
        server_thread
            .join()
            .unwrap()
            .starts_with("GET /api/generation-tasks/task-1/events?attempt=1 ")
    );
}

#[test]
fn malformed_task_sse_frames_fail_without_retrying_as_success() {
    for body in [
        b"data: {not-json}\n\n".to_vec(),
        b"data: {\"event\":{\"kind\":\"completed\"}}\n\n".to_vec(),
    ] {
        let temp = TempDir::new().unwrap();
        let config = temp.path().join("cli.json");
        let (server, server_thread) = spawn_chunked_response("text/event-stream", vec![body]);
        write_config(&config, &server);
        let output = cli(&temp, &config)
            .args(["task", "watch", "task-1", "--after-sequence", "0"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(agent_error(&output)["error"]["code"], "invalid_input");
        assert!(
            server_thread
                .join()
                .unwrap()
                .starts_with("GET /api/generation-tasks/task-1/events?attempt=1 ")
        );
    }
}

#[test]
fn workflow_watch_deduplicates_repeated_event_ids_across_polls() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_scripted_replies(vec![
        json_response(json!({
            "events": [
                {"id": "event-1", "message": "one"},
                {"id": "event-2", "message": "two"}
            ]
        })),
        json_response(json!({"run": {"status": "running"}})),
        json_response(json!({
            "events": [
                {"id": "event-1", "message": "one-again"},
                {"id": "event-3", "message": "three"}
            ]
        })),
        json_response(json!({"run": {"status": "completed"}})),
    ]);
    write_config(&config, &server);

    let output = cli(&temp, &config)
        .args(["workflow", "watch", "31", "--interval-ms", "200"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let documents = output_documents(&output);
    assert_eq!(documents.len(), 4);
    assert_eq!(documents[0]["data"]["id"], "event-1");
    assert_eq!(documents[1]["data"]["id"], "event-2");
    assert_eq!(documents[2]["data"]["id"], "event-3");
    assert_eq!(documents[3]["data"]["run"]["status"], "completed");
    assert_eq!(server_thread.join().unwrap().len(), 4);
}

#[test]
fn cli_refreshes_expired_access_token_before_retrying_a_command() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_scripted_replies(vec![
        MockResponse {
            status: "401 Unauthorized",
            content_type: "application/json",
            body: api_error("401 Unauthorized", "unauthorized"),
            declared_length: None,
        },
        json_response(json!({
            "access_token": "access-new",
            "refresh_token": "refresh-new",
            "expires_in": 3600,
            "token_type": "Bearer",
            "session_id": "session-1"
        })),
        json_response(json!({"items": [{"id": 1}]})),
    ]);
    write_json(
        &config,
        &json!({
            "server": server,
            "access_token": "access-old",
            "refresh_token": "refresh-old",
            "session_id": "session-1"
        }),
    );

    let data = success_data(
        &cli(&temp, &config)
            .args(["call", "list_projects", "--args", "{}"])
            .output()
            .unwrap(),
    );
    assert_eq!(data["items"][0]["id"], 1);
    let stored: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    assert_eq!(stored["access_token"], "access-new");
    assert_eq!(stored["refresh_token"], "refresh-new");

    let requests = server_thread.join().unwrap();
    assert_eq!(requests.len(), 3);
    assert!(
        requests[0]
            .to_ascii_lowercase()
            .contains("authorization: bearer access-old")
    );
    assert!(requests[1].starts_with("POST /api/commands/refresh_token "));
    assert!(!requests[1].to_ascii_lowercase().contains("authorization:"));
    assert_eq!(
        request_json_body(&requests[1])["input"]["refresh_token"],
        "refresh-old"
    );
    assert!(
        requests[2]
            .to_ascii_lowercase()
            .contains("authorization: bearer access-new")
    );
}

#[test]
fn failed_agent_heartbeat_preserves_error_contract_and_headers() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_scripted_replies(vec![MockResponse {
        status: "500 Internal Server Error",
        content_type: "application/json",
        body: api_error("500 Internal Server Error", "agent_unavailable"),
        declared_length: None,
    }]);
    write_json(
        &config,
        &json!({
            "server": server,
            "access_token": "access-token",
            "agent_id": "agent-1",
            "agent_token": "lease-1"
        }),
    );

    let output = cli(&temp, &config)
        .args(["agent", "heartbeat", "--status", "running"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(agent_error(&output)["error"]["code"], "agent_unavailable");
    let request = server_thread.join().unwrap().pop().unwrap();
    assert!(request.starts_with("POST /api/agents/agent-1/heartbeat "));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer access-token")
    );
    assert!(
        request
            .to_ascii_lowercase()
            .contains("x-agent-token: lease-1")
    );
}

#[test]
fn local_commands_cover_empty_state_and_reject_remote_start() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    write_config(&config, "http://127.0.0.1:9");

    let status = success_data(
        &cli(&temp, &config)
            .args(["local", "status"])
            .output()
            .unwrap(),
    );
    assert_eq!(status["owned"], false);
    assert_eq!(status["running"], false);
    let stopped = success_data(
        &cli(&temp, &config)
            .args(["local", "stop"])
            .output()
            .unwrap(),
    );
    assert_eq!(stopped["already_stopped"], true);
    let logs = success_data(
        &cli(&temp, &config)
            .args(["local", "logs", "--lines", "1"])
            .output()
            .unwrap(),
    );
    assert!(logs["server"].as_array().unwrap().is_empty());
    assert!(logs["qdrant"].as_array().unwrap().is_empty());

    let invalid_lines = cli(&temp, &config)
        .args(["local", "logs", "--lines", "0"])
        .output()
        .unwrap();
    assert_eq!(invalid_lines.status.code(), Some(2));

    let remote_config = temp.path().join("remote-cli.json");
    write_config(&remote_config, "https://example.test");
    let remote_start = cli(&temp, &remote_config)
        .args(["local", "start"])
        .output()
        .unwrap();
    assert_eq!(remote_start.status.code(), Some(2));
    assert_eq!(agent_error(&remote_start)["error"]["code"], "invalid_input");
}

#[test]
fn local_start_rejects_conflicting_full_and_no_sidecars_flags() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    write_config(&config, "http://127.0.0.1:9");

    let output = cli(&temp, &config)
        .args(["local", "start", "--full", "--no-sidecars"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .to_ascii_lowercase()
            .contains("cannot be used with")
    );
}

#[test]
fn local_full_start_requires_a_source_workspace() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    write_config(&config, "http://127.0.0.1:9");

    let output = cli(&temp, &config)
        .env_remove("A4N_SOURCE_ROOT")
        .env("A4N_DATABASE_URL", "postgres://user:pass@127.0.0.1:1/a4n")
        .env("A4N_JWT_SECRET", "local-start-test-secret")
        .args(["local", "start", "--full"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let error = agent_error(&output);
    assert_eq!(error["error"]["code"], "invalid_input");
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("A4N 源码 workspace")
    );
}

#[test]
fn local_start_rejects_an_invalid_source_root_before_building() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let invalid_root = temp.path().join("not-an-a4n-workspace");
    write_config(&config, "http://127.0.0.1:9");

    let output = cli(&temp, &config)
        .env("A4N_SOURCE_ROOT", &invalid_root)
        .env("A4N_DATABASE_URL", "postgres://user:pass@127.0.0.1:1/a4n")
        .env("A4N_JWT_SECRET", "local-start-test-secret")
        .args(["local", "start", "--full"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let error = agent_error(&output);
    assert_eq!(error["error"]["code"], "invalid_input");
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("A4N_SOURCE_ROOT 不是 A4N workspace")
    );
}

#[test]
fn local_start_accepts_external_sidecars_and_qdrant_flags_together() {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("cli.json");
    let (server, server_thread) = spawn_reply(
        "200 OK",
        serde_json::to_vec(&json!({ "host": HOST_IDENTITY })).unwrap(),
        None,
    );
    write_config(&config, &server);

    let data = success_data(
        &cli(&temp, &config)
            .args(["local", "start", "--no-sidecars", "--no-qdrant"])
            .output()
            .unwrap(),
    );
    assert_eq!(data["owned"], false);
    assert_eq!(data["already_running"], true);
    assert!(server_thread.join().unwrap().starts_with("GET /api/meta "));
}
