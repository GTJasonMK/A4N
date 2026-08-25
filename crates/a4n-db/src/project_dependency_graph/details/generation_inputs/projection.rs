use sea_orm::FromQueryResult;
use serde_json::Value;

use super::rules::{has_structural_lineage, should_expect_generation_inputs};
use crate::project_dependency_graph::{AssetLineageRecord, GenerationInputSource};

#[derive(Debug, Clone, FromQueryResult)]
pub(super) struct GenerationRunInputRecord {
    pub(super) id: i64,
    pub(super) rendered_messages_json: Option<Value>,
    pub(super) rendered_prompts_json: Option<Value>,
    pub(super) input_variables_json: Option<Value>,
    pub(super) source_refs_json: Option<Value>,
    pub(super) media_refs_json: Option<Value>,
    pub(super) request_json: Option<Value>,
    pub(super) response_summary_json: Option<Value>,
}

pub(super) fn apply_generation_runs(
    record: &mut AssetLineageRecord,
    runs: &[GenerationRunInputRecord],
) {
    let inputs = generation_inputs_from_runs(runs);

    record.input_status = if !runs.is_empty() && !inputs.is_empty() {
        "complete".to_string()
    } else if !runs.is_empty() {
        if should_expect_generation_inputs(record) {
            record
                .warnings
                .push("该模型生成资产有 generation_run，但缺少生成输入快照。".to_string());
        }
        "run_without_inputs".to_string()
    } else if should_expect_generation_inputs(record) {
        record
            .warnings
            .push("该模型生成资产缺少 generation_run 生成输入记录。".to_string());
        "missing".to_string()
    } else if has_structural_lineage(record) {
        "structural".to_string()
    } else {
        "not_applicable".to_string()
    };
    record.generation_inputs = inputs;
}

fn generation_inputs_from_runs(runs: &[GenerationRunInputRecord]) -> Vec<GenerationInputSource> {
    let mut inputs = Vec::new();
    for run in runs {
        push_json_input(
            &mut inputs,
            "prompt",
            "rendered_messages",
            "渲染消息",
            run.rendered_messages_json.clone(),
        );
        push_json_input(
            &mut inputs,
            "prompt",
            "rendered_prompts",
            "渲染提示词",
            run.rendered_prompts_json.clone(),
        );
        push_json_input(
            &mut inputs,
            "variables",
            "input_variables",
            "输入变量",
            run.input_variables_json.clone(),
        );
        push_json_input(
            &mut inputs,
            "source_text",
            "source_refs",
            "上游文本与资产",
            run.source_refs_json.clone(),
        );
        push_json_input(
            &mut inputs,
            "media",
            "media_refs",
            "参考媒体",
            run.media_refs_json.clone(),
        );
        push_json_input(
            &mut inputs,
            "request",
            "request",
            "模型请求",
            run.request_json.clone(),
        );
        push_json_input(
            &mut inputs,
            "response",
            "response_summary",
            "响应摘要",
            run.response_summary_json.clone(),
        );
    }
    inputs
}

fn push_json_input(
    inputs: &mut Vec<GenerationInputSource>,
    group: &str,
    kind: &str,
    label: &str,
    value: Option<Value>,
) {
    let Some(value) = value else {
        return;
    };
    if value.is_null() {
        return;
    }
    inputs.push(GenerationInputSource {
        group: group.to_string(),
        kind: kind.to_string(),
        label: label.to_string(),
        value,
    });
}
