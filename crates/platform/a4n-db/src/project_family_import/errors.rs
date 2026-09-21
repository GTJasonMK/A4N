use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum ProjectFamilyMediaImportError {
    #[error("{0}")]
    Write(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectFamilyImportError {
    #[error(transparent)]
    Database(#[from] DbErr),
    #[error("提交项目族导入事务失败: {0}")]
    Commit(#[source] DbErr),
    #[error("{context}: {source}")]
    DatabaseContext {
        context: &'static str,
        #[source]
        source: DbErr,
    },
    #[error("{0}")]
    InvalidPackage(String),
    #[error(transparent)]
    MediaImport(#[from] ProjectFamilyMediaImportError),
}

impl ProjectFamilyImportError {
    pub(in crate::project_family_import) fn invalid_package(message: impl Into<String>) -> Self {
        Self::InvalidPackage(message.into())
    }
}
