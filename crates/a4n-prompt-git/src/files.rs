use std::path::{Path, PathBuf};

use a4n_error::AppError;
use git2::{ErrorCode, Index, Repository};

pub(super) struct PromptFileChange {
    pub(super) path: PathBuf,
    pub(super) index_path: PathBuf,
    pub(super) previous_content: Option<String>,
    pub(super) previous_committed_content: Option<String>,
    pub(super) next_content: String,
}

pub(super) fn stage_prompt_file_changes(
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

pub(super) fn restore_prompt_index(
    index: &mut Index,
    files: &[PromptFileChange],
) -> Result<(), git2::Error> {
    for file in files {
        if file.previous_content.is_some() {
            index.add_path(&file.index_path)?;
        } else {
            index.remove_path(&file.index_path)?;
        }
    }
    Ok(())
}

pub(super) fn restore_prompt_files(files: &[PromptFileChange]) -> Result<(), AppError> {
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

pub(super) fn restore_prompt_worktree_to_next_content(
    files: &[PromptFileChange],
) -> Result<(), AppError> {
    for file in files {
        if file.previous_content.as_deref() != Some(file.next_content.as_str()) {
            std::fs::write(&file.path, &file.next_content).map_err(AppError::Io)?;
        }
    }
    Ok(())
}

pub(super) fn read_worktree_prompt_file(path: &Path) -> Result<Option<String>, AppError> {
    match std::fs::read_to_string(path) {
        Ok(existing) => Ok(Some(existing)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(AppError::Io(err)),
    }
}

pub(super) fn read_head_prompt_file(
    repo: &Repository,
    index_path: &Path,
) -> Result<Option<String>, AppError> {
    let head = match repo.head() {
        Ok(head) => head,
        Err(err) if matches!(err.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => {
            return Ok(None);
        }
        Err(err) => {
            return Err(AppError::Internal(anyhow::anyhow!(
                "git HEAD 读取失败: {err}"
            )));
        }
    };
    let Some(head_oid) = head.target() else {
        return Ok(None);
    };
    let commit = repo
        .find_commit(head_oid)
        .map_err(|err| AppError::Internal(anyhow::anyhow!("git find_commit 失败: {err}")))?;
    let tree = commit
        .tree()
        .map_err(|err| AppError::Internal(anyhow::anyhow!("git tree 读取失败: {err}")))?;
    let entry = match tree.get_path(index_path) {
        Ok(entry) => entry,
        Err(err) if err.code() == ErrorCode::NotFound => return Ok(None),
        Err(err) => {
            return Err(AppError::Internal(anyhow::anyhow!(
                "git tree 路径读取失败: {err}"
            )));
        }
    };
    let blob = repo
        .find_blob(entry.id())
        .map_err(|err| AppError::Internal(anyhow::anyhow!("git blob 读取失败: {err}")))?;
    let content = std::str::from_utf8(blob.content()).map_err(|error| {
        AppError::Internal(anyhow::anyhow!(
            "git HEAD 提示词不是 UTF-8（{}）：{error}",
            index_path.display()
        ))
    })?;
    Ok(Some(content.to_string()))
}
