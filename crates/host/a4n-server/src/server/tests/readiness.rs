fn live_child() -> Child {
    Command::new("sleep")
        .arg("5")
        .spawn()
        .expect("test child should start")
}

fn exited_child() -> Child {
    Command::new("sh")
        .args(["-c", "exit 7"])
        .spawn()
        .expect("test child should start")
}

#[test]
fn waits_for_control_listener() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
    let address = listener
        .local_addr()
        .expect("test listener should expose address");
    let mut child = live_child();

    wait_for_control(&mut child, address).expect("listener should be considered ready");

    child.kill().expect("test child should stop");
    child.wait().expect("test child should be reaped");
}

#[test]
fn reports_control_exit_before_listener() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
    let address = listener
        .local_addr()
        .expect("test listener should expose address");
    drop(listener);
    let mut child = exited_child();

    let error = wait_for_control(&mut child, address).expect_err("exited Control must fail");
    assert!(error.to_string().contains("listener 就绪前退出"));
}

#[test]
fn rejects_worker_capability_version_mismatch() {
    let capabilities = WorkerCapabilities {
        schema_version: a4n_worker_contract::WORKER_SCHEMA_VERSION,
        worker_id: "test-worker".into(),
        capabilities: vec![a4n_worker_contract::WorkerCapability {
            name: "test.read".into(),
            version: "test-v1".into(),
            enabled: true,
        }],
    };

    let error = validate_worker_capabilities("test-worker", &capabilities, &[], "test-v2")
        .expect_err("stale capability version must fail readiness");
    assert!(error.to_string().contains("版本不匹配"));
}

#[test]
fn accepts_matching_enabled_required_capability() {
    let capabilities = WorkerCapabilities {
        schema_version: a4n_worker_contract::WORKER_SCHEMA_VERSION,
        worker_id: "test-worker".into(),
        capabilities: vec![a4n_worker_contract::WorkerCapability {
            name: "test.read".into(),
            version: "test-v1".into(),
            enabled: true,
        }],
    };

    validate_worker_capabilities(
        "test-worker",
        &capabilities,
        &["test.read"],
        "test-v1",
    )
    .expect("matching enabled capability should pass readiness");
}
