use a4n_prompt::engine::{PromptEntry, PromptVersion};

use super::{PromptEntrySummary, PromptVersionSummary};

pub(super) fn to_entry_summary(entry: &PromptEntry) -> PromptEntrySummary {
    PromptEntrySummary {
        name: entry.name.clone(),
        title: entry.title.clone(),
        content: entry.content.clone(),
        description: entry.description.clone(),
        tags: entry.tags.clone(),
        category: entry.category.clone(),
        dependencies: entry.dependencies.clone(),
        is_modified: entry.is_modified,
        version: entry.version.clone(),
        modified_at: entry.modified_at.to_string(),
        used_by: entry.used_by.clone(),
        path: entry.path.clone(),
    }
}

pub(super) fn to_version_summary(version: &PromptVersion) -> PromptVersionSummary {
    PromptVersionSummary {
        version: version.version.clone(),
        timestamp: version.timestamp.to_string(),
        author: version.author.clone(),
        message: version.message.clone(),
        content_snapshot: version.content_snapshot.clone(),
    }
}
