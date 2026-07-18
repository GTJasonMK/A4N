use tokio_util::sync::CancellationToken;

pub type CharacterAiProgressReporter<'a> = &'a (dyn Fn(u32, u32, &str) + Send + Sync);

pub struct CharacterAiProgress<'a> {
    reporter: Option<CharacterAiProgressReporter<'a>>,
    pub(super) cancel_token: Option<&'a CancellationToken>,
}

impl<'a> CharacterAiProgress<'a> {
    pub fn new(
        reporter: Option<CharacterAiProgressReporter<'a>>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            reporter,
            cancel_token,
        }
    }

    pub(crate) fn report(
        &self,
        current: u32,
        total: u32,
        message: &str,
        cancelled_message: &str,
    ) -> Result<(), String> {
        if let Some(reporter) = self.reporter {
            reporter(current, total.max(1), message);
        }
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            Err(cancelled_message.to_string())
        } else {
            Ok(())
        }
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancel_token
            .is_some_and(CancellationToken::is_cancelled)
    }
}
