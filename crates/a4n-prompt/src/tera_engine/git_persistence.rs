use std::sync::Arc;

use a4n_core::error::AppError;

use crate::engine::PromptVersion;
use crate::version_store::{CommitInfo, PromptVersionStore};

use super::TeraPromptEngine;
use super::version::bump_version;

impl TeraPromptEngine {
    /// 启用 Git 持久化：每次 update 自动 commit 到给定仓库
    pub fn with_git_store(mut self, store: Arc<dyn PromptVersionStore>) -> Result<Self, AppError> {
        self.hydrate_from_git_store(store.as_ref())?;
        self.git_store = Some(store);
        Ok(self)
    }

    fn hydrate_from_git_store(&self, store: &dyn PromptVersionStore) -> Result<(), AppError> {
        let names: Vec<String> = self.entries.read().keys().cloned().collect();

        for name in names {
            let Some(latest_content) = store.latest(&name)? else {
                continue;
            };

            let history = store.history(&name)?;
            let mut staged_tera = self.tera.read().clone();
            staged_tera
                .add_raw_template(&name, &latest_content)
                .map_err(|e| {
                    AppError::Internal(anyhow::anyhow!("Git 提示词模板编译失败 {}: {e}", name))
                })?;

            let original_content = self
                .original_contents
                .read()
                .get(&name)
                .cloned()
                .ok_or_else(|| AppError::Internal(anyhow::anyhow!("提示词缺少原始内容：{name}")))?;

            let (base_version, base_modified_at) = {
                let entries = self.entries.read();
                let entry = entries
                    .get(&name)
                    .ok_or_else(|| AppError::Internal(anyhow::anyhow!("提示词不存在：{name}")))?;
                (entry.version.clone(), entry.modified_at)
            };

            let current_version = if history.is_empty() {
                if latest_content == original_content {
                    base_version.clone()
                } else {
                    bump_version(&base_version)
                }
            } else {
                bump_version_by(&base_version, history.len())
            };
            let current_modified_at = history
                .first()
                .map(|commit| unix_timestamp_to_naive(commit.timestamp))
                .transpose()?
                .unwrap_or(base_modified_at);

            let version_log = hydrated_version_log(
                store,
                &name,
                &base_version,
                base_modified_at,
                &original_content,
                &history,
            )?;

            {
                let mut entries = self.entries.write();
                let entry = entries
                    .get_mut(&name)
                    .ok_or_else(|| AppError::Internal(anyhow::anyhow!("提示词不存在：{name}")))?;
                entry.content = latest_content.clone();
                entry.version = current_version;
                entry.is_modified = latest_content != original_content;
                entry.modified_at = current_modified_at;
            }
            self.tera
                .write()
                .add_raw_template(&name, &latest_content)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("模板编译失败: {e}")))?;
            if !version_log.is_empty() {
                self.version_log.write().insert(name, version_log);
            }
        }

        Ok(())
    }
}

fn hydrated_version_log(
    store: &dyn PromptVersionStore,
    name: &str,
    base_version: &str,
    base_modified_at: chrono::NaiveDateTime,
    original_content: &str,
    history: &[CommitInfo],
) -> Result<Vec<PromptVersion>, AppError> {
    if history.is_empty() {
        return Ok(Vec::new());
    }

    let mut version_log = vec![PromptVersion {
        version: base_version.to_string(),
        timestamp: base_modified_at,
        author: "registry".into(),
        message: "原始版本".into(),
        content_snapshot: original_content.to_string(),
    }];

    let chronological: Vec<&CommitInfo> = history.iter().rev().collect();
    for (index, commit) in chronological
        .iter()
        .take(chronological.len() - 1)
        .enumerate()
    {
        version_log.push(PromptVersion {
            version: bump_version_by(base_version, index + 1),
            timestamp: unix_timestamp_to_naive(commit.timestamp)?,
            author: commit.author.clone(),
            message: commit.message.clone(),
            content_snapshot: store.get_at(name, &commit.sha)?,
        });
    }

    Ok(version_log)
}

fn bump_version_by(version: &str, count: usize) -> String {
    let mut next = version.to_string();
    for _ in 0..count {
        next = bump_version(&next);
    }
    next
}

fn unix_timestamp_to_naive(timestamp: i64) -> Result<chrono::NaiveDateTime, AppError> {
    chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0)
        .map(|dt| dt.naive_utc())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("Prompt Git commit 时间戳无效：{timestamp}"))
        })
}

#[cfg(test)]
mod tests {
    use super::unix_timestamp_to_naive;

    #[test]
    fn invalid_git_timestamp_is_rejected() {
        assert!(unix_timestamp_to_naive(i64::MAX).is_err());
    }
}
