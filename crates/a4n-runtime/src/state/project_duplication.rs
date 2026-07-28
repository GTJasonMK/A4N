use a4n_core::{error::AppError, models::project::novel_project};
use a4n_orchestrator::{
    OrchestratorContext,
    project::{
        BranchProjectApplicationInput, DuplicateProjectApplicationInput, ProjectDuplicationHostPort,
    },
};

pub(crate) struct ProjectDuplicationRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ProjectDuplicationRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn duplicate_project_validated<Host>(
        &self,
        input: DuplicateProjectApplicationInput,
        host_port: &mut Host,
    ) -> Result<novel_project::Model, AppError>
    where
        Host: ProjectDuplicationHostPort,
    {
        self.orchestrator
            .duplicate_project_validated(input, host_port)
            .await
    }

    pub(crate) async fn branch_project_validated<Host>(
        &self,
        input: BranchProjectApplicationInput,
        host_port: &Host,
    ) -> Result<novel_project::Model, AppError>
    where
        Host: ProjectDuplicationHostPort,
    {
        self.orchestrator
            .branch_project_validated(input, host_port)
            .await
    }
}
