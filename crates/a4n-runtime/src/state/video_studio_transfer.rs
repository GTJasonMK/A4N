use a4n_core::models::project::novel_project;
use a4n_orchestrator::{
    OrchestratorContext,
    project::{
        ExportVideoStudioProjectApplicationInput, ExportedVideoStudioProject,
        ImportVideoStudioProjectApplicationInput, VideoStudioProjectTransferError,
        VideoStudioProjectTransferHostPort,
    },
};

pub(crate) struct VideoStudioTransferRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> VideoStudioTransferRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn export_video_studio_project_validated<Host>(
        &self,
        input: ExportVideoStudioProjectApplicationInput,
        host_port: &Host,
    ) -> Result<ExportedVideoStudioProject, VideoStudioProjectTransferError>
    where
        Host: VideoStudioProjectTransferHostPort,
    {
        self.orchestrator
            .export_video_studio_project_validated(input, host_port)
            .await
    }

    pub(crate) async fn import_video_studio_project_validated<Host>(
        &self,
        input: ImportVideoStudioProjectApplicationInput,
        host_port: &mut Host,
    ) -> Result<novel_project::Model, VideoStudioProjectTransferError>
    where
        Host: VideoStudioProjectTransferHostPort,
    {
        self.orchestrator
            .import_video_studio_project_validated(input, host_port)
            .await
    }
}
