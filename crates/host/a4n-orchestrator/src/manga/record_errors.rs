use a4n_db::manga::{MangaPipelineDbError, MangaPromptRecordError};
use a4n_domain::manga::MangaPipelineArtifactStage;
use a4n_error::AppError;

pub(in crate::manga) fn manga_stage_output_exists_error(
    stage: MangaPipelineArtifactStage,
    page_number: Option<u32>,
) -> AppError {
    let label = match stage {
        MangaPipelineArtifactStage::Preprocessing => "漫画预处理",
        MangaPipelineArtifactStage::PageSlots => "分页",
        MangaPipelineArtifactStage::PageContent => "页面内容",
        MangaPipelineArtifactStage::PageAdaptation => "页面改编",
        MangaPipelineArtifactStage::Storyboard => "分镜",
        MangaPipelineArtifactStage::ReferenceDiscovery => "参考图发现",
        MangaPipelineArtifactStage::ReferenceBindings => "参考图绑定",
        MangaPipelineArtifactStage::DrawingPrompt => "绘图提示词",
        MangaPipelineArtifactStage::Image => "页面成图",
    };
    let scope = page_number
        .map(|page| format!("第 {page} 页的"))
        .unwrap_or_else(|| "本章节的".to_string());
    AppError::InvalidState {
        current: format!("manga_stage_output_exists:{}", stage.key()),
        operation: format!("{scope}{label}产物已存在，请先删除后再生成"),
    }
}

pub(crate) fn manga_prompt_record_error(error: MangaPromptRecordError) -> AppError {
    match error {
        MangaPromptRecordError::InvalidArgument(message) => AppError::InvalidArgument(message),
        MangaPromptRecordError::NotFound(message) => AppError::NotFound(message),
        MangaPromptRecordError::Conflict(message) => AppError::InvalidState {
            current: message,
            operation: "生成漫画阶段产物".to_string(),
        },
        MangaPromptRecordError::Database { source, .. } => AppError::database(source),
        MangaPromptRecordError::Generation(error) => crate::generation::generation_error(error),
    }
}

pub(in crate::manga) fn manga_pipeline_db_error(error: MangaPipelineDbError) -> AppError {
    match error {
        MangaPipelineDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        MangaPipelineDbError::Database(error) => AppError::database(error),
        MangaPipelineDbError::Generation(error) => crate::generation::generation_error(error),
    }
}
