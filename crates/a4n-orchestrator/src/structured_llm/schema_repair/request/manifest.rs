use a4n_core::error::AppError;
use a4n_llm::{
    context::{LlmContextManifest, replace_request_user_content_from_manifest},
    provider::ChatRequest,
};

pub(super) enum RepairOutputContract<'a> {
    Replace(&'a str),
    Append(&'a str),
}

pub(super) fn rewrite_manifest_repair_request(
    request: &mut ChatRequest,
    repair_task: &str,
    output_contract: RepairOutputContract<'_>,
) -> Result<bool, AppError> {
    let Some(manifest_json) = request.context_manifest_json.as_ref() else {
        return Ok(false);
    };
    let mut manifest: LlmContextManifest =
        serde_json::from_value(manifest_json.clone()).map_err(|error| {
            AppError::InvalidArgument(format!("结构纠正 LLM 上下文 manifest 解析失败：{error}"))
        })?;
    manifest.dynamic_user_content = append_section(
        &manifest.dynamic_user_content,
        "[结构纠正任务]",
        repair_task,
    );
    manifest.output_contract = match output_contract {
        RepairOutputContract::Replace(contract) => Some(contract.to_string()),
        RepairOutputContract::Append(contract) => Some(append_section(
            manifest.output_contract.as_deref().unwrap_or_default(),
            "[结构纠正规则]",
            contract,
        )),
    };
    replace_request_user_content_from_manifest(request, &manifest, &[])?;
    request.context_manifest_json =
        Some(serde_json::to_value(manifest).map_err(AppError::Serialization)?);
    Ok(true)
}

fn append_section(base: &str, heading: &str, content: &str) -> String {
    let base = base.trim();
    if base.is_empty() {
        format!("{heading}\n{content}")
    } else {
        format!("{base}\n\n{heading}\n{content}")
    }
}
