fn live_child() -> Child {
        Command::new("sh")
            .args(["-c", "sleep 5"])
            .spawn()
            .expect("test child should start")
}
    fn exited_child() -> Child {
        Command::new("sh")
            .args(["-c", "exit 7"])
            .spawn()
            .expect("test child should start")
    }

    fn novel_planning_fixture(
        tempdir: &tempfile::TempDir,
    ) -> (Child, std::path::PathBuf, super::BinaryFingerprint) {
        let child = live_child();
        let binary = tempdir.path().join("a4n-novel-planning-worker");
        std::fs::write(&binary, b"novel-planning-worker")
            .expect("novel planning worker fixture should be written");
        let fingerprint =
            super::binary_fingerprint(&binary).expect("novel planning fingerprint should work");
        (child, binary, fingerprint)
    }

    fn artifact_media_fixture(
        tempdir: &tempfile::TempDir,
    ) -> (Child, std::path::PathBuf, super::BinaryFingerprint) {
        let child = live_child();
        let binary = tempdir.path().join("a4n-artifact-media-worker");
        std::fs::write(&binary, b"artifact-media-worker")
            .expect("artifact media worker fixture should be written");
        let fingerprint =
            super::binary_fingerprint(&binary).expect("artifact media fingerprint should work");
        (child, binary, fingerprint)
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
    fn reports_child_exit_while_edge_is_running() {
        let mut edge = live_child();
        let mut control = exited_child();
        let mut worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        std::fs::write(&control_binary, b"control").expect("control fixture should be written");
        let control_fingerprint =
            super::binary_fingerprint(&control_binary).expect("control fingerprint should work");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        std::fs::write(&worker_binary, b"worker").expect("worker fixture should be written");
        let worker_fingerprint =
            super::binary_fingerprint(&worker_binary).expect("worker fingerprint should work");
        let edge_binary = tempdir.path().join("a4n-edge");
        std::fs::write(&edge_binary, b"edge").expect("edge fixture should be written");
        let edge_fingerprint =
            super::binary_fingerprint(&edge_binary).expect("edge fingerprint should work");
        let (
            mut novel_planning_worker,
            novel_planning_worker_binary,
            novel_planning_worker_fingerprint,
        ) = novel_planning_fixture(&tempdir);
        let (
            mut artifact_media_worker,
            artifact_media_worker_binary,
            artifact_media_worker_fingerprint,
        ) = artifact_media_fixture(&tempdir);

        let result = super::wait_for_edge_or_child(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_planning_worker_binary,
            novel_planning_worker_fingerprint,
            &mut artifact_media_worker,
            &artifact_media_worker_binary,
            artifact_media_worker_fingerprint,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ControlExited(_))
        ));
        edge.kill().expect("test edge should stop");
        edge.wait().expect("test edge should be reaped");
        novel_planning_worker
            .kill()
            .expect("test novel planning worker should stop");
        novel_planning_worker
            .wait()
            .expect("test novel planning worker should be reaped");
        artifact_media_worker
            .kill()
            .expect("test artifact media worker should stop");
        artifact_media_worker
            .wait()
            .expect("test artifact media worker should be reaped");
        worker.kill().expect("test worker should stop");
        worker.wait().expect("test worker should be reaped");
    }
