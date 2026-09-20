
    #[test]
    fn detects_replaced_worker_binary() {
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        std::fs::write(&worker_binary, b"old-worker")
            .expect("old worker fixture should be written");
        let old = super::binary_fingerprint(&worker_binary).expect("old fingerprint should work");
        std::fs::write(&worker_binary, b"new-worker-with-a-different-size")
            .expect("new worker fixture should be written");
        let new = super::binary_fingerprint(&worker_binary).expect("new fingerprint should work");

        assert_ne!(old, new);
}
    #[test]
    fn control_binary_change_only_selects_control_replacement() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        std::fs::write(&control_binary, b"old-control").expect("control fixture should be written");
        std::fs::write(&worker_binary, b"worker").expect("worker fixture should be written");
        let control_fingerprint =
            super::binary_fingerprint(&control_binary).expect("control fingerprint should work");
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
        std::fs::write(&control_binary, b"new-control").expect("control replacement should work");

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
            Some(super::ChildWatchEvent::ControlChanged)
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
        control.kill().expect("test control should stop");
        control.wait().expect("test control should be reaped");
        worker.kill().expect("test worker should stop");
        worker.wait().expect("test worker should be reaped");
    }

    #[test]
    fn worker_binary_change_only_selects_worker_replacement() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        std::fs::write(&control_binary, b"control").expect("control fixture should be written");
        std::fs::write(&worker_binary, b"old-worker").expect("worker fixture should be written");
        let control_fingerprint =
            super::binary_fingerprint(&control_binary).expect("control fingerprint should work");
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
        std::fs::write(&worker_binary, b"new-worker").expect("worker replacement should work");

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
            Some(super::ChildWatchEvent::WorkerChanged)
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
        control.kill().expect("test control should stop");
        control.wait().expect("test control should be reaped");
        worker.kill().expect("test worker should stop");
        worker.wait().expect("test worker should be reaped");
    }
