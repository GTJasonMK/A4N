//! 基于 git2 的提示词版本仓库
//!
//! 每个提示词作为仓库内的一个文件（`{name}.md`），每次写入触发一次提交。
//! 历史检索、按 commit sha 取旧版本都通过 git2 完成。
//!
//! 设计要点：
//! - 仓库通过 `open_or_init` 在 `data_dir/prompts.git` 自动建库
//! - 所有 git2 API 是同步阻塞的，async 调用方需自行用 `spawn_blocking` 包装
//! - 提交作者从环境/参数读取，避免依赖外部 git config

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use a4n_core::error::AppError;
use git2::{Index, Oid, Repository, Signature, Sort};
use serde::Serialize;

const ENCODED_PROMPT_FILENAME_PREFIX: &str = "__a4n_prompt_hex__";

/// 单条 commit 摘要
#[derive(Debug, Clone, Serialize)]
pub struct CommitInfo {
    /// 完整 SHA
    pub sha: String,
    /// commit message
    pub message: String,
    /// 作者签名 "name <email>"
    pub author: String,
    /// Unix 时间戳（秒）
    pub timestamp: i64,
}

/// 基于 git2 的提示词版本存储
pub struct GitPromptStore {
    repo_path: PathBuf,
    repo: Mutex<Repository>,
    author_name: String,
    author_email: String,
}

impl GitPromptStore {
    /// 打开已有仓库或初始化新仓库
    pub fn open_or_init(repo_path: PathBuf) -> Result<Self, AppError> {
        std::fs::create_dir_all(&repo_path).map_err(AppError::Io)?;
        let repo = match Repository::open(&repo_path) {
            Ok(r) => r,
            Err(_) => Repository::init(&repo_path)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("git init 失败: {e}")))?,
        };
        Ok(Self {
            repo_path,
            repo: Mutex::new(repo),
            author_name: "A4N".into(),
            author_email: "a4n@local".into(),
        })
    }

    /// 仓库绝对路径
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    fn file_path(&self, name: &str) -> PathBuf {
        self.repo_path.join(sanitize_filename(name))
    }

    /// 提交提示词（覆盖写文件 → stage → commit）
    ///
    /// 返回新 commit 的 SHA。如果新内容与最新版完全一致则跳过提交并返回当前 HEAD（或空仓库时返回空 String）。
    pub fn commit_prompt(
        &self,
        name: &str,
        content: &str,
        message: &str,
    ) -> Result<String, AppError> {
        self.commit_prompts(&[(name.to_string(), content.to_string())], message)
    }

    /// 批量提交提示词文件（覆盖写文件 → stage → commit）。
    ///
    /// 所有文件写入同一个 commit；任一写入或 Git 步骤失败时恢复本次写入前的工作区文件内容。
    pub fn commit_prompts(
        &self,
        prompts: &[(String, String)],
        message: &str,
    ) -> Result<String, AppError> {
        let repo = self.repo.lock().unwrap();
        let mut files = Vec::with_capacity(prompts.len());

        for (name, content) in prompts {
            let path = self.file_path(name);
            let index_path = path
                .strip_prefix(&self.repo_path)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("git prompt 路径不在仓库内: {e}")))?
                .to_path_buf();
            let existing = read_worktree_prompt_file(&path)?;
            let committed = read_head_prompt_file(&repo, &index_path)?;
            files.push(PromptFileChange {
                path,
                index_path,
                previous_content: existing,
                previous_committed_content: committed,
                next_content: content.clone(),
            });
        }

        if files.iter().all(|file| {
            file.previous_committed_content.as_deref() == Some(file.next_content.as_str())
        }) {
            restore_prompt_worktree_to_next_content(&files)?;
            let mut index = repo
                .index()
                .map_err(|e| AppError::Internal(anyhow::anyhow!("git index 打开失败: {e}")))?;
            for file in &files {
                index
                    .add_path(&file.index_path)
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("git stage 清理失败: {e}")))?;
            }
            index
                .write()
                .map_err(|e| AppError::Internal(anyhow::anyhow!("git index 清理失败: {e}")))?;
            if let Ok(head) = repo.head()
                && let Some(oid) = head.target()
            {
                return Ok(oid.to_string());
            }
            return Ok(String::new());
        }

        let sig = Signature::now(&self.author_name, &self.author_email)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("git signature 失败: {e}")))?;

        for file in &files {
            if let Err(err) = std::fs::write(&file.path, &file.next_content).map_err(AppError::Io) {
                restore_prompt_files(&files)?;
                return Err(err);
            }
        }

        let commit_result = (|| -> Result<String, AppError> {
            let mut index = repo
                .index()
                .map_err(|e| AppError::Internal(anyhow::anyhow!("git index 打开失败: {e}")))?;
            stage_prompt_file_changes(&mut index, &files)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("git stage 失败: {e}")))?;
            index
                .write()
                .map_err(|e| AppError::Internal(anyhow::anyhow!("git index 写入失败: {e}")))?;
            let tree_oid = index
                .write_tree()
                .map_err(|e| AppError::Internal(anyhow::anyhow!("git write_tree 失败: {e}")))?;
            let tree = repo
                .find_tree(tree_oid)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("git find_tree 失败: {e}")))?;

            let parents: Vec<git2::Commit> = match repo.head() {
                Ok(head) => match head.target() {
                    Some(oid) => vec![repo.find_commit(oid).map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("git find_commit 失败: {e}"))
                    })?],
                    None => vec![],
                },
                Err(_) => vec![],
            };
            let parents_refs: Vec<&git2::Commit> = parents.iter().collect();

            let commit_oid = repo
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents_refs)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("git commit 失败: {e}")))?;

            Ok(commit_oid.to_string())
        })();

        if let Err(err) = commit_result {
            restore_prompt_files(&files)?;
            let mut index = repo
                .index()
                .map_err(|e| AppError::Internal(anyhow::anyhow!("git index 打开失败: {e}")))?;
            restore_prompt_index(&mut index, &files)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("git stage 恢复失败: {e}")))?;
            index
                .write()
                .map_err(|e| AppError::Internal(anyhow::anyhow!("git index 恢复失败: {e}")))?;
            return Err(err);
        }

        commit_result
    }

    /// 列出提示词的提交历史（从新到旧）
    ///
    /// 仅返回影响 `{name}.md` 的 commit；空仓库返回空数组。
    pub fn history(&self, name: &str) -> Result<Vec<CommitInfo>, AppError> {
        let repo = self.repo.lock().unwrap();
        let head = match repo.head() {
            Ok(h) => h,
            Err(_) => return Ok(vec![]),
        };
        let head_oid = match head.target() {
            Some(o) => o,
            None => return Ok(vec![]),
        };

        let mut revwalk = repo
            .revwalk()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("git revwalk 失败: {e}")))?;
        revwalk
            .push(head_oid)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("git revwalk push 失败: {e}")))?;
        revwalk
            .set_sorting(Sort::TIME)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("git sort 失败: {e}")))?;

        let target = sanitize_filename(name);
        let mut out = Vec::new();

        for oid in revwalk {
            let oid = oid.map_err(|e| AppError::Internal(anyhow::anyhow!("revwalk: {e}")))?;
            let commit = repo
                .find_commit(oid)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("find_commit: {e}")))?;

            // 判定本次 commit 是否触及目标文件：与父对比
            let touches =
                if let Ok(parent) = commit.parent(0) {
                    let a = parent.tree().ok();
                    let b = commit.tree().ok();
                    match (a, b) {
                        (Some(at), Some(bt)) => {
                            let diff = repo.diff_tree_to_tree(Some(&at), Some(&bt), None).map_err(
                                |e| AppError::Internal(anyhow::anyhow!("diff_tree_to_tree: {e}")),
                            )?;
                            diff.deltas()
                                .filter_map(|d| d.new_file().path().or_else(|| d.old_file().path()))
                                .any(|p| p.to_string_lossy() == target.as_str())
                        }
                        _ => false,
                    }
                } else {
                    // 根提交：直接看 tree 是否包含目标
                    let bt = commit
                        .tree()
                        .map_err(|e| AppError::Internal(anyhow::anyhow!("tree: {e}")))?;
                    bt.get_path(Path::new(&target)).is_ok()
                };

            if touches {
                let author = commit.author();
                let message = commit.message().ok_or_else(|| {
                    AppError::Internal(anyhow::anyhow!("commit {oid} 缺少 UTF-8 message"))
                })?;
                let author_name = author.name().ok_or_else(|| {
                    AppError::Internal(anyhow::anyhow!("commit {oid} 缺少 author name"))
                })?;
                let author_email = author.email().ok_or_else(|| {
                    AppError::Internal(anyhow::anyhow!("commit {oid} 缺少 author email"))
                })?;
                out.push(CommitInfo {
                    sha: oid.to_string(),
                    message: message.to_string(),
                    author: format!("{author_name} <{author_email}>"),
                    timestamp: author.when().seconds(),
                });
            }
        }

        Ok(out)
    }

    /// 取得指定 commit 处的提示词内容
    pub fn get_at(&self, name: &str, sha: &str) -> Result<String, AppError> {
        let repo = self.repo.lock().unwrap();
        let oid =
            Oid::from_str(sha).map_err(|e| AppError::Internal(anyhow::anyhow!("非法 SHA: {e}")))?;
        let commit = repo
            .find_commit(oid)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("find_commit: {e}")))?;
        let tree = commit
            .tree()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("tree: {e}")))?;
        let target = sanitize_filename(name);
        let entry = tree
            .get_path(Path::new(&target))
            .map_err(|_| AppError::NotFound(format!("commit {sha} 内不存在 {target}")))?;
        let blob = repo
            .find_blob(entry.id())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("find_blob: {e}")))?;
        Ok(String::from_utf8_lossy(blob.content()).to_string())
    }

    /// 当前 HEAD 处提示词内容
    pub fn latest(&self, name: &str) -> Result<Option<String>, AppError> {
        let repo = self.repo.lock().unwrap();
        let target = sanitize_filename(name);
        read_head_prompt_file(&repo, Path::new(&target))
    }
}

fn sanitize_filename(name: &str) -> String {
    if is_plain_prompt_filename_name(name) {
        format!("{name}.md")
    } else {
        format!(
            "{ENCODED_PROMPT_FILENAME_PREFIX}{}.md",
            hex_encode(name.as_bytes())
        )
    }
}

fn is_plain_prompt_filename_name(name: &str) -> bool {
    !name.is_empty()
        && !name.ends_with(".md")
        && !name.starts_with(ENCODED_PROMPT_FILENAME_PREFIX)
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

struct PromptFileChange {
    path: PathBuf,
    index_path: PathBuf,
    previous_content: Option<String>,
    previous_committed_content: Option<String>,
    next_content: String,
}

fn stage_prompt_file_changes(
    index: &mut Index,
    files: &[PromptFileChange],
) -> Result<(), git2::Error> {
    for file in files {
        if file.previous_committed_content.as_deref() != Some(file.next_content.as_str()) {
            index.add_path(&file.index_path)?;
        }
    }
    Ok(())
}

fn restore_prompt_index(index: &mut Index, files: &[PromptFileChange]) -> Result<(), git2::Error> {
    for file in files {
        if file.previous_content.is_some() {
            index.add_path(&file.index_path)?;
        } else {
            index.remove_path(&file.index_path)?;
        }
    }
    Ok(())
}

fn restore_prompt_files(files: &[PromptFileChange]) -> Result<(), AppError> {
    for file in files {
        match &file.previous_content {
            Some(content) => std::fs::write(&file.path, content).map_err(AppError::Io)?,
            None => match std::fs::remove_file(&file.path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(AppError::Io(err)),
            },
        }
    }
    Ok(())
}

fn restore_prompt_worktree_to_next_content(files: &[PromptFileChange]) -> Result<(), AppError> {
    for file in files {
        if file.previous_content.as_deref() != Some(file.next_content.as_str()) {
            std::fs::write(&file.path, &file.next_content).map_err(AppError::Io)?;
        }
    }
    Ok(())
}

fn read_worktree_prompt_file(path: &Path) -> Result<Option<String>, AppError> {
    match std::fs::read_to_string(path) {
        Ok(existing) => Ok(Some(existing)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(AppError::Io(err)),
    }
}

fn read_head_prompt_file(repo: &Repository, index_path: &Path) -> Result<Option<String>, AppError> {
    let head = match repo.head() {
        Ok(head) => head,
        Err(_) => return Ok(None),
    };
    let Some(head_oid) = head.target() else {
        return Ok(None);
    };
    let commit = repo
        .find_commit(head_oid)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("git find_commit 失败: {e}")))?;
    let tree = commit
        .tree()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("git tree 读取失败: {e}")))?;
    let entry = match tree.get_path(index_path) {
        Ok(entry) => entry,
        Err(_) => return Ok(None),
    };
    let blob = repo
        .find_blob(entry.id())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("git blob 读取失败: {e}")))?;
    Ok(Some(String::from_utf8_lossy(blob.content()).to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, GitPromptStore) {
        let tmp = tempfile::tempdir().unwrap();
        let s = GitPromptStore::open_or_init(tmp.path().to_path_buf()).unwrap();
        (tmp, s)
    }

    #[test]
    fn commit_and_history_basic() {
        let (_tmp, s) = store();
        let sha1 = s
            .commit_prompt("chapter_writing", "v1 内容", "首次提交")
            .unwrap();
        assert!(!sha1.is_empty());
        let sha2 = s
            .commit_prompt("chapter_writing", "v2 内容", "修订正文")
            .unwrap();
        assert_ne!(sha1, sha2);

        let log = s.history("chapter_writing").unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].sha, sha2);
        assert_eq!(log[1].sha, sha1);
    }

    #[test]
    fn get_at_old_sha_returns_old_content() {
        let (_tmp, s) = store();
        let sha1 = s.commit_prompt("p1", "AAA", "init").unwrap();
        s.commit_prompt("p1", "BBB", "update").unwrap();
        assert_eq!(s.get_at("p1", &sha1).unwrap(), "AAA");
        assert_eq!(s.latest("p1").unwrap().unwrap(), "BBB");
    }

    #[test]
    fn identical_content_skips_commit() {
        let (_tmp, s) = store();
        let sha1 = s.commit_prompt("same", "X", "init").unwrap();
        let sha2 = s.commit_prompt("same", "X", "noop").unwrap();
        assert_eq!(sha1, sha2);
        let log = s.history("same").unwrap();
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn unrelated_files_not_in_history() {
        let (_tmp, s) = store();
        s.commit_prompt("a", "1", "a init").unwrap();
        s.commit_prompt("b", "2", "b init").unwrap();
        assert_eq!(s.history("a").unwrap().len(), 1);
        assert_eq!(s.history("b").unwrap().len(), 1);
    }

    #[test]
    fn commit_prompt_does_not_stage_unrelated_dirty_repo_files() {
        let (_tmp, s) = store();
        std::fs::write(s.repo_path().join("manual.md"), "manual").unwrap();

        s.commit_prompt("tracked", "tracked", "tracked init")
            .unwrap();

        let repo = Repository::open(s.repo_path()).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let tree = head.tree().unwrap();
        assert!(tree.get_path(Path::new("tracked.md")).is_ok());
        assert!(tree.get_path(Path::new("manual.md")).is_err());
    }

    #[test]
    fn latest_reads_committed_head_not_dirty_worktree_file() {
        let (_tmp, s) = store();
        s.commit_prompt("dirty", "committed", "init").unwrap();
        std::fs::write(s.repo_path().join("dirty.md"), "uncommitted").unwrap();

        assert_eq!(s.latest("dirty").unwrap().as_deref(), Some("committed"));
    }

    #[test]
    fn commit_prompt_records_dirty_worktree_content_when_head_differs() {
        let (_tmp, s) = store();
        let first = s.commit_prompt("dirty", "committed", "init").unwrap();
        std::fs::write(s.repo_path().join("dirty.md"), "uncommitted").unwrap();

        let second = s
            .commit_prompt("dirty", "uncommitted", "commit dirty")
            .unwrap();

        assert_ne!(first, second);
        assert_eq!(s.latest("dirty").unwrap().as_deref(), Some("uncommitted"));
        assert_eq!(s.history("dirty").unwrap().len(), 2);
    }

    #[test]
    fn prompt_names_with_same_sanitized_shape_keep_isolated_files_and_history() {
        let (_tmp, s) = store();

        s.commit_prompt("foo/bar", "slash content", "slash init")
            .unwrap();
        s.commit_prompt("foo_bar", "underscore content", "underscore init")
            .unwrap();

        assert_eq!(
            s.latest("foo/bar").unwrap().as_deref(),
            Some("slash content")
        );
        assert_eq!(
            s.latest("foo_bar").unwrap().as_deref(),
            Some("underscore content")
        );
        assert_eq!(s.history("foo/bar").unwrap().len(), 1);
        assert_eq!(s.history("foo_bar").unwrap().len(), 1);
    }

    #[test]
    fn prompt_names_ending_with_md_do_not_collide_with_extensionless_names() {
        let (_tmp, s) = store();

        s.commit_prompt("foo", "plain content", "plain init")
            .unwrap();
        s.commit_prompt("foo.md", "md content", "md init").unwrap();

        assert_eq!(s.latest("foo").unwrap().as_deref(), Some("plain content"));
        assert_eq!(s.latest("foo.md").unwrap().as_deref(), Some("md content"));
        assert_eq!(s.history("foo").unwrap().len(), 1);
        assert_eq!(s.history("foo.md").unwrap().len(), 1);
    }
}
