use a4n_core::error::AppError;
use git2::{ErrorCode, Signature};

use crate::files::{
    PromptFileChange, read_head_prompt_file, read_worktree_prompt_file, restore_prompt_files,
    restore_prompt_index, restore_prompt_worktree_to_next_content, stage_prompt_file_changes,
};
use crate::store::GitPromptStore;

impl GitPromptStore {
    /// 提交提示词（覆盖写文件 → stage → commit）
    ///
    /// 返回新 commit 的 SHA。如果新内容与最新版完全一致则跳过提交并返回当前 HEAD（或空仓库时返回空 String）。
    pub(super) fn commit_prompt(
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
    pub(super) fn commit_prompts(
        &self,
        prompts: &[(String, String)],
        message: &str,
    ) -> Result<String, AppError> {
        let repo = self.lock_repo()?;
        let mut files = Vec::with_capacity(prompts.len());

        for (name, content) in prompts {
            let path = self.file_path(name);
            let index_path = path
                .strip_prefix(&self.repo_path)
                .map_err(|err| {
                    AppError::Internal(anyhow::anyhow!("git prompt 路径不在仓库内: {err}"))
                })?
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
                .map_err(|err| AppError::Internal(anyhow::anyhow!("git index 打开失败: {err}")))?;
            for file in &files {
                index.add_path(&file.index_path).map_err(|err| {
                    AppError::Internal(anyhow::anyhow!("git stage 清理失败: {err}"))
                })?;
            }
            index
                .write()
                .map_err(|err| AppError::Internal(anyhow::anyhow!("git index 清理失败: {err}")))?;
            match repo.head() {
                Ok(head) => {
                    if let Some(oid) = head.target() {
                        return Ok(oid.to_string());
                    }
                }
                Err(err) if matches!(err.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => {
                }
                Err(err) => {
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "git HEAD 读取失败: {err}"
                    )));
                }
            }
            return Ok(String::new());
        }

        let signature = Signature::now(&self.author_name, &self.author_email)
            .map_err(|err| AppError::Internal(anyhow::anyhow!("git signature 失败: {err}")))?;

        for file in &files {
            if let Err(err) = std::fs::write(&file.path, &file.next_content).map_err(AppError::Io) {
                restore_prompt_files(&files)?;
                return Err(err);
            }
        }

        let commit_result = (|| -> Result<String, AppError> {
            let mut index = repo
                .index()
                .map_err(|err| AppError::Internal(anyhow::anyhow!("git index 打开失败: {err}")))?;
            stage_prompt_file_changes(&mut index, &files)
                .map_err(|err| AppError::Internal(anyhow::anyhow!("git stage 失败: {err}")))?;
            index
                .write()
                .map_err(|err| AppError::Internal(anyhow::anyhow!("git index 写入失败: {err}")))?;
            let tree_oid = index
                .write_tree()
                .map_err(|err| AppError::Internal(anyhow::anyhow!("git write_tree 失败: {err}")))?;
            let tree = repo
                .find_tree(tree_oid)
                .map_err(|err| AppError::Internal(anyhow::anyhow!("git find_tree 失败: {err}")))?;

            let parents: Vec<git2::Commit> = match repo.head() {
                Ok(head) => match head.target() {
                    Some(oid) => vec![repo.find_commit(oid).map_err(|err| {
                        AppError::Internal(anyhow::anyhow!("git find_commit 失败: {err}"))
                    })?],
                    None => vec![],
                },
                Err(err) if matches!(err.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => {
                    vec![]
                }
                Err(err) => {
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "git HEAD 读取失败: {err}"
                    )));
                }
            };
            let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

            let commit_oid = repo
                .commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    message,
                    &tree,
                    &parent_refs,
                )
                .map_err(|err| AppError::Internal(anyhow::anyhow!("git commit 失败: {err}")))?;

            Ok(commit_oid.to_string())
        })();

        if let Err(err) = commit_result {
            restore_prompt_files(&files)?;
            let mut index = repo.index().map_err(|source| {
                AppError::Internal(anyhow::anyhow!("git index 打开失败: {source}"))
            })?;
            restore_prompt_index(&mut index, &files).map_err(|source| {
                AppError::Internal(anyhow::anyhow!("git stage 恢复失败: {source}"))
            })?;
            index.write().map_err(|source| {
                AppError::Internal(anyhow::anyhow!("git index 恢复失败: {source}"))
            })?;
            return Err(err);
        }

        commit_result
    }
}
