use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use a4n_core::error::AppError;
use git2::{ErrorCode, Repository};

use crate::naming::sanitize_filename;

/// 基于 git2 的提示词版本存储
pub struct GitPromptStore {
    pub(super) repo_path: PathBuf,
    pub(super) repo: Mutex<Repository>,
    pub(super) author_name: String,
    pub(super) author_email: String,
}

impl GitPromptStore {
    /// 打开已有仓库或初始化新仓库
    pub fn open_or_init(repo_path: PathBuf) -> Result<Self, AppError> {
        std::fs::create_dir_all(&repo_path).map_err(AppError::Io)?;
        let repo = match Repository::open(&repo_path) {
            Ok(repo) => repo,
            Err(err) if err.code() == ErrorCode::NotFound => Repository::init(&repo_path)
                .map_err(|err| AppError::Internal(anyhow::anyhow!("git init 失败: {err}")))?,
            Err(err) => {
                return Err(AppError::Internal(anyhow::anyhow!(
                    "git 仓库打开失败: {err}"
                )));
            }
        };
        match repo.head() {
            Ok(_) => {}
            Err(err) if matches!(err.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => {}
            Err(err) => {
                return Err(AppError::Internal(anyhow::anyhow!(
                    "git 仓库 HEAD 校验失败: {err}"
                )));
            }
        }
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

    pub(super) fn file_path(&self, name: &str) -> PathBuf {
        self.repo_path.join(sanitize_filename(name))
    }

    pub(super) fn lock_repo(&self) -> Result<MutexGuard<'_, Repository>, AppError> {
        self.repo
            .lock()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("git 仓库锁已中毒")))
    }
}
