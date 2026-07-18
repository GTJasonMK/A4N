#[test]
fn video_delete_file_paths_ignore_blank_optional_thumbnail() {
    let data_dir = unique_temp_dir("a4n_video_delete_blank_thumb");
    let video_path = data_dir.join("videos/chapter_1/video.mp4");
    std::fs::create_dir_all(video_path.parent().expect("video parent")).expect("create parent");
    std::fs::write(&video_path, b"video").expect("write video");

    let paths = video_delete_file_paths(
        &data_dir,
        Some("videos/chapter_1/video.mp4"),
        Some(" \n\t "),
    )
    .expect("blank optional thumbnail should be ignored");

    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].0, "视频文件");
    assert_eq!(
        paths[0].1,
        video_path.canonicalize().expect("canonical video")
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn video_delete_file_paths_deduplicate_video_and_thumbnail_same_file() {
    let data_dir = unique_temp_dir("a4n_video_delete_dedup");
    let video_path = data_dir.join("videos/chapter_1/video.mp4");
    std::fs::create_dir_all(video_path.parent().expect("video parent")).expect("create parent");
    std::fs::write(&video_path, b"video").expect("write video");

    let paths = video_delete_file_paths(
        &data_dir,
        Some("videos/chapter_1/video.mp4"),
        Some("videos/chapter_1/video.mp4"),
    )
    .expect("same relative file should resolve once");

    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].0, "视频文件");
    assert_eq!(
        paths[0].1,
        video_path.canonicalize().expect("canonical video")
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn video_delete_file_paths_skip_missing_files() {
    let data_dir = unique_temp_dir("a4n_video_delete_missing");
    std::fs::create_dir_all(&data_dir).expect("create data dir");

    let paths = video_delete_file_paths(
        &data_dir,
        Some("videos/chapter_1/missing.mp4"),
        Some("videos/chapter_1/missing-thumb.jpg"),
    )
    .expect("missing delete targets should be treated as already deleted");

    assert!(paths.is_empty());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn video_delete_file_paths_reject_existing_file_outside_data_dir() {
    let root = unique_temp_dir("a4n_video_delete_outside_root");
    let data_dir = root.join("data");
    let outside = root.join("outside.mp4");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::write(&outside, b"video").expect("write outside video");

    let err = video_delete_file_paths(&data_dir, Some(&outside.to_string_lossy()), None)
        .expect_err("existing outside file must not be accepted for deletion");

    assert!(err.contains("相对路径"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn video_delete_file_paths_reject_missing_traversal_path() {
    let root = unique_temp_dir("a4n_video_delete_missing_traversal");
    let data_dir = root.join("data");
    std::fs::create_dir_all(&data_dir).expect("create data dir");

    let err = video_delete_file_paths(&data_dir, Some("../missing.mp4"), None)
        .expect_err("missing traversal target must not be treated as already deleted");

    assert!(err.contains("不在应用数据目录内"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn video_delete_file_paths_reject_existing_directory() {
    let data_dir = unique_temp_dir("a4n_video_delete_directory");
    let directory = data_dir.join("videos/chapter_1/not_a_file.mp4");
    std::fs::create_dir_all(&directory).expect("create directory target");

    let err = video_delete_file_paths(&data_dir, Some("videos/chapter_1/not_a_file.mp4"), None)
        .expect_err("directory target must not be accepted as a file");

    assert_eq!(err, "视频文件不是文件");
    let _ = std::fs::remove_dir_all(&data_dir);
}
