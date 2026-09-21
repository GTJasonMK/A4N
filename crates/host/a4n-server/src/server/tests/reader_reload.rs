
    #[test]
    fn marker_change_without_binary_change_does_not_select_a_process() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut artifact_media_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let marker = tempdir.path().join("runtime-workers.ready");
        std::fs::write(&control_binary, b"control").expect("control fixture should be written");
        std::fs::write(&worker_binary, b"worker").expect("worker fixture should be written");
        let control_fingerprint =
            super::binary_fingerprint(&control_binary).expect("control fingerprint should work");
        let worker_fingerprint =
            super::binary_fingerprint(&worker_binary).expect("worker fingerprint should work");
        let edge_binary = tempdir.path().join("a4n-edge");
        let artifact_media_worker_binary = tempdir.path().join("a4n-artifact-media-worker");
        std::fs::write(&edge_binary, b"edge").expect("edge fixture should be written");
        let edge_fingerprint =
            super::binary_fingerprint(&edge_binary).expect("edge fingerprint should work");
        std::fs::write(&artifact_media_worker_binary, b"artifact-media-worker")
            .expect("artifact media fixture should be written");
        let artifact_media_worker_fingerprint =
            super::binary_fingerprint(&artifact_media_worker_binary)
                .expect("artifact media fingerprint should work");
        let (
            mut novel_planning_worker,
            novel_planning_worker_binary,
            novel_planning_worker_fingerprint,
        ) = novel_planning_fixture(&tempdir);
        let marker_fingerprint = None;
        std::fs::write(&marker, b"published").expect("marker should be published");

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
            Some(&marker),
            marker_fingerprint,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ReloadMarkerChanged)
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
    fn theme_read_worker_binary_changes_select_only_theme_read_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let mut theme_read_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_binary = tempdir.path().join("a4n-artifact-media-worker");
        let theme_binary = tempdir.path().join("a4n-theme-read-worker");
        for (path, content) in [
            (&control_binary, b"control".as_slice()),
            (&worker_binary, b"worker".as_slice()),
            (&edge_binary, b"edge".as_slice()),
            (&novel_binary, b"novel".as_slice()),
            (&artifact_binary, b"artifact".as_slice()),
            (&theme_binary, b"old-theme".as_slice()),
        ] {
            std::fs::write(path, content).expect("worker fixture should be written");
        }
        let control_fingerprint = super::binary_fingerprint(&control_binary).unwrap();
        let worker_fingerprint = super::binary_fingerprint(&worker_binary).unwrap();
        let edge_fingerprint = super::binary_fingerprint(&edge_binary).unwrap();
        let novel_fingerprint = super::binary_fingerprint(&novel_binary).unwrap();
        let artifact_fingerprint = super::binary_fingerprint(&artifact_binary).unwrap();
        let theme_fingerprint = super::binary_fingerprint(&theme_binary).unwrap();
        std::fs::write(&theme_binary, b"new-theme").expect("theme replacement should be written");

        let result = super::wait_for_edge_or_child_with_workers(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_binary,
            artifact_fingerprint,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some((&mut theme_read_worker, &theme_binary, theme_fingerprint)),
            None,
            None,
            None,
            None,
            None,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ThemeReadWorkerChanged)
        ));
        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
            &mut theme_read_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }

    #[test]
    fn project_read_worker_binary_changes_select_only_project_read_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let mut project_read_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_binary = tempdir.path().join("a4n-artifact-media-worker");
        let project_binary = tempdir.path().join("a4n-project-read-worker");
        for (path, content) in [
            (&control_binary, b"control".as_slice()),
            (&worker_binary, b"worker".as_slice()),
            (&edge_binary, b"edge".as_slice()),
            (&novel_binary, b"novel".as_slice()),
            (&artifact_binary, b"artifact".as_slice()),
            (&project_binary, b"old-project".as_slice()),
        ] {
            std::fs::write(path, content).expect("worker fixture should be written");
        }
        let control_fingerprint = super::binary_fingerprint(&control_binary).unwrap();
        let worker_fingerprint = super::binary_fingerprint(&worker_binary).unwrap();
        let edge_fingerprint = super::binary_fingerprint(&edge_binary).unwrap();
        let novel_fingerprint = super::binary_fingerprint(&novel_binary).unwrap();
        let artifact_fingerprint = super::binary_fingerprint(&artifact_binary).unwrap();
        let project_fingerprint = super::binary_fingerprint(&project_binary).unwrap();
        std::fs::write(&project_binary, b"new-project")
            .expect("project read replacement should be written");

        let result = super::wait_for_edge_or_child_with_workers(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_binary,
            artifact_fingerprint,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some((
                &mut project_read_worker,
                &project_binary,
                project_fingerprint,
            )),
            None,
            None,
            None,
            None,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ProjectReadWorkerChanged)
        ));
        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
            &mut project_read_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }

    #[test]
    fn import_review_read_worker_binary_changes_select_only_import_review_read_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let mut import_review_read_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_binary = tempdir.path().join("a4n-artifact-media-worker");
        let import_review_binary = tempdir.path().join("a4n-import-review-worker");
        for (path, content) in [
            (&control_binary, b"control".as_slice()),
            (&worker_binary, b"worker".as_slice()),
            (&edge_binary, b"edge".as_slice()),
            (&novel_binary, b"novel".as_slice()),
            (&artifact_binary, b"artifact".as_slice()),
            (&import_review_binary, b"old-import-review".as_slice()),
        ] {
            std::fs::write(path, content).expect("worker fixture should be written");
        }
        let control_fingerprint = super::binary_fingerprint(&control_binary).unwrap();
        let worker_fingerprint = super::binary_fingerprint(&worker_binary).unwrap();
        let edge_fingerprint = super::binary_fingerprint(&edge_binary).unwrap();
        let novel_fingerprint = super::binary_fingerprint(&novel_binary).unwrap();
        let artifact_fingerprint = super::binary_fingerprint(&artifact_binary).unwrap();
        let import_review_fingerprint = super::binary_fingerprint(&import_review_binary).unwrap();
        std::fs::write(&import_review_binary, b"new-import-review")
            .expect("import review replacement should be written");

        let result = super::wait_for_edge_or_child_with_workers(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_binary,
            artifact_fingerprint,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some((
                &mut import_review_read_worker,
                &import_review_binary,
                import_review_fingerprint,
            )),
            None,
            None,
            None,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ImportReviewReadWorkerChanged)
        ));
        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
            &mut import_review_read_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }

    #[test]
    fn inspiration_history_worker_binary_changes_select_only_inspiration_history_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let mut inspiration_history_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_binary = tempdir.path().join("a4n-artifact-media-worker");
        let inspiration_binary = tempdir.path().join("a4n-inspiration-worker");
        for (path, content) in [
            (&control_binary, b"control".as_slice()),
            (&worker_binary, b"worker".as_slice()),
            (&edge_binary, b"edge".as_slice()),
            (&novel_binary, b"novel".as_slice()),
            (&artifact_binary, b"artifact".as_slice()),
            (&inspiration_binary, b"old-inspiration".as_slice()),
        ] {
            std::fs::write(path, content).expect("worker fixture should be written");
        }
        let control_fingerprint = super::binary_fingerprint(&control_binary).unwrap();
        let worker_fingerprint = super::binary_fingerprint(&worker_binary).unwrap();
        let edge_fingerprint = super::binary_fingerprint(&edge_binary).unwrap();
        let novel_fingerprint = super::binary_fingerprint(&novel_binary).unwrap();
        let artifact_fingerprint = super::binary_fingerprint(&artifact_binary).unwrap();
        let inspiration_fingerprint = super::binary_fingerprint(&inspiration_binary).unwrap();
        std::fs::write(&inspiration_binary, b"new-inspiration")
            .expect("inspiration replacement should be written");

        let result = super::wait_for_edge_or_child_with_workers(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_binary,
            artifact_fingerprint,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some((
                &mut inspiration_history_worker,
                &inspiration_binary,
                inspiration_fingerprint,
            )),
            None,
            None,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::InspirationHistoryWorkerChanged)
        ));
        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
            &mut inspiration_history_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }
