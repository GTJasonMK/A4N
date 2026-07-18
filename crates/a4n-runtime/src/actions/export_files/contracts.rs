use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

pub type ExportProgressReporter<'a> = &'a (dyn Fn(u32, u32, &str) + Send + Sync);

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportMarkdownInput {
    pub title: String,
    pub author: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchExportInput {
    pub project_id: i64,
    pub formats: Vec<String>,
    pub title: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportFileData {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
}

pub struct ExportProgress<'a> {
    reporter: Option<ExportProgressReporter<'a>>,
    cancel_token: Option<&'a CancellationToken>,
}

#[derive(Debug, Clone, Copy)]
pub struct ExportTaskScope {
    pub project_id: i64,
    pub total: u32,
}

impl<'a> ExportProgress<'a> {
    pub fn new(
        reporter: Option<ExportProgressReporter<'a>>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            reporter,
            cancel_token,
        }
    }

    pub(super) fn progress(&self, current: u32, total: u32, message: &str) {
        if let Some(reporter) = self.reporter {
            reporter(current, total.max(1), message);
        }
    }

    pub(super) fn check_cancelled(&self, message: &str) -> Result<(), String> {
        if self.cancel_token.is_some_and(|token| token.is_cancelled()) {
            Err(message.to_string())
        } else {
            Ok(())
        }
    }
}
