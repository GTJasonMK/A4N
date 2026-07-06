use a4n_core::models::character::protagonist_profile;
use serde_json::Value;

use super::CharacterAiProgress;
use super::sync_response::ChapterStateDraft;

pub const CHARACTER_PROFILE_SYNC_STEPS: u32 = 10;

#[derive(Clone, Copy)]
pub(super) struct CharacterProgressScope<'a> {
    progress: Option<&'a CharacterAiProgress<'a>>,
    base: u32,
    span: u32,
    total: u32,
    cancelled_message: &'a str,
}

impl<'a> CharacterProgressScope<'a> {
    pub(super) fn root(
        progress: Option<&'a CharacterAiProgress<'a>>,
        total: u32,
        cancelled_message: &'a str,
    ) -> Self {
        let total = total.max(1);
        Self {
            progress,
            base: 0,
            span: total,
            total,
            cancelled_message,
        }
    }

    pub(super) fn child(
        progress: Option<&'a CharacterAiProgress<'a>>,
        base: u32,
        span: u32,
        total: u32,
        cancelled_message: &'a str,
    ) -> Self {
        Self {
            progress,
            base,
            span: span.max(1),
            total: total.max(1),
            cancelled_message,
        }
    }

    pub(super) fn report(
        &self,
        current: u32,
        local_total: u32,
        message: &str,
    ) -> Result<(), String> {
        if let Some(progress) = self.progress {
            let local_total = local_total.max(1);
            let current = current.min(local_total);
            let mapped = self.base + current.saturating_mul(self.span) / local_total;
            progress.report(
                mapped.min(self.total),
                self.total,
                message,
                self.cancelled_message,
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(super) struct CharacterSyncExtraction {
    pub(super) changes: Vec<Value>,
    pub(super) behaviors: Vec<Value>,
    pub(super) deletion_candidates: Vec<Value>,
    pub(super) chapter_states: Vec<ChapterStateDraft>,
}

#[derive(Debug, Clone)]
pub(super) struct CharacterSyncContext {
    pub(super) profile: protagonist_profile::Model,
    pub(super) start_chapter: i32,
    pub(super) target_chapter: i32,
    pub(super) clear_to_chapter: i32,
    pub(super) selected_chapter_numbers: Vec<i32>,
    pub(super) chapter_data: Vec<Value>,
    pub(super) baseline_state: Value,
}
