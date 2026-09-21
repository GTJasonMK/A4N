use a4n_error::AppError;

use crate::engine::{PromptEntry, PromptVersion};

use super::super::TeraPromptEngine;
use super::super::version::bump_version;
use super::validation::{
    ensure_unique_prompt_update_names, normalize_prompt_update_author,
    validate_prompt_update_content,
};

impl TeraPromptEngine {
    pub async fn update_many(
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
                content_snapshot: entry.content.clone(),
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
}
