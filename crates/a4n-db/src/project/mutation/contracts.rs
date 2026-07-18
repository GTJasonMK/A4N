use sea_orm::DbErr;

pub const PROJECT_KIND_TEXT: &str = "text";
pub const PROJECT_KIND_COMIC: &str = "comic";
pub const PROJECT_KIND_VIDEO: &str = "video";
pub const PROJECT_KIND_GALGAME: &str = "galgame";
pub const SOURCE_MODE_SELF: &str = "self";
pub const SOURCE_MODE_DERIVED: &str = "derived";
pub const SOURCE_MODE_FREEFORM: &str = "freeform";
pub const PROJECT_TYPE_NOVEL_LONG: &str = "novel_long";
pub const PROJECT_TYPE_NOVEL_SHORT: &str = "novel_short";
pub const PROJECT_TYPE_SCRIPT: &str = "script";
pub const PROJECT_TYPE_PROSE: &str = "prose";
pub const SOURCE_SYNC_CURRENT: &str = "current";
pub const SOURCE_SYNC_ROOT_UPDATED: &str = "root_updated";
pub const SOURCE_SYNC_CONFLICT: &str = "conflict";
pub const SOURCE_SYNC_DETACHED: &str = "detached";
pub const DERIVED_PROJECT_STATUS_DRAFT: &str = "draft";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateProjectRecordInput {
    pub user_id: i64,
    pub title: String,
    pub description: String,
    pub project_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateFreeformVideoProjectParams {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateProjectRecordInput {
    pub project_id: i64,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectMutationError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("内部错误：{0}")]
    Internal(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}
