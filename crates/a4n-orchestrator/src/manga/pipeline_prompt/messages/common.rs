use a4n_error::AppError;
use a4n_llm_context::{
    COMPRESSION_FULL_RAW, LlmContextLedger, LlmContextLedgerPackDraft, LlmContextLedgerPackInput,
    LlmContextManifest,
};
use a4n_prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

pub(in crate::manga) fn build_manga_step_user_message(
    prompt_name: &str,
    instruction: &str,
    input: Value,
    output_contract: Value,
) -> Result<LlmContextManifest, AppError> {
    let mut input = input
        .as_object()
        .cloned()
        .ok_or_else(|| AppError::InvalidArgument("漫画步骤 input 必须是对象".into()))?;
    let mut contract = output_contract
        .as_object()
        .cloned()
        .ok_or_else(|| AppError::InvalidArgument("漫画步骤 output_contract 必须是对象".into()))?;
    let current_page = input.remove("current_page");
    let page_number = contract.remove("page_number");
    let mut ledger = LlmContextLedger::new();
    let mut add = |kind: String, content: Value| {
        ledger.add_pack(LlmContextLedgerPackInput {
            scope_kind: "manga_pipeline".into(),
            pack_kind: kind,
            source_refs_json: None,
            config_identity_json: json!({"version": 2}),
            required: true,
            stable_prefix: true,
            variants: vec![LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                content,
            )],
        })
    };
    add(
        "manga_output_contract".into(),
        json!({"output_contract": contract}),
    )?;
    for (key, value) in input {
        add(format!("manga_input_{key}"), json!({"input": {key: value}}))?;
    }
    let mut dynamic_input = serde_json::Map::new();
    if let Some(page) = current_page {
        dynamic_input.insert("current_page".into(), page);
    }
    let mut dynamic_contract = serde_json::Map::new();
    if let Some(number) = page_number {
        dynamic_contract.insert("page_number".into(), number);
    }
    let payload = json!({
        "task": {
            "prompt_name": prompt_name,
            "instruction": instruction
        },
        "output_contract": dynamic_contract,
        "input": dynamic_input,
        "context_refs": "input 和 output_contract 由稳定上下文中同名字段与当前任务中同名字段共同组成；必须使用两部分的完整内容。"
    });
    let task = build_json_task_user_message(
        "执行漫画流水线步骤",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )?;
    Ok(ledger.into_manifest(task, None))
}
