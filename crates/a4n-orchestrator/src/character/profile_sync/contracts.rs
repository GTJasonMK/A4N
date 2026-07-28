use a4n_core::{error::AppError, models::character::protagonist_profile};

use super::super::CharacterSyncRangeMode;

pub type CharacterProfileSyncProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Clone, Copy)]
pub struct CharacterProfileSyncProgress<'a> {
    pub reporter: Option<CharacterProfileSyncProgressReporter<'a>>,
}

impl CharacterProfileSyncProgress<'_> {
    pub(super) fn report(&self, current: u32, message: &str) -> Result<(), AppError> {
        if let Some(reporter) = self.reporter {
            reporter(current, CHARACTER_PROFILE_SYNC_PROGRESS_TOTAL, message)?;
        }
        Ok(())
    }
}

pub(super) const CHARACTER_PROFILE_SYNC_PROGRESS_TOTAL: u32 = 10;

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
