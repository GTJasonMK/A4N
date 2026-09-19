use serde::{Deserialize, Serialize};

pub const ARTIFACT_MEDIA_WORKER_ID: &str = "artifact-media-worker";
pub const ARTIFACT_MEDIA_WORKER_URL_ENV: &str = "A4N_ARTIFACT_MEDIA_WORKER_URL";
pub const ARTIFACT_MEDIA_WORKER_TOKEN_ENV: &str = "A4N_ARTIFACT_MEDIA_WORKER_TOKEN";
pub const ARTIFACT_MEDIA_MAINTENANCE_COMMAND: &str = "maintain";
pub const ARTIFACT_MEDIA_UNREFERENCED_PATHS_COMMAND: &str = "unreferenced-paths";
pub const ARTIFACT_MEDIA_PATH_PREFIX_REFERENCED_COMMAND: &str = "path-prefix-referenced";
pub const ARTIFACT_MEDIA_PROJECT_PUBLICATION_COMMAND: &str = "project-has-publication";
pub const ARTIFACT_MEDIA_PROTECTED_PATHS_COMMAND: &str = "protected-publication-paths";
pub const ARTIFACT_MEDIA_PUBLISH_IMAGE_COMMAND: &str = "publish-image";
pub const ARTIFACT_MEDIA_IMAGE_BY_ID_COMMAND: &str = "image-by-id";
pub const ARTIFACT_MEDIA_IMAGE_LIST_PROJECT_COMMAND: &str = "image-list-project";
pub const ARTIFACT_MEDIA_IMAGE_LIST_PROJECTS_COMMAND: &str = "image-list-projects";
pub const ARTIFACT_MEDIA_IMAGE_LIST_CHAPTER_COMMAND: &str = "image-list-chapter";
pub const ARTIFACT_MEDIA_IMAGE_UPDATE_COMMAND: &str = "image-update";
pub const ARTIFACT_MEDIA_IMAGE_DELETE_COMMAND: &str = "image-delete";
pub const ARTIFACT_MEDIA_CAPABILITY_VERSION: &str = "artifact-media-v1";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArtifactMediaContractError {
    #[error("{field} must not be blank")]
    BlankField { field: &'static str },
    #[error("{field} must contain between 1 and {max} items")]
    InvalidCount { field: &'static str, max: usize },
    #[error("{field} must be positive")]
    InvalidValue { field: &'static str },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactMediaMaintenanceRequest {
    pub publication_limit: u16,
    pub cleanup_limit: u16,
}

impl ArtifactMediaMaintenanceRequest {
    pub fn validate(&self) -> Result<(), ArtifactMediaContractError> {
        if self.publication_limit == 0 || self.cleanup_limit == 0 {
            return Err(ArtifactMediaContractError::InvalidValue {
                field: "maintenance_limit",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactMediaMaintenanceReport {
    pub publications_examined: u16,
    pub cleanups_processed: u16,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MediaPathsRequest {
    pub paths: Vec<String>,
}

impl MediaPathsRequest {
    pub fn validate(&self) -> Result<(), ArtifactMediaContractError> {
        if self.paths.is_empty() || self.paths.len() > 10_000 {
            return Err(ArtifactMediaContractError::InvalidCount {
                field: "paths",
                max: 10_000,
            });
        }
        if self.paths.iter().any(|path| path.trim().is_empty()) {
            return Err(ArtifactMediaContractError::BlankField { field: "paths[]" });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MediaPathPrefixRequest {
    pub prefix: String,
}

impl MediaPathPrefixRequest {
    pub fn validate(&self) -> Result<(), ArtifactMediaContractError> {
        validate_non_blank("prefix", &self.prefix)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectPublicationRequest {
    pub project_id: i64,
}

impl ProjectPublicationRequest {
    pub fn validate(&self) -> Result<(), ArtifactMediaContractError> {
        if self.project_id <= 0 {
            return Err(ArtifactMediaContractError::InvalidValue {
                field: "project_id",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectPathsRequest {
    pub project_id: i64,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PublishImageRequest {
    pub project_id: i64,
    pub publication_id: String,
    pub file_size: i64,
    pub sha256: String,
}

impl PublishImageRequest {
    pub fn validate(&self) -> Result<(), ArtifactMediaContractError> {
        ProjectPublicationRequest {
            project_id: self.project_id,
        }
        .validate()?;
        validate_non_blank("publication_id", &self.publication_id)?;
        if self.file_size < 0 {
            return Err(ArtifactMediaContractError::InvalidValue { field: "file_size" });
        }
        if self.sha256.len() != 64 || !self.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ArtifactMediaContractError::InvalidValue { field: "sha256" });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PublishedImageResponse {
    pub publication_id: String,
    pub image_id: i64,
    pub project_id: i64,
    pub image_type: String,
    pub file_path: String,
    pub file_size: i64,
    pub revision: i64,
}

#[async_trait::async_trait]
pub trait ArtifactMediaPublicationPort: Send + Sync {
    async fn publish_image(
        &self,
        actor_user_id: i64,
        request: PublishImageRequest,
    ) -> Result<PublishedImageResponse, String>;
}

#[async_trait::async_trait]
pub trait ArtifactMediaReferencePort: Send + Sync {
    async fn unreferenced_paths(&self, request: MediaPathsRequest) -> Result<Vec<String>, String>;

    async fn path_prefix_has_reference(
        &self,
        request: MediaPathPrefixRequest,
    ) -> Result<bool, String>;

    async fn project_has_publication(
        &self,
        request: ProjectPublicationRequest,
    ) -> Result<bool, String>;

    async fn protected_paths(&self, request: ProjectPathsRequest) -> Result<Vec<String>, String>;
}

impl ProjectPathsRequest {
    pub fn validate(&self) -> Result<(), ArtifactMediaContractError> {
        ProjectPublicationRequest {
            project_id: self.project_id,
        }
        .validate()?;
        MediaPathsRequest {
            paths: self.paths.clone(),
        }
        .validate()
    }
}

fn validate_non_blank(field: &'static str, value: &str) -> Result<(), ArtifactMediaContractError> {
    if value.trim().is_empty() {
        Err(ArtifactMediaContractError::BlankField { field })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{MediaPathsRequest, ProjectPathsRequest};

    #[test]
    fn cleanup_reference_requests_reject_invalid_scope_and_paths() {
        assert!(MediaPathsRequest { paths: vec![] }.validate().is_err());
        assert!(
            MediaPathsRequest {
                paths: vec![" ".to_string()]
            }
            .validate()
            .is_err()
        );
        assert!(
            ProjectPathsRequest {
                project_id: 0,
                paths: vec!["projects/1/a.png".to_string()],
            }
            .validate()
            .is_err()
        );
    }
}
