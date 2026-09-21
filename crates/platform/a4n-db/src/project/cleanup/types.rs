mod error;
mod results;

pub use error::ProjectCleanupDbError;
pub(crate) use results::{
    ProjectBlueprintCleanupResult, ProjectChapterAdaptationDeleteScope,
    ProjectConversationCleanupResult, ProjectGeneratedArtifactLinkCleanupResult,
    ProjectGeneratedArtifactRowCleanupResult, ProjectGenerationMediaGraphCleanupResult,
    ProjectOptimizationCleanupResult, ProjectTaskQueueCleanupResult,
    ProjectTextAnchorCleanupResult, ProjectTextAnchorDeleteScope,
};
