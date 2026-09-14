use async_trait::async_trait;

pub use a4n_domain::ProviderError;
pub use a4n_image_contract::{
    ImageGenerationOutput, ImageGenerationRequest, ImageProviderPayload, ImageReference,
};

/// 单次生成结果
#[derive(Debug, Clone)]
pub struct ImageGenerationResult {
    /// 原始字节（PNG/JPEG/WEBP，由 provider 决定）
    pub bytes: Vec<u8>,
    /// 输出扩展名（不含点，如 "png"）
    pub format: String,
    /// 实际使用的模型
    pub model: String,
}

#[async_trait]
pub trait ImageProvider: Send + Sync {
    async fn generate(
        &self,
        req: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, ProviderError>;
}
