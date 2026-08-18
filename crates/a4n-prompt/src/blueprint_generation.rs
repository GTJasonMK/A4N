//! Blueprint generation prompt request contracts.

use a4n_core::error::AppError;
use a4n_llm::provider::{ChatRequest, is_truncated_finish_reason};

mod refine;
mod stage_request;
mod stage_response;

pub use refine::build_blueprint_refine_request;
pub use stage_request::{BlueprintStageRequestInput, build_blueprint_stage_request};
pub use stage_response::parse_blueprint_stage_json_object;

pub struct BlueprintGenerationPreviewStep {
    pub key: String,
    pub title: String,
    pub prompt_name: String,
    pub request: ChatRequest,
}

pub fn ensure_blueprint_response_not_truncated(
    finish_reason: Option<&str>,
    label: &str,
) -> Result<(), AppError> {
    if is_truncated_finish_reason(finish_reason) {
        return Err(AppError::InvalidArgument(format!(
            "{label} 输出达到当前蓝图 max_tokens 上限，返回 JSON 被截断；请在设置页调大“蓝图”输出档位，或减少本次蓝图输入/输出规模后重试"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_blueprint_length_finish_reasons() {
        for finish_reason in ["length", "max_tokens"] {
            let error =
                ensure_blueprint_response_not_truncated(Some(finish_reason), "蓝图故事核心")
                    .expect_err("truncated blueprint response must fail");
            assert_eq!(
                error.to_string(),
                "参数无效：蓝图故事核心 输出达到当前蓝图 max_tokens 上限，返回 JSON 被截断；请在设置页调大“蓝图”输出档位，或减少本次蓝图输入/输出规模后重试"
            );
        }
    }

    #[test]
    fn accepts_non_truncated_blueprint_finish_reasons() {
        ensure_blueprint_response_not_truncated(Some("stop"), "蓝图故事核心").unwrap();
        ensure_blueprint_response_not_truncated(None, "蓝图故事核心").unwrap();
    }
}
