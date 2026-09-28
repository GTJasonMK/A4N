use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Host-provided progress and cancellation state for long-running exports.
pub struct ExportProgress<'a> {
    reporter: Option<&'a (dyn Fn(u32, u32, &str) + Send + Sync)>,
    cancel_checker: Option<&'a (dyn Fn() -> bool + Send + Sync)>,
}

impl<'a> ExportProgress<'a> {
    pub fn new(
        reporter: Option<&'a (dyn Fn(u32, u32, &str) + Send + Sync)>,
        cancel_checker: Option<&'a (dyn Fn() -> bool + Send + Sync)>,
    ) -> Self {
        Self {
            reporter,
            cancel_checker,
        }
    }

    pub fn progress(&self, current: u32, total: u32, message: &str) {
        if let Some(reporter) = self.reporter {
            reporter(current, total, message);
        }
    }

    pub fn check_cancelled(&self) -> Result<(), a4n_error::AppError> {
        if self.cancel_checker.is_some_and(|check| check()) {
            Err(a4n_error::AppError::Cancelled)
        } else {
            Ok(())
        }
    }

    pub fn without_reporting(&self) -> Self {
        Self {
            reporter: None,
            cancel_checker: self.cancel_checker,
        }
    }
}

impl a4n_export_contract::ExportRenderObserver for ExportProgress<'_> {
    fn progress(&self, current: u32, total: u32, message: &str) {
        self.progress(current, total, message);
    }

    fn is_cancelled(&self) -> bool {
        self.cancel_checker.is_some_and(|check| check())
    }
}

/// Host-facing project export boundary.
#[async_trait::async_trait]
pub trait ProjectExportApplicationPort: Send + Sync {
    async fn export_txt(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ExportFileData, a4n_error::AppError>;

    async fn export_markdown(
        &self,
        user_id: i64,
        project_id: i64,
        title: &str,
        author: &str,
    ) -> Result<ExportFileData, a4n_error::AppError>;

    async fn export_outline_json(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ExportFileData, a4n_error::AppError>;

    async fn export_characters_json(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ExportFileData, a4n_error::AppError>;

    async fn export_epub(
        &self,
        user_id: i64,
        project_id: i64,
        title: &str,
        author: &str,
        progress: &ExportProgress<'_>,
    ) -> Result<ExportFileData, a4n_error::AppError>;

    async fn export_pdf(
        &self,
        user_id: i64,
        project_id: i64,
        progress: &ExportProgress<'_>,
    ) -> Result<ExportFileData, a4n_error::AppError>;

    async fn export_batch(
        &self,
        user_id: i64,
        input: BatchExportInput,
        progress: &ExportProgress<'_>,
    ) -> Result<Vec<ExportFileData>, a4n_error::AppError>;
}

/// Host-facing Galgame WebGAL export boundary.
#[async_trait::async_trait]
pub trait GalgameExportApplicationPort: Send + Sync {
    async fn export_galgame_webgal(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ExportFileData, a4n_error::AppError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportProjectInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportPublicationMetadata {
    #[schemars(length(min = 1))]
    pub title: String,
    #[schemars(length(min = 1))]
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportPublicationInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub metadata: ExportPublicationMetadata,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BatchExportFormat {
    Txt,
    Markdown,
    Epub,
    #[serde(rename = "json")]
    OutlineJson,
    Pdf,
}

impl BatchExportFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Markdown => "markdown",
            Self::Epub => "epub",
            Self::OutlineJson => "json",
            Self::Pdf => "pdf",
        }
    }

    pub const fn requires_metadata(self) -> bool {
        matches!(self, Self::Markdown | Self::Epub)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BatchExportInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 5))]
    pub formats: Vec<BatchExportFormat>,
    pub metadata: Option<ExportPublicationMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ExportFileData {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_inputs_accept_only_canonical_formats_and_complete_metadata() {
        for legacy in ["md", "MD", " EPUB "] {
            assert!(serde_json::from_value::<BatchExportFormat>(legacy.into()).is_err());
        }

        for (format, canonical) in [
            (BatchExportFormat::Txt, "txt"),
            (BatchExportFormat::Markdown, "markdown"),
            (BatchExportFormat::Epub, "epub"),
            (BatchExportFormat::OutlineJson, "json"),
            (BatchExportFormat::Pdf, "pdf"),
        ] {
            assert_eq!(format.as_str(), canonical);
        }
        assert!(BatchExportFormat::Markdown.requires_metadata());
        assert!(BatchExportFormat::Epub.requires_metadata());
        assert!(!BatchExportFormat::Pdf.requires_metadata());

        let partial_metadata = serde_json::json!({
            "project_id": 42,
            "metadata": { "title": "测试书名" },
        });
        assert!(serde_json::from_value::<ExportPublicationInput>(partial_metadata).is_err());
    }
}
