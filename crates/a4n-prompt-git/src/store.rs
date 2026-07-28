use std::ffi::OsString;
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
        validate_repository_head(&repo)?;
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

pub fn backup_prompt_repository(source: &Path, destination: &Path) -> Result<(), AppError> {
    let source = canonical_existing(source, "Prompt Git 源目录")?;
    validate_existing_repository(&source)?;
    let destination = planned_path(destination)?;
    if destination.exists() {
        return Err(AppError::InvalidArgument(format!(
            "Prompt Git 备份目标已存在: {}",
            destination.display()
        )));
    }
    ensure_disjoint(&source, &destination)?;

    let staging = sibling_path(&destination, "backup-tmp")?;
    ensure_absent(&staging)?;
    if let Err(error) = copy_directory(&source, &staging)
        .and_then(|_| validate_existing_repository(&staging))
        .and_then(|_| rename_path(&staging, &destination, "提交 Prompt Git 备份"))
    {
        return Err(cleanup_staging(error, &staging));
    }
    Ok(())
}

pub fn restore_prompt_repository(backup: &Path, target: &Path) -> Result<(), AppError> {
    let backup = canonical_existing(backup, "Prompt Git 备份目录")?;
    validate_existing_repository(&backup)?;
    let target = planned_path(target)?;
    ensure_disjoint(&backup, &target)?;

    let staging = sibling_path(&target, "restore-tmp")?;
    let previous = sibling_path(&target, "restore-previous")?;
    ensure_absent(&staging)?;
    ensure_absent(&previous)?;
    if let Err(error) =
        copy_directory(&backup, &staging).and_then(|_| validate_existing_repository(&staging))
    {
        return Err(cleanup_staging(error, &staging));
    }

    let had_target = target.exists();
    if had_target {
        if let Err(error) = rename_path(&target, &previous, "暂存现有 Prompt Git 仓库") {
            return Err(cleanup_staging(error, &staging));
        }
    }
    if let Err(replace_error) = rename_path(&staging, &target, "恢复 Prompt Git 仓库") {
        let rollback = had_target
            .then(|| rename_path(&previous, &target, "回滚现有 Prompt Git 仓库"))
            .transpose();
        let error = match rollback {
            Ok(_) => replace_error,
            Err(rollback_error) => {
                AppError::Internal(anyhow::anyhow!("{replace_error}；{rollback_error}"))
            }
        };
        return Err(cleanup_staging(error, &staging));
    }
    if had_target {
        std::fs::remove_dir_all(&previous).map_err(|error| {
            AppError::Internal(anyhow::anyhow!(
                "Prompt Git 已恢复，但旧仓库清理失败 {}: {error}",
                previous.display()
            ))
        })?;
    }
    Ok(())
}

fn validate_existing_repository(path: &Path) -> Result<(), AppError> {
    let repo = Repository::open(path).map_err(|error| {
        AppError::Internal(anyhow::anyhow!(
            "Prompt Git 仓库无效 {}: {error}",
            path.display()
        ))
    })?;
    validate_repository_head(&repo)
}

fn validate_repository_head(repo: &Repository) -> Result<(), AppError> {
    match repo.head() {
        Ok(_) => Ok(()),
        Err(err) if matches!(err.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => Ok(()),
        Err(err) => Err(AppError::Internal(anyhow::anyhow!(
            "git 仓库 HEAD 校验失败: {err}"
        ))),
    }
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), AppError> {
    std::fs::create_dir(destination).map_err(|error| path_error("创建目录", destination, error))?;
    for entry in std::fs::read_dir(source).map_err(|error| path_error("读取目录", source, error))?
    {
        let entry = entry.map_err(|error| path_error("读取目录项", source, error))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|error| path_error("读取文件类型", &source_path, error))?;
        if file_type.is_dir() {
            copy_directory(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&source_path, &destination_path)
                .map_err(|error| path_error("复制文件", &source_path, error))?;
        } else {
            return Err(AppError::InvalidArgument(format!(
                "Prompt Git 备份不支持符号链接或特殊文件: {}",
                source_path.display()
            )));
        }
    }
    Ok(())
}

fn canonical_existing(path: &Path, label: &str) -> Result<PathBuf, AppError> {
    std::fs::canonicalize(path).map_err(|error| {
        AppError::InvalidArgument(format!(
            "{label}不存在或不可访问 {}: {error}",
            path.display()
        ))
    })
}

fn planned_path(path: &Path) -> Result<PathBuf, AppError> {
    let file_name = path.file_name().ok_or_else(|| {
        AppError::InvalidArgument(format!("路径必须包含目录名: {}", path.display()))
    })?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(|error| path_error("创建父目录", parent, error))?;
    Ok(std::fs::canonicalize(parent)
        .map_err(|error| path_error("解析父目录", parent, error))?
        .join(file_name))
}

fn ensure_disjoint(left: &Path, right: &Path) -> Result<(), AppError> {
    if left == right || left.starts_with(right) || right.starts_with(left) {
        return Err(AppError::InvalidArgument(format!(
            "Prompt Git 源目录、备份目录和恢复目录不得互相包含: {} <-> {}",
            left.display(),
            right.display()
        )));
    }
    Ok(())
}

fn sibling_path(path: &Path, label: &str) -> Result<PathBuf, AppError> {
    let mut name = OsString::from(".");
    name.push(path.file_name().ok_or_else(|| {
        AppError::InvalidArgument(format!("路径必须包含目录名: {}", path.display()))
    })?);
    name.push(format!(".{label}-{}", std::process::id()));
    Ok(path.parent().unwrap_or_else(|| Path::new(".")).join(name))
}

fn ensure_absent(path: &Path) -> Result<(), AppError> {
    if path.exists() {
        return Err(AppError::InvalidState {
            current: format!("临时路径已存在: {}", path.display()),
            operation: "Prompt Git 备份或恢复".to_string(),
        });
    }
    Ok(())
}

fn rename_path(source: &Path, destination: &Path, operation: &str) -> Result<(), AppError> {
    std::fs::rename(source, destination).map_err(|error| {
        AppError::Internal(anyhow::anyhow!(
            "{operation}失败 {} -> {}: {error}",
            source.display(),
            destination.display()
        ))
    })
}

fn cleanup_staging(primary: AppError, path: &Path) -> AppError {
    if !path.exists() {
        return primary;
    }
    match std::fs::remove_dir_all(path) {
        Ok(_) => primary,
        Err(cleanup_error) => AppError::Internal(anyhow::anyhow!(
            "{primary}；临时目录清理失败 {}: {cleanup_error}",
            path.display()
        )),
    }
}

fn path_error(operation: &str, path: &Path, error: std::io::Error) -> AppError {
    AppError::Internal(anyhow::anyhow!(
        "{operation}失败 {}: {error}",
        path.display()
    ))
}
