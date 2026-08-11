use a4n_core::error::AppError;

use crate::engine::{PromptEntry, PromptVersion};

use super::super::TeraPromptEngine;
use super::super::version::bump_version;
use super::validation::{normalize_prompt_update_author, validate_prompt_update_content};

impl TeraPromptEngine {
    pub async fn update(
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

        // 持久化成功后才提交运行时状态，避免 Err 后内存已变。
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

        // parking_lot 的 WriteGuard 不是 Send，必须在 await 之后重新进入块作用域。
        let result = {
            let mut entries = self.entries.write();
            let entry = entries
                .get_mut(name)
                .ok_or_else(|| Self::prompt_not_found(name))?;
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
            self.version_log
                .write()
                .entry(name.to_string())
                .or_default()
                .push(prev_version);
            entry.version = bump_version(&entry.version);
            entry.content = content.clone();
            entry.is_modified = true;
            entry.modified_at = chrono::Utc::now().naive_utc();
            entry.clone()
        };

        Ok(result)
    }
}
