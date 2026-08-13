use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
