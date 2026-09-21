//! 人物立绘 Host 适配。

mod application;
mod generation_task;
mod host_port;
mod preview;

pub use a4n_api_contract::portrait::{
    GenerateAgeVariantInput, GenerateMissingPortraitsInput, GeneratePortraitInput,
    GeneratePortraitPromptInput, RegeneratePortraitInput,
};
pub use a4n_api_contract::portrait::{PortraitSummary, UploadPortraitInput};
pub(crate) use application::generate_portrait_inline;
pub use application::{generate_portrait_prompt, upload_portrait};
pub use generation_task::start_portrait_generation_typed;
pub use preview::{
    preview_generate_age_variant_request, preview_generate_missing_portraits_request,
    preview_generate_portrait_prompt_request, preview_generate_portrait_request,
    preview_regenerate_portrait_request,
};

#[cfg(test)]
mod tests;
