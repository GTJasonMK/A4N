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
