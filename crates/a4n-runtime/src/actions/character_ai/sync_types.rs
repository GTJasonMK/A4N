use super::CharacterAiProgress;
pub use a4n_orchestrator::character::CHARACTER_PROFILE_SYNC_STEPS;

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

    pub(super) fn report(
        &self,
        current: u32,
        local_total: u32,
        message: &str,
    ) -> Result<(), String> {
        if let Some(progress) = self.progress {
            let local_total = local_total.max(1);
            let current = current.min(local_total);
            let offset = u64::from(current) * u64::from(self.span) / u64::from(local_total);
            let mapped = self.base
                + u32::try_from(offset)
                    .expect("character progress offset is bounded by the u32 span");
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
