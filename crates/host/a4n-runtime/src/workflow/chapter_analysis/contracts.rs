use a4n_error::AppError;
use tokio_util::sync::CancellationToken;

pub type ChapterAnalysisProgressReporter<'a> = &'a (dyn Fn(u32, u32, &str) + Send + Sync);

pub struct ChapterAnalysisProgress<'a> {
    pub(super) reporter: Option<ChapterAnalysisProgressReporter<'a>>,
    pub(super) stream_reporter: Option<&'a (dyn Fn(&str) + Send + Sync)>,
    pub(super) cancel_token: Option<&'a CancellationToken>,
    pub(super) task_id: Option<&'a str>,
}

impl<'a> ChapterAnalysisProgress<'a> {
    pub fn new(
        reporter: Option<ChapterAnalysisProgressReporter<'a>>,
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

    /// 附加流式输出回调。
    pub fn with_stream_reporter(
        mut self,
        stream_reporter: Option<&'a (dyn Fn(&str) + Send + Sync)>,
    ) -> Self {
        self.stream_reporter = stream_reporter;
        self
    }

    pub(super) fn report(&self, current: u32, total: u32, message: &str) -> Result<(), AppError> {
        if let Some(reporter) = self.reporter {
            reporter(current, total.max(1), message);
        }
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            Err(AppError::Cancelled)
        } else {
            Ok(())
        }
    }
}
