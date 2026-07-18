use a4n_core::error::AppError;
use a4n_prompt::version_store::{CommitInfo, PromptVersionStore};

use crate::store::GitPromptStore;

impl PromptVersionStore for GitPromptStore {
    fn commit_prompt(&self, name: &str, content: &str, message: &str) -> Result<String, AppError> {
        GitPromptStore::commit_prompt(self, name, content, message)
    }

    fn commit_prompts(
        &self,
        prompts: &[(String, String)],
        message: &str,
    ) -> Result<String, AppError> {
        GitPromptStore::commit_prompts(self, prompts, message)
    }

    fn history(&self, name: &str) -> Result<Vec<CommitInfo>, AppError> {
        GitPromptStore::history(self, name)
    }

    fn get_at(&self, name: &str, sha: &str) -> Result<String, AppError> {
        GitPromptStore::get_at(self, name, sha)
    }

    fn latest(&self, name: &str) -> Result<Option<String>, AppError> {
        GitPromptStore::latest(self, name)
    }
}
