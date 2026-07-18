use super::VideoGenerationProgressCallback;
use tokio_util::sync::CancellationToken;

pub struct VideoGenerationProgress<'a> {
    reporter: Option<VideoGenerationProgressCallback<'a>>,
    cancel_token: CancellationToken,
}

impl<'a> VideoGenerationProgress<'a> {
    pub fn new(
        reporter: Option<VideoGenerationProgressCallback<'a>>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            reporter,
            cancel_token,
        }
    }

    pub(in crate::actions::video_generation) fn progress(
        &mut self,
        current: u32,
        total: u32,
        message: &str,
    ) -> Result<(), String> {
        if let Some(reporter) = self.reporter.as_deref_mut() {
            reporter(current, total.max(1), message).map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    pub(in crate::actions::video_generation) fn check_cancelled(
        &self,
        message: &str,
    ) -> Result<(), String> {
        if self.cancel_token.is_cancelled() {
            return Err(message.to_string());
        }
        Ok(())
    }

    pub(in crate::actions::video_generation) fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }
}
