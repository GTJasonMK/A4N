use a4n_domain::character::CharacterProfileRecord;
use sea_orm::DbErr;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterPortraitRollbackResult {
    pub delete_file: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PortraitPromptProfileRows {
    pub profile: CharacterProfileRecord,
    pub project_context: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PortraitNamedPromptProfileRows {
    pub profile: CharacterProfileRecord,
    pub existing_portrait_count: u64,
    pub project_context: Option<Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum CharacterPortraitRollbackError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error("开启立绘失败回滚事务失败: {0}")]
    Begin(#[source] DbErr),
    #[error("查询立绘记录失败: {0}")]
    QueryPortrait(#[source] DbErr),
    #[error("锁定立绘镜像图片记录失败: {0}")]
    LockMirrorImages(#[source] DbErr),
    #[error("回滚立绘镜像图片媒体绑定失败: {0}")]
    DeleteMirrorImageBindings(#[source] DbErr),
    #[error("回滚立绘镜像图片引用失败: {0}")]
    ClearMirrorImageReferences(#[source] crate::video::VideoDbError),
    #[error("回滚立绘镜像图片主素材引用失败: {0}")]
    DeleteMirrorImagePrimaryMaterials(#[source] DbErr),
    #[error("回滚立绘镜像图片溯源目标失败: {0}")]
    DeleteMirrorImageTargets(#[source] crate::generation::GenerationDbError),
    #[error("回滚立绘镜像图片记录失败: {0}")]
    DeleteMirrorImages(#[source] DbErr),
    #[error("回滚立绘溯源目标失败: {0}")]
    DeletePortraitTargets(#[source] crate::generation::GenerationDbError),
    #[error("回滚立绘记录失败: {0}")]
    DeletePortrait(#[source] DbErr),
    #[error("检查立绘图片剩余引用失败: {0}")]
    CheckFileReferences(#[source] DbErr),
    #[error("提交立绘失败回滚事务失败: {0}")]
    Commit(#[source] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum CharacterPortraitRecordError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}
