use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum ProjectRollbackImpactReadError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{operation}: {source}")]
    Database {
        operation: &'static str,
        #[source]
        source: DbErr,
    },
}

impl ProjectRollbackImpactReadError {
    pub(in crate::project::rollback_impact) fn database(
        operation: &'static str,
        source: DbErr,
    ) -> Self {
        Self::Database { operation, source }
    }
}
