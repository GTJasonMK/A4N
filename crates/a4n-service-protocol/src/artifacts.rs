use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub artifact_id: String,
    pub artifact_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_user_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i64>,
    pub content_type: String,
    pub sha256: String,
    pub byte_size: u64,
    pub location: ArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub ownership: ArtifactOwnership,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactLocation {
    LocalPath(String),
    UploadUrl(String),
    DownloadUrl(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactOwnership {
    Host,
    Sidecar,
    Remote,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArtifactValidationError {
    #[error("remote sidecar artifact 不能返回本地路径: artifact_id={artifact_id}")]
    RemoteLocalPath { artifact_id: String },
}

pub fn validate_artifact_refs(
    artifact_refs: &[ArtifactRef],
) -> Result<(), ArtifactValidationError> {
    for artifact in artifact_refs {
        validate_artifact_ref(artifact)?;
    }
    Ok(())
}

fn validate_artifact_ref(artifact: &ArtifactRef) -> Result<(), ArtifactValidationError> {
    if matches!(artifact.ownership, ArtifactOwnership::Remote)
        && matches!(artifact.location, ArtifactLocation::LocalPath(_))
    {
        return Err(ArtifactValidationError::RemoteLocalPath {
            artifact_id: artifact.artifact_id.clone(),
        });
    }
    Ok(())
}
