use std::path::Path;

use a4n_core::error::AppError;
use a4n_prompt::version_store::CommitInfo;
use git2::{ErrorCode, Oid, Sort};

use crate::files::read_head_prompt_file;
use crate::naming::sanitize_filename;
use crate::store::GitPromptStore;

impl GitPromptStore {
    /// 列出提示词的提交历史（从新到旧）
    ///
    /// 仅返回影响 `{name}.md` 的 commit；空仓库返回空数组。
    pub fn history(&self, name: &str) -> Result<Vec<CommitInfo>, AppError> {
        let repo = self.lock_repo()?;
        let head = match repo.head() {
            Ok(head) => head,
            Err(err) if matches!(err.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => {
                return Ok(vec![]);
            }
            Err(err) => {
                return Err(AppError::Internal(anyhow::anyhow!(
                    "git HEAD 读取失败: {err}"
                )));
            }
        };
        let head_oid = match head.target() {
            Some(oid) => oid,
            None => return Ok(vec![]),
        };

        let mut revwalk = repo
            .revwalk()
            .map_err(|err| AppError::Internal(anyhow::anyhow!("git revwalk 失败: {err}")))?;
        revwalk
            .push(head_oid)
            .map_err(|err| AppError::Internal(anyhow::anyhow!("git revwalk push 失败: {err}")))?;
        revwalk
            .set_sorting(Sort::TIME)
            .map_err(|err| AppError::Internal(anyhow::anyhow!("git sort 失败: {err}")))?;

        let target = sanitize_filename(name);
        let mut out = Vec::new();

        for oid in revwalk {
            let oid = oid.map_err(|err| AppError::Internal(anyhow::anyhow!("revwalk: {err}")))?;
            let commit = repo
                .find_commit(oid)
                .map_err(|err| AppError::Internal(anyhow::anyhow!("find_commit: {err}")))?;

            let touches = match commit.parent(0) {
                Ok(parent) => {
                    let previous_tree = parent
                        .tree()
                        .map_err(|err| AppError::Internal(anyhow::anyhow!("parent tree: {err}")))?;
                    let current_tree = commit
                        .tree()
                        .map_err(|err| AppError::Internal(anyhow::anyhow!("tree: {err}")))?;
                    let diff = repo
                        .diff_tree_to_tree(Some(&previous_tree), Some(&current_tree), None)
                        .map_err(|err| {
                            AppError::Internal(anyhow::anyhow!("diff_tree_to_tree: {err}"))
                        })?;
                    diff.deltas()
                        .filter_map(|delta| {
                            delta.new_file().path().or_else(|| delta.old_file().path())
                        })
                        .any(|path| path.to_string_lossy() == target.as_str())
                }
                Err(err) if err.code() == ErrorCode::NotFound => {
                    let tree = commit
                        .tree()
                        .map_err(|err| AppError::Internal(anyhow::anyhow!("tree: {err}")))?;
                    tree.get_path(Path::new(&target)).is_ok()
                }
                Err(err) => {
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "commit parent 读取失败: {err}"
                    )));
                }
            };

            if touches {
                let author = commit.author();
                let message = commit.message().map_err(|err| {
                    AppError::Internal(anyhow::anyhow!("commit {oid} message 不是 UTF-8: {err}"))
                })?;
                let author_name = author.name().map_err(|err| {
                    AppError::Internal(anyhow::anyhow!(
                        "commit {oid} author name 不是 UTF-8: {err}"
                    ))
                })?;
                let author_email = author.email().map_err(|err| {
                    AppError::Internal(anyhow::anyhow!(
                        "commit {oid} author email 不是 UTF-8: {err}"
                    ))
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
        let repo = self.lock_repo()?;
        let oid = Oid::from_str(sha)
            .map_err(|err| AppError::Internal(anyhow::anyhow!("非法 SHA: {err}")))?;
        let commit = repo
            .find_commit(oid)
            .map_err(|err| AppError::Internal(anyhow::anyhow!("find_commit: {err}")))?;
        let tree = commit
            .tree()
            .map_err(|err| AppError::Internal(anyhow::anyhow!("tree: {err}")))?;
        let target = sanitize_filename(name);
        let entry = tree
            .get_path(Path::new(&target))
            .map_err(|_| AppError::NotFound(format!("commit {sha} 内不存在 {target}")))?;
        let blob = repo
            .find_blob(entry.id())
            .map_err(|err| AppError::Internal(anyhow::anyhow!("find_blob: {err}")))?;
        std::str::from_utf8(blob.content())
            .map(str::to_string)
            .map_err(|error| {
                AppError::Internal(anyhow::anyhow!(
                    "commit {sha} 内提示词 {target} 不是 UTF-8：{error}"
                ))
            })
    }

    /// 当前 HEAD 处提示词内容
    pub fn latest(&self, name: &str) -> Result<Option<String>, AppError> {
        let repo = self.lock_repo()?;
        let target = sanitize_filename(name);
        read_head_prompt_file(&repo, Path::new(&target))
    }
}
