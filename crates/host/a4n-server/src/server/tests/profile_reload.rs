#[test]
    fn edge_binary_change_only_selects_edge_replacement() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        std::fs::write(&control_binary, b"control").expect("control fixture should be written");
        std::fs::write(&worker_binary, b"worker").expect("worker fixture should be written");
        std::fs::write(&edge_binary, b"old-edge").expect("edge fixture should be written");
        let control_fingerprint =
            super::binary_fingerprint(&control_binary).expect("control fingerprint should work");
        let worker_fingerprint =
            super::binary_fingerprint(&worker_binary).expect("worker fingerprint should work");
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
        std::fs::write(&edge_binary, b"new-edge").expect("edge replacement should work");

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

        assert!(matches!(result, Some(super::ChildWatchEvent::EdgeChanged)));
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
    fn novel_planning_worker_binary_change_only_selects_novel_planning_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_media_worker_binary = tempdir.path().join("a4n-artifact-media-worker");
        std::fs::write(&control_binary, b"control").expect("control fixture should be written");
        std::fs::write(&worker_binary, b"worker").expect("worker fixture should be written");
        std::fs::write(&edge_binary, b"edge").expect("edge fixture should be written");
        std::fs::write(&novel_binary, b"old-novel").expect("novel fixture should be written");
        std::fs::write(&artifact_media_worker_binary, b"artifact-media-worker")
            .expect("artifact media fixture should be written");
        let control_fingerprint =
            super::binary_fingerprint(&control_binary).expect("control fingerprint should work");
        let worker_fingerprint =
            super::binary_fingerprint(&worker_binary).expect("worker fingerprint should work");
        let edge_fingerprint =
            super::binary_fingerprint(&edge_binary).expect("edge fingerprint should work");
        let novel_fingerprint =
            super::binary_fingerprint(&novel_binary).expect("novel fingerprint should work");
        let artifact_media_worker_fingerprint =
            super::binary_fingerprint(&artifact_media_worker_binary)
                .expect("artifact media fingerprint should work");
        std::fs::write(&novel_binary, b"new-novel").expect("novel replacement should be written");

        let result = super::wait_for_edge_or_child(
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
            Some(super::ChildWatchEvent::NovelPlanningWorkerChanged)
        ));
        edge.kill().expect("test edge should stop");
        edge.wait().expect("test edge should be reaped");
        control.kill().expect("test control should stop");
        control.wait().expect("test control should be reaped");
        worker.kill().expect("test worker should stop");
        worker.wait().expect("test worker should be reaped");
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
    }

    #[test]
    fn artifact_media_worker_binary_change_only_selects_artifact_media_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut artifact_media_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        std::fs::write(&control_binary, b"control").expect("control fixture should be written");
        std::fs::write(&worker_binary, b"worker").expect("worker fixture should be written");
        std::fs::write(&edge_binary, b"edge").expect("edge fixture should be written");
        let control_fingerprint =
            super::binary_fingerprint(&control_binary).expect("control fingerprint should work");
        let worker_fingerprint =
            super::binary_fingerprint(&worker_binary).expect("worker fingerprint should work");
        let edge_fingerprint =
            super::binary_fingerprint(&edge_binary).expect("edge fingerprint should work");
        let (mut novel_planning_worker, novel_binary, novel_fingerprint) =
            novel_planning_fixture(&tempdir);
        let artifact_media_worker_binary = tempdir.path().join("a4n-artifact-media-worker");
        std::fs::write(&artifact_media_worker_binary, b"old-artifact")
            .expect("artifact fixture should be written");
        let artifact_media_worker_fingerprint =
            super::binary_fingerprint(&artifact_media_worker_binary)
                .expect("artifact fingerprint should work");
        std::fs::write(&artifact_media_worker_binary, b"new-artifact")
            .expect("artifact replacement should be written");

        let result = super::wait_for_edge_or_child(
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
            Some(super::ChildWatchEvent::ArtifactMediaWorkerChanged)
        ));
        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }

    #[test]
    fn foreshadowing_worker_binary_change_only_selects_foreshadowing_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let mut foreshadowing_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_binary = tempdir.path().join("a4n-artifact-media-worker");
        let foreshadowing_binary = tempdir.path().join("a4n-foreshadowing-worker");
        for (path, content) in [
            (&control_binary, b"control".as_slice()),
            (&worker_binary, b"worker".as_slice()),
            (&edge_binary, b"edge".as_slice()),
            (&novel_binary, b"novel".as_slice()),
            (&artifact_binary, b"artifact".as_slice()),
            (&foreshadowing_binary, b"old-foreshadowing".as_slice()),
        ] {
            std::fs::write(path, content).expect("worker fixture should be written");
        }
        let control_fingerprint = super::binary_fingerprint(&control_binary).unwrap();
        let worker_fingerprint = super::binary_fingerprint(&worker_binary).unwrap();
        let edge_fingerprint = super::binary_fingerprint(&edge_binary).unwrap();
        let novel_fingerprint = super::binary_fingerprint(&novel_binary).unwrap();
        let artifact_fingerprint = super::binary_fingerprint(&artifact_binary).unwrap();
        let foreshadowing_fingerprint = super::binary_fingerprint(&foreshadowing_binary).unwrap();
        std::fs::write(&foreshadowing_binary, b"new-foreshadowing")
            .expect("foreshadowing replacement should be written");

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
            Some((
                &mut foreshadowing_worker,
                &foreshadowing_binary,
                foreshadowing_fingerprint,
            )),
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
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ForeshadowingWorkerChanged)
        ));
        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
            &mut foreshadowing_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }

    #[test]
    fn profile_worker_binary_changes_select_only_changed_profile_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let mut object_profile_worker = live_child();
        let mut scene_profile_worker = live_child();
        let mut profile_material_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_binary = tempdir.path().join("a4n-artifact-media-worker");
        let object_binary = tempdir.path().join("a4n-object-profile-worker");
        let scene_binary = tempdir.path().join("a4n-scene-profile-worker");
        let profile_material_binary = tempdir.path().join("a4n-profile-material-worker");
        for (path, content) in [
            (&control_binary, b"control".as_slice()),
            (&worker_binary, b"worker".as_slice()),
            (&edge_binary, b"edge".as_slice()),
            (&novel_binary, b"novel".as_slice()),
            (&artifact_binary, b"artifact".as_slice()),
            (&object_binary, b"old-object".as_slice()),
            (&scene_binary, b"old-scene".as_slice()),
            (&profile_material_binary, b"old-profile-material".as_slice()),
        ] {
            std::fs::write(path, content).expect("worker fixture should be written");
        }
        let control_fingerprint = super::binary_fingerprint(&control_binary).unwrap();
        let worker_fingerprint = super::binary_fingerprint(&worker_binary).unwrap();
        let edge_fingerprint = super::binary_fingerprint(&edge_binary).unwrap();
        let novel_fingerprint = super::binary_fingerprint(&novel_binary).unwrap();
        let artifact_fingerprint = super::binary_fingerprint(&artifact_binary).unwrap();
        let object_fingerprint = super::binary_fingerprint(&object_binary).unwrap();
        let scene_fingerprint = super::binary_fingerprint(&scene_binary).unwrap();
        let profile_material_fingerprint =
            super::binary_fingerprint(&profile_material_binary).unwrap();
        std::fs::write(&object_binary, b"new-object")
            .expect("object profile replacement should be written");

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
            Some((
                &mut object_profile_worker,
                &object_binary,
                object_fingerprint,
            )),
            Some((&mut scene_profile_worker, &scene_binary, scene_fingerprint)),
            Some((
                &mut profile_material_worker,
                &profile_material_binary,
                profile_material_fingerprint,
            )),
            None,
            None,
            None,
            None,
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
            Some(super::ChildWatchEvent::ObjectProfileWorkerChanged)
        ));

        let object_fingerprint = super::binary_fingerprint(&object_binary).unwrap();
        std::fs::write(&profile_material_binary, b"new-profile-material")
            .expect("profile material replacement should be written");
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
            Some((
                &mut object_profile_worker,
                &object_binary,
                object_fingerprint,
            )),
            Some((&mut scene_profile_worker, &scene_binary, scene_fingerprint)),
            Some((
                &mut profile_material_worker,
                &profile_material_binary,
                profile_material_fingerprint,
            )),
            None,
            None,
            None,
            None,
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
            Some(super::ChildWatchEvent::ProfileMaterialWorkerChanged)
        ));

        let profile_material_fingerprint =
            super::binary_fingerprint(&profile_material_binary).unwrap();
        std::fs::write(&scene_binary, b"new-scene")
            .expect("scene profile replacement should be written");
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
            Some((
                &mut object_profile_worker,
                &object_binary,
                object_fingerprint,
            )),
            Some((&mut scene_profile_worker, &scene_binary, scene_fingerprint)),
            Some((
                &mut profile_material_worker,
                &profile_material_binary,
                profile_material_fingerprint,
            )),
            None,
            None,
            None,
            None,
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
            Some(super::ChildWatchEvent::SceneProfileWorkerChanged)
        ));

        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
            &mut object_profile_worker,
            &mut scene_profile_worker,
            &mut profile_material_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }

    #[test]
    fn outline_and_blueprint_worker_changes_select_only_changed_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let mut outline_worker = live_child();
        let mut blueprint_worker = live_child();
        let mut asset_graph_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_binary = tempdir.path().join("a4n-artifact-media-worker");
        let outline_binary = tempdir.path().join("a4n-outline-worker");
        let blueprint_binary = tempdir.path().join("a4n-blueprint-worker");
        let asset_graph_binary = tempdir.path().join("a4n-asset-graph-worker");
        for (path, content) in [
            (&control_binary, b"control".as_slice()),
            (&worker_binary, b"worker".as_slice()),
            (&edge_binary, b"edge".as_slice()),
            (&novel_binary, b"novel".as_slice()),
            (&artifact_binary, b"artifact".as_slice()),
            (&outline_binary, b"old-outline".as_slice()),
            (&blueprint_binary, b"old-blueprint".as_slice()),
            (&asset_graph_binary, b"old-asset-graph".as_slice()),
        ] {
            std::fs::write(path, content).expect("worker fixture should be written");
        }
        let control_fingerprint = super::binary_fingerprint(&control_binary).unwrap();
        let worker_fingerprint = super::binary_fingerprint(&worker_binary).unwrap();
        let edge_fingerprint = super::binary_fingerprint(&edge_binary).unwrap();
        let novel_fingerprint = super::binary_fingerprint(&novel_binary).unwrap();
        let artifact_fingerprint = super::binary_fingerprint(&artifact_binary).unwrap();
        let outline_fingerprint = super::binary_fingerprint(&outline_binary).unwrap();
        let blueprint_fingerprint = super::binary_fingerprint(&blueprint_binary).unwrap();
        let asset_graph_fingerprint = super::binary_fingerprint(&asset_graph_binary).unwrap();
        std::fs::write(&outline_binary, b"new-outline").expect("outline replacement should write");

        let result = super::wait_for_edge_or_child_with_outline(
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
            Some((&mut outline_worker, &outline_binary, outline_fingerprint)),
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
            Some(super::ChildWatchEvent::OutlineWorkerChanged)
        ));

        std::fs::write(&blueprint_binary, b"new-blueprint")
            .expect("blueprint replacement should write");
        let outline_fingerprint = super::binary_fingerprint(&outline_binary).unwrap();
        let result = super::wait_for_edge_or_child_with_outline(
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
            Some((&mut outline_worker, &outline_binary, outline_fingerprint)),
            Some((
                &mut blueprint_worker,
                &blueprint_binary,
                blueprint_fingerprint,
            )),
            Some((
                &mut asset_graph_worker,
                &asset_graph_binary,
                asset_graph_fingerprint,
            )),
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::BlueprintWorkerChanged)
        ));
        std::fs::write(&asset_graph_binary, b"new-asset-graph")
            .expect("asset graph replacement should write");
        let blueprint_fingerprint = super::binary_fingerprint(&blueprint_binary).unwrap();
        let result = super::wait_for_edge_or_child_with_outline(
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
            Some((&mut outline_worker, &outline_binary, outline_fingerprint)),
            Some((
                &mut blueprint_worker,
                &blueprint_binary,
                blueprint_fingerprint,
            )),
            Some((
                &mut asset_graph_worker,
                &asset_graph_binary,
                asset_graph_fingerprint,
            )),
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");
        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::AssetGraphWorkerChanged)
        ));
        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
            &mut outline_worker,
            &mut blueprint_worker,
            &mut asset_graph_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }

    #[test]
    fn reload_marker_is_optional_and_detects_publication() {
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let marker = tempdir.path().join("runtime-workers.ready");

        assert_eq!(
            super::optional_binary_fingerprint(Some(&marker)).expect("missing marker is valid"),
            None
        );
        std::fs::write(&marker, b"1\n").expect("marker should be published");
        let first = super::optional_binary_fingerprint(Some(&marker))
            .expect("published marker should be fingerprinted")
            .expect("published marker should exist");
        std::fs::write(&marker, b"2\n").expect("marker should be republished");
        let second = super::optional_binary_fingerprint(Some(&marker))
            .expect("republished marker should be fingerprinted")
            .expect("republished marker should exist");

        assert_ne!(first, second);
    }
