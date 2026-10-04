use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum GalgameAssetPlanSpecRecordError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameGenerationRecordError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameMediaRecordError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameProjectRecordError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameContextRecordError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameScriptRecordError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    StoryGraph(#[from] GalgameStoryGraphRecordError),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameStoryGraphRecordError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("节点图 revision 冲突：expected={expected_revision}, current={current_revision}")]
    RevisionConflict {
        expected_revision: i64,
        current_revision: i64,
    },
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameStoryRuntimeRecordError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error(transparent)]
    StoryGraph(#[from] GalgameStoryGraphRecordError),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameLineReorderRecordError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameLineRecordError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameSceneRecordError {
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}
