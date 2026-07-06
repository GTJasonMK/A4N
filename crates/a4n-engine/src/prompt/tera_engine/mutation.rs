use a4n_core::error::AppError;

use crate::prompt::engine::{PromptEntry, PromptVersion};

use super::TeraPromptEngine;
use super::version::bump_version;
use std::collections::HashSet;

impl TeraPromptEngine {
    pub(super) async fn update_prompt(
        &self,
        name: &str,
        content: String,
        author: &str,
    ) -> Result<PromptEntry, AppError> {
        validate_prompt_update_content(&content)?;
        let _update_guard = self.update_lock.lock().await;

        let current = {
            let entries = self.entries.read();
            entries
                .get(name)
                .cloned()
                .ok_or_else(|| Self::prompt_not_found(name))?
        };
        if current.content == content {
            return Ok(current);
        }

        let author = normalize_prompt_update_author(author)?;
        let planned_version = bump_version(&current.version);

        {
            let mut staged_tera = self.tera.read().clone();
            staged_tera
                .add_raw_template(name, &content)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("模板编译失败: {e}")))?;
        }

        // 同步 Git 提交（如果启用）。持久化成功后才提交运行时状态，避免 Err 后内存已变。
        if let Some(store) = &self.git_store {
            let message = format!("{author}: 更新 {name} → v{planned_version}");
            let store_clone = store.clone();
            let name_clone = name.to_string();
            let commit_content = content.clone();
            tokio::task::spawn_blocking(move || {
                store_clone.commit_prompt(&name_clone, &commit_content, &message)
            })
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("git commit task 失败: {e}")))??;
        }

        // 关键：parking_lot 的 WriteGuard 不是 Send，必须在 await 之后重新进入块作用域。
        let result = {
            let mut entries = self.entries.write();
            let entry = entries
                .get_mut(name)
                .ok_or_else(|| Self::prompt_not_found(name))?;

            // 将当前版本保存到历史记录
            let prev_version = PromptVersion {
                version: entry.version.clone(),
                timestamp: entry.modified_at,
                author: author.clone(),
                message: format!("更新前版本: {}", entry.version),
                content_snapshot: Some(entry.content.clone()),
            };
            self.tera
                .write()
                .add_raw_template(name, &content)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("模板编译失败: {e}")))?;

            // 生成新版本号（递增补丁版本）
            let new_version = bump_version(&entry.version);
            self.version_log
                .write()
                .entry(name.to_string())
                .or_default()
                .push(prev_version);
            entry.version = new_version;
            entry.content = content.clone();
            entry.is_modified = true;
            entry.modified_at = chrono::Utc::now().naive_utc();
            entry.clone()
        };

        Ok(result)
    }

    pub(super) async fn update_prompts(
        &self,
        updates: Vec<(String, String)>,
        author: &str,
    ) -> Result<Vec<PromptEntry>, AppError> {
        if updates.is_empty() {
            return Ok(Vec::new());
        }

        for (_, content) in &updates {
            validate_prompt_update_content(content)?;
        }
        let author = normalize_prompt_update_author(author)?;
        ensure_unique_prompt_update_names(&updates)?;

        let _update_guard = self.update_lock.lock().await;

        let current_entries = {
            let entries = self.entries.read();
            updates
                .iter()
                .map(|(name, _)| {
                    entries
                        .get(name)
                        .cloned()
                        .ok_or_else(|| Self::prompt_not_found(name))
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        let mut staged_tera = self.tera.read().clone();
        for (name, content) in &updates {
            staged_tera
                .add_raw_template(name, content)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("模板编译失败: {e}")))?;
        }

        let changed_updates = updates
            .iter()
            .zip(current_entries.iter())
            .filter(|((_, content), current)| current.content != **content)
            .map(|((name, content), _)| (name.clone(), content.clone()))
            .collect::<Vec<_>>();

        if !changed_updates.is_empty()
            && let Some(store) = &self.git_store
        {
            let message = format!("{author}: 批量更新提示词 {} 个", changed_updates.len());
            let store_clone = store.clone();
            tokio::task::spawn_blocking(move || {
                store_clone.commit_prompts(&changed_updates, &message)
            })
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("git commit task 失败: {e}")))??;
        }

        let now = chrono::Utc::now().naive_utc();
        let mut next_entries = self.entries.read().clone();
        let mut next_version_log = self.version_log.read().clone();

        for (name, content) in &updates {
            let entry = next_entries
                .get_mut(name)
                .ok_or_else(|| Self::prompt_not_found(name))?;
            if entry.content == *content {
                continue;
            }

            let prev_version = PromptVersion {
                version: entry.version.clone(),
                timestamp: entry.modified_at,
                author: author.clone(),
                message: format!("更新前版本: {}", entry.version),
                content_snapshot: Some(entry.content.clone()),
            };
            next_version_log
                .entry(name.clone())
                .or_default()
                .push(prev_version);
            entry.version = bump_version(&entry.version);
            entry.content = content.clone();
            entry.is_modified = true;
            entry.modified_at = now;
        }

        let result = updates
            .iter()
            .map(|(name, _)| {
                next_entries
                    .get(name)
                    .cloned()
                    .ok_or_else(|| Self::prompt_not_found(name))
            })
            .collect::<Result<Vec<_>, _>>()?;

        *self.entries.write() = next_entries;
        *self.version_log.write() = next_version_log;
        *self.tera.write() = staged_tera;

        Ok(result)
    }

    pub(super) async fn reset_prompt(&self, name: &str) -> Result<(), AppError> {
        let _update_guard = self.update_lock.lock().await;

        let current = {
            let entries = self.entries.read();
            entries
                .get(name)
                .cloned()
                .ok_or_else(|| Self::prompt_not_found(name))?
        };

        let orig_content = {
            let originals = self.original_contents.read();
            originals.get(name).cloned()
        }
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("提示词缺少原始内容：{name}")))?;

        {
            let mut staged_tera = self.tera.read().clone();
            staged_tera
                .add_raw_template(name, &orig_content)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("模板编译失败: {e}")))?;
        }

        if current.content == orig_content && !current.is_modified {
            return Ok(());
        }

        let new_version = bump_version(&current.version);
        if let Some(store) = &self.git_store {
            let message = format!("system: 重置 {name} → v{new_version}");
            let store_clone = store.clone();
            let name_clone = name.to_string();
            let commit_content = orig_content.clone();
            tokio::task::spawn_blocking(move || {
                store_clone.commit_prompt(&name_clone, &commit_content, &message)
            })
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("git commit task 失败: {e}")))??;
        }

        {
            let mut entries = self.entries.write();
            let entry = entries
                .get_mut(name)
                .ok_or_else(|| Self::prompt_not_found(name))?;
            let prev_version = PromptVersion {
                version: entry.version.clone(),
                timestamp: entry.modified_at,
                author: "system".into(),
                message: format!("重置前版本: {}", entry.version),
                content_snapshot: Some(entry.content.clone()),
            };
            self.tera
                .write()
                .add_raw_template(name, &orig_content)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("模板编译失败: {e}")))?;
            self.version_log
                .write()
                .entry(name.to_string())
                .or_default()
                .push(prev_version);
            entry.version = new_version;
            entry.content = orig_content;
            entry.is_modified = false;
            entry.modified_at = chrono::Utc::now().naive_utc();
        }

        Ok(())
    }
}

fn validate_prompt_update_content(content: &str) -> Result<(), AppError> {
    if content.trim().is_empty() {
        return Err(AppError::InvalidArgument("提示词内容不能为空".to_string()));
    }
    Ok(())
}

fn normalize_prompt_update_author(author: &str) -> Result<String, AppError> {
    let author = author.trim();
    if author.is_empty() {
        return Err(AppError::InvalidArgument(
            "提示词更新作者不能为空".to_string(),
        ));
    }
    Ok(author.to_string())
}

fn ensure_unique_prompt_update_names(updates: &[(String, String)]) -> Result<(), AppError> {
    let mut names = HashSet::new();
    for (name, _) in updates {
        if !names.insert(name.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "提示词批量更新包含重复 name：{name}"
            )));
        }
    }
    Ok(())
}
