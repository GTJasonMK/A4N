use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use git2::Repository;

use crate::{GitPromptStore, backup_prompt_repository, restore_prompt_repository};

fn store() -> (tempfile::TempDir, GitPromptStore) {
    let tmp = tempfile::tempdir().unwrap();
    let store = GitPromptStore::open_or_init(tmp.path().to_path_buf()).unwrap();
    (tmp, store)
}

#[test]
fn commit_and_history_basic() {
    let (_tmp, store) = store();
    let sha1 = store
        .commit_prompt("chapter_writing", "v1 内容", "首次提交")
        .unwrap();
    assert!(!sha1.is_empty());
    let sha2 = store
        .commit_prompt("chapter_writing", "v2 内容", "修订正文")
        .unwrap();
    assert_ne!(sha1, sha2);

    let log = store.history("chapter_writing").unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].sha, sha2);
    assert_eq!(log[1].sha, sha1);
}

#[test]
fn get_at_old_sha_returns_old_content() {
    let (_tmp, store) = store();
    let sha1 = store.commit_prompt("p1", "AAA", "init").unwrap();
    store.commit_prompt("p1", "BBB", "update").unwrap();
    assert_eq!(store.get_at("p1", &sha1).unwrap(), "AAA");
    assert_eq!(store.latest("p1").unwrap().unwrap(), "BBB");
}

#[test]
fn identical_content_skips_commit() {
    let (_tmp, store) = store();
    let sha1 = store.commit_prompt("same", "X", "init").unwrap();
    let sha2 = store.commit_prompt("same", "X", "noop").unwrap();
    assert_eq!(sha1, sha2);
    let log = store.history("same").unwrap();
    assert_eq!(log.len(), 1);
}

#[test]
fn unrelated_files_not_in_history() {
    let (_tmp, store) = store();
    store.commit_prompt("a", "1", "a init").unwrap();
    store.commit_prompt("b", "2", "b init").unwrap();
    assert_eq!(store.history("a").unwrap().len(), 1);
    assert_eq!(store.history("b").unwrap().len(), 1);
}

#[test]
fn commit_prompt_does_not_stage_unrelated_dirty_repo_files() {
    let (_tmp, store) = store();
    std::fs::write(store.repo_path().join("manual.md"), "manual").unwrap();

    store
        .commit_prompt("tracked", "tracked", "tracked init")
        .unwrap();

    let repo = Repository::open(store.repo_path()).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = head.tree().unwrap();
    assert!(tree.get_path(Path::new("tracked.md")).is_ok());
    assert!(tree.get_path(Path::new("manual.md")).is_err());
}

#[test]
fn latest_reads_committed_head_not_dirty_worktree_file() {
    let (_tmp, store) = store();
    store.commit_prompt("dirty", "committed", "init").unwrap();
    std::fs::write(store.repo_path().join("dirty.md"), "uncommitted").unwrap();

    assert_eq!(store.latest("dirty").unwrap().as_deref(), Some("committed"));
}

#[test]
fn commit_prompt_records_dirty_worktree_content_when_head_differs() {
    let (_tmp, store) = store();
    let first = store.commit_prompt("dirty", "committed", "init").unwrap();
    std::fs::write(store.repo_path().join("dirty.md"), "uncommitted").unwrap();

    let second = store
        .commit_prompt("dirty", "uncommitted", "commit dirty")
        .unwrap();

    assert_ne!(first, second);
    assert_eq!(
        store.latest("dirty").unwrap().as_deref(),
        Some("uncommitted")
    );
    assert_eq!(store.history("dirty").unwrap().len(), 2);
}

#[test]
fn prompt_names_with_same_sanitized_shape_keep_isolated_files_and_history() {
    let (_tmp, store) = store();

    store
        .commit_prompt("foo/bar", "slash content", "slash init")
        .unwrap();
    store
        .commit_prompt("foo_bar", "underscore content", "underscore init")
        .unwrap();

    assert_eq!(
        store.latest("foo/bar").unwrap().as_deref(),
        Some("slash content")
    );
    assert_eq!(
        store.latest("foo_bar").unwrap().as_deref(),
        Some("underscore content")
    );
    assert_eq!(store.history("foo/bar").unwrap().len(), 1);
    assert_eq!(store.history("foo_bar").unwrap().len(), 1);
}

#[test]
fn prompt_names_ending_with_md_do_not_collide_with_extensionless_names() {
    let (_tmp, store) = store();

    store
        .commit_prompt("foo", "plain content", "plain init")
        .unwrap();
    store
        .commit_prompt("foo.md", "md content", "md init")
        .unwrap();

    assert_eq!(
        store.latest("foo").unwrap().as_deref(),
        Some("plain content")
    );
    assert_eq!(
        store.latest("foo.md").unwrap().as_deref(),
        Some("md content")
    );
    assert_eq!(store.history("foo").unwrap().len(), 1);
    assert_eq!(store.history("foo.md").unwrap().len(), 1);
}

#[test]
fn poisoned_repository_lock_is_reported_as_error() {
    let (_tmp, store) = store();
    let poison_result = catch_unwind(AssertUnwindSafe(|| {
        let _guard = store
            .repo
            .lock()
            .expect("repository lock should start healthy");
        panic!("poison repository lock for error-path coverage");
    }));
    assert!(poison_result.is_err());

    let error = store
        .history("chapter_writing")
        .expect_err("poisoned repository lock must fail explicitly");
    assert!(error.to_string().contains("git 仓库锁已中毒"));
}

#[test]
fn corrupted_repository_is_not_reinitialized() {
    let tmp = tempfile::tempdir().unwrap();
    let git_dir = tmp.path().join(".git");
    std::fs::create_dir(&git_dir).unwrap();
    std::fs::write(git_dir.join("HEAD"), "not a valid symbolic ref\n").unwrap();

    let error = match GitPromptStore::open_or_init(tmp.path().to_path_buf()) {
        Ok(_) => panic!("corrupted repository must not be silently reinitialized"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("git 仓库 HEAD 校验失败"));
}

#[test]
fn backup_and_restore_preserve_committed_history() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("prompts.git");
    let backup = root.path().join("backup");
    let store = GitPromptStore::open_or_init(source.clone()).unwrap();
    store.commit_prompt("greeting", "v1", "init").unwrap();
    store.commit_prompt("greeting", "v2", "update").unwrap();

    backup_prompt_repository(&source, &backup).unwrap();
    store.commit_prompt("greeting", "v3", "later").unwrap();
    drop(store);
    restore_prompt_repository(&backup, &source).unwrap();

    let restored = GitPromptStore::open_or_init(source.clone()).unwrap();
    assert_eq!(restored.latest("greeting").unwrap().as_deref(), Some("v2"));
    assert_eq!(restored.history("greeting").unwrap().len(), 2);
    assert!(backup_prompt_repository(&source, &source.join("nested-backup")).is_err());
}
