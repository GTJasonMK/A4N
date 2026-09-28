use super::{
    OrphanProjectDataDirSummary, OrphanProjectDataDirsSummary,
    paths::path_to_string,
    scanning::{confirmed_orphan_project_data_dirs, orphan_project_data_dirs},
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
fn cleanup_selection_contains_only_confirmed_orphans() {
    let current = OrphanProjectDataDirsSummary {
        items: vec![
            OrphanProjectDataDirSummary {
                project_id: 2,
                path: "projects/2".to_string(),
                size_bytes: 4,
            },
            OrphanProjectDataDirSummary {
                project_id: 3,
                path: "projects/3".to_string(),
                size_bytes: 8,
            },
        ],
        total_count: 2,
        total_size_bytes: 12,
    };

    let selected =
        confirmed_orphan_project_data_dirs(&[2], &current).expect("select confirmed project only");

    assert_eq!(selected.total_count, 1);
    assert_eq!(selected.total_size_bytes, 4);
    assert_eq!(selected.items[0].project_id, 2);
}

#[test]
fn cleanup_selection_rejects_stale_and_invalid_ids() {
    let current = OrphanProjectDataDirsSummary {
        items: vec![OrphanProjectDataDirSummary {
            project_id: 2,
            path: "projects/2".to_string(),
            size_bytes: 4,
        }],
        total_count: 1,
        total_size_bytes: 4,
    };

    assert!(confirmed_orphan_project_data_dirs(&[3], &current).is_err());
    assert!(confirmed_orphan_project_data_dirs(&[], &current).is_err());
    assert!(confirmed_orphan_project_data_dirs(&[2, 2], &current).is_err());
    assert!(confirmed_orphan_project_data_dirs(&[-2], &current).is_err());
}
