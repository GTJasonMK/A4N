use a4n_db_models::models::character::protagonist_profile;

use super::super::CharacterSyncRangeMode;

pub use a4n_character_host_contract::CharacterProfileSyncProgress;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterProfileSyncAnchorMode {
    EnsureCurrent,
    RequireExistingCurrent,
}

#[derive(Debug, Clone)]
pub enum CharacterProfileSyncSubject {
    ByName { project_id: i64, name: String },
    Profile(protagonist_profile::Model),
}

#[derive(Debug, Clone)]
pub struct CharacterProfileSyncApplicationInput {
    pub subject: CharacterProfileSyncSubject,
    pub target_chapter: i32,
    pub range_mode: CharacterSyncRangeMode,
    pub anchor_mode: CharacterProfileSyncAnchorMode,
}

impl CharacterProfileSyncApplicationInput {
    pub fn incremental_by_name(
        project_id: i64,
        name: impl Into<String>,
        target_chapter: i32,
    ) -> Self {
        Self {
            subject: CharacterProfileSyncSubject::ByName {
                project_id,
                name: name.into(),
            },
            target_chapter,
            range_mode: CharacterSyncRangeMode::Incremental,
            anchor_mode: CharacterProfileSyncAnchorMode::EnsureCurrent,
        }
    }

    pub fn preview_by_name(project_id: i64, name: impl Into<String>, target_chapter: i32) -> Self {
        Self {
            subject: CharacterProfileSyncSubject::ByName {
                project_id,
                name: name.into(),
            },
            target_chapter,
            range_mode: CharacterSyncRangeMode::Incremental,
            anchor_mode: CharacterProfileSyncAnchorMode::RequireExistingCurrent,
        }
    }
}
