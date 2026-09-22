pub use a4n_generation_artifact_contract::{ArtifactLocation, ArtifactOwnership, ArtifactRef};

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
