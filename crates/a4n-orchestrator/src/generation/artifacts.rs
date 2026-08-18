use a4n_service_protocol::{ArtifactEvent, ArtifactOperation, GenerationArtifactRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedMediaArtifactFile {
    pub content_type: String,
    pub sha256: String,
    pub byte_size: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedArtifactEvent(ArtifactEvent);

impl PreparedArtifactEvent {
    pub(crate) fn artifact_ref(&self) -> &GenerationArtifactRef {
        &self.0.artifact
    }

    pub(crate) fn into_event(self) -> ArtifactEvent {
        self.0
    }
}

/// generation artifact 事件的唯一构造入口。
pub(crate) struct GenerationArtifactCoordinator;

impl GenerationArtifactCoordinator {
    pub(crate) fn available(
        artifact: GenerationArtifactRef,
        item_key: Option<String>,
        item_index: Option<u32>,
        item_total: Option<u32>,
    ) -> PreparedArtifactEvent {
        PreparedArtifactEvent(ArtifactEvent {
            artifact,
            operation: ArtifactOperation::Available,
            replaces_artifact_id: None,
            item_key,
            item_index,
            item_total,
        })
    }

    pub(crate) fn generated_media_available(
        artifact_id: String,
        artifact_kind: &str,
        file: GeneratedMediaArtifactFile,
        item_key: Option<String>,
        item_index: Option<u32>,
        item_total: Option<u32>,
    ) -> PreparedArtifactEvent {
        let revision = file.sha256.clone();
        Self::available(
            GenerationArtifactRef {
                artifact_id,
                artifact_kind: artifact_kind.to_string(),
                content_type: file.content_type,
                sha256: file.sha256,
                byte_size: file.byte_size,
                revision,
                temporary: false,
                expires_at_ms: None,
            },
            item_key,
            item_index,
            item_total,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_artifact_never_exposes_location_or_replacement() {
        let event = GenerationArtifactCoordinator::available(
            GenerationArtifactRef {
                artifact_id: "portrait:42".to_string(),
                artifact_kind: "portrait_image".to_string(),
                content_type: "image/png".to_string(),
                sha256: "a".repeat(64),
                byte_size: 4,
                revision: "a".repeat(64),
                temporary: false,
                expires_at_ms: None,
            },
            Some("portrait-profile-7".to_string()),
            Some(0),
            Some(1),
        );

        let event = event.into_event();
        assert_eq!(event.operation, ArtifactOperation::Available);
        assert_eq!(event.replaces_artifact_id, None);
        assert!(!event.artifact.temporary);
        assert_eq!(event.artifact.expires_at_ms, None);
    }
}
