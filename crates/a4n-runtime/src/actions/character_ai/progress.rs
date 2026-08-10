use tokio_util::sync::CancellationToken;

pub type CharacterAiProgressReporter<'a> = &'a (dyn Fn(u32, u32, &str) + Send + Sync);

pub struct CharacterAiProgress<'a> {
    reporter: Option<CharacterAiProgressReporter<'a>>,
    stream_reporter: Option<&'a (dyn Fn(&str) + Send + Sync)>,
    pub(super) cancel_token: Option<&'a CancellationToken>,
    pub(super) task_id: Option<&'a str>,
}

impl<'a> CharacterAiProgress<'a> {
    pub fn new(
        reporter: Option<CharacterAiProgressReporter<'a>>,
        cancel_token: Option<&'a CancellationToken>,
        task_id: Option<&'a str>,
    ) -> Self {
        Self {
            reporter,
            stream_reporter: None,
            cancel_token,
            task_id,
        }
    }

    /// 附加活体信号出口。不提供时行为与之前完全一致。
    pub fn with_stream_reporter(
        mut self,
        stream_reporter: Option<&'a (dyn Fn(&str) + Send + Sync)>,
    ) -> Self {
        self.stream_reporter = stream_reporter;
        self
    }

    pub(crate) fn stream_text(&self, text: &str) {
        if let Some(reporter) = self.stream_reporter {
            reporter(text);
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
            reporter(current, total, message);
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
