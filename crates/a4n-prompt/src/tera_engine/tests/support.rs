use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use a4n_core::error::AppError;
use chrono::Utc;

use crate::engine::PromptEntry;
use crate::version_store::{CommitInfo, PromptVersionStore};

#[derive(Debug, Clone)]
struct StoredPromptCommit {
    info: CommitInfo,
    content: String,
}

#[derive(Default)]
pub(super) struct MemoryPromptVersionStore {
    commits: Mutex<HashMap<String, Vec<StoredPromptCommit>>>,
    fail_commits: std::sync::atomic::AtomicBool,
}

impl MemoryPromptVersionStore {
    pub(super) fn fail_commits(&self) {
        self.fail_commits
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn commit_failure(&self) -> Result<String, AppError> {
        Err(AppError::Io(std::io::Error::other(
            "test version store commit failure",
        )))
    }
}

impl PromptVersionStore for MemoryPromptVersionStore {
    fn commit_prompt(&self, name: &str, content: &str, message: &str) -> Result<String, AppError> {
        self.commit_prompts(&[(name.to_string(), content.to_string())], message)
    }

    fn commit_prompts(
        &self,
        prompts: &[(String, String)],
        message: &str,
    ) -> Result<String, AppError> {
        if self.fail_commits.load(std::sync::atomic::Ordering::SeqCst) {
            return self.commit_failure();
        }

        let mut commits = self.commits.lock().expect("memory prompt store lock");
        let sha = format!(
            "memory-{}",
            commits.values().map(Vec::len).sum::<usize>() + 1
        );
        for (name, content) in prompts {
            let list = commits.entry(name.clone()).or_default();
            list.push(StoredPromptCommit {
                info: CommitInfo {
                    sha: sha.clone(),
                    message: message.to_string(),
                    author: "A4N <a4n@local>".to_string(),
                    timestamp: i64::try_from(list.len() + 1).unwrap(),
                },
                content: content.clone(),
            });
        }
        Ok(sha)
    }

    fn history(&self, name: &str) -> Result<Vec<CommitInfo>, AppError> {
        Ok(self
            .commits
            .lock()
            .expect("memory prompt store lock")
            .get(name)
            .into_iter()
            .flat_map(|items| items.iter().rev().map(|item| item.info.clone()))
            .collect())
    }

    fn get_at(&self, name: &str, sha: &str) -> Result<String, AppError> {
        self.commits
            .lock()
            .expect("memory prompt store lock")
            .get(name)
            .and_then(|items| items.iter().find(|item| item.info.sha == sha))
            .map(|item| item.content.clone())
            .ok_or_else(|| AppError::NotFound(format!("commit {sha} 内不存在 {name}")))
    }

    fn latest(&self, name: &str) -> Result<Option<String>, AppError> {
        Ok(self
            .commits
            .lock()
            .expect("memory prompt store lock")
            .get(name)
            .and_then(|items| items.last())
            .map(|item| item.content.clone()))
    }
}

pub(super) fn write_test_registry(dir: &Path, entries: &str) {
    fs::write(
        dir.join("_registry.yaml"),
        format!(
            "prompts:\n{entries}categories:\n  writing:\n    label: \"写作\"\n    order: 1\n    description: \"写作模板\"\n  outline:\n    label: \"大纲\"\n    order: 2\n    description: \"大纲模板\""
        ),
    )
    .unwrap();
}

pub(super) fn make_entry(name: &str, content: &str) -> PromptEntry {
    PromptEntry {
        name: name.to_string(),
        title: name.to_string(),
        content: content.to_string(),
        description: "test prompt".to_string(),
        tags: vec![],
        category: "writing".to_string(),
        dependencies: vec![],
        is_modified: false,
        version: "1.0".to_string(),
        modified_at: Utc::now().naive_utc(),
        used_by: Vec::new(),
        path: format!("{name}.md"),
    }
}
