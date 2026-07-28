use super::{
    paths::path_to_string,
    scanning::{managed_media_files, orphan_project_data_dirs},
};
use std::{
    collections::HashSet,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .expect("system clock must be after UNIX_EPOCH");
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
}

#[test]
fn path_to_string_uses_lossy_platform_path_display() {
    assert_eq!(
        path_to_string(PathBuf::from("data/projects")),
        "data/projects"
    );
}

#[test]
fn orphan_project_data_dirs_returns_numeric_dirs_missing_from_database() {
    let data_dir = unique_temp_dir("a4n_storage_orphans");
    let projects_dir = data_dir.join("projects");
    std::fs::create_dir_all(projects_dir.join("1")).expect("create live project dir");
    std::fs::create_dir_all(projects_dir.join("2/images")).expect("create orphan project dir");
    std::fs::write(projects_dir.join("2/images/ref.png"), b"demo").expect("write orphan file");
    std::fs::create_dir_all(projects_dir.join("notes")).expect("create nonnumeric dir");
    std::fs::write(projects_dir.join("3"), b"not a dir").expect("write numeric file");
    let existing = HashSet::from([1_i64]);

    let summary = orphan_project_data_dirs(&data_dir, &existing).expect("scan orphan project dirs");

    assert_eq!(summary.total_count, 1);
    assert_eq!(summary.items[0].project_id, 2);
    assert_eq!(summary.items[0].size_bytes, 4);

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn orphan_project_data_dirs_accepts_missing_projects_dir() {
    let data_dir = unique_temp_dir("a4n_storage_no_projects");

    let summary =
        orphan_project_data_dirs(&data_dir, &HashSet::new()).expect("missing dir is empty");

    assert_eq!(summary.total_count, 0);
    assert!(summary.items.is_empty());

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn managed_media_scan_skips_non_media_and_complete_assembly_cache() {
    let data_dir = unique_temp_dir("a4n_managed_media_scan");
    let project = data_dir.join("projects/7");
    for dir in ["images", "portraits", "audio", "media", "assets"] {
        std::fs::create_dir_all(project.join(dir)).expect("create managed media dir");
    }
    std::fs::create_dir_all(project.join("videos/chapter_1")).expect("create video dir");
    std::fs::create_dir_all(project.join("chapters")).expect("create chapter dir");
    std::fs::write(project.join("images/orphan.png"), b"image").expect("write image");
    std::fs::write(project.join("portraits/orphan.png"), b"portrait").expect("write portrait");
    std::fs::write(project.join("audio/orphan.mp3"), b"audio").expect("write audio");
    std::fs::write(project.join("media/orphan.bin"), b"media").expect("write media");
    std::fs::write(project.join("assets/orphan.bin"), b"asset").expect("write asset");
    std::fs::write(project.join("chapters/keep.md"), b"chapter").expect("write chapter");
    std::fs::write(
        project.join("videos/chapter_1/assembled_preview.mp4"),
        b"cache",
    )
    .expect("write cache output");
    std::fs::write(
        project.join("videos/chapter_1/assembled_preview.mp4.cache.json"),
        b"{}",
    )
    .expect("write cache manifest");
    std::fs::write(
        project.join("videos/chapter_1/assembled_broken.mp4"),
        b"partial",
    )
    .expect("write broken cache output");

    let files = managed_media_files(&data_dir).expect("scan managed media files");
    let paths = files
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec![
            "projects/7/assets/orphan.bin",
            "projects/7/audio/orphan.mp3",
            "projects/7/images/orphan.png",
            "projects/7/media/orphan.bin",
            "projects/7/portraits/orphan.png",
            "projects/7/videos/chapter_1/assembled_broken.mp4",
        ]
    );
    assert!(files.iter().all(|file| file.project_id == 7));
    assert!(files.iter().all(|file| file.version.len() == 64));

    let _ = std::fs::remove_dir_all(&data_dir);
}
