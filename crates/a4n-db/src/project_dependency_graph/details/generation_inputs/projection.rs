use a4n_core::models::generation::generation_run;
use serde_json::Value;

use super::rules::{has_structural_lineage, should_expect_generation_inputs};
use crate::project_dependency_graph::{
    AssetLineageRecord, GenerationInputSource, GenerationRunSnapshot,
};

pub(super) fn apply_generation_runs(
    record: &mut AssetLineageRecord,
    runs: &[generation_run::Model],
) {
    let inputs = generation_inputs_from_runs(runs);
    record.generation_runs = runs.iter().map(generation_run_snapshot).collect();

    record.input_status = if !record.generation_runs.is_empty() && !inputs.is_empty() {
        "complete".to_string()
    } else if !record.generation_runs.is_empty() {
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

fn generation_run_snapshot(run: &generation_run::Model) -> GenerationRunSnapshot {
    GenerationRunSnapshot {
        id: Some(run.id),
        workflow: run.workflow.clone(),
        step_key: run.step_key.clone(),
        title: run.title.clone(),
        model_kind: run.model_kind.clone(),
        provider: run.provider.clone(),
        model: run.model.clone(),
        status: run.status.clone(),
        created_at: Some(run.created_at.to_string()),
        completed_at: run.completed_at.map(|value| value.to_string()),
    }
}

fn generation_inputs_from_runs(runs: &[generation_run::Model]) -> Vec<GenerationInputSource> {
    let mut inputs = Vec::new();
    for run in runs {
        push_json_input(
            &mut inputs,
            Some(run.id),
            "prompt",
            "rendered_messages",
            "渲染消息",
            run.rendered_messages_json.clone(),
        );
        push_json_input(
            &mut inputs,
            Some(run.id),
            "prompt",
            "rendered_prompts",
            "渲染提示词",
            run.rendered_prompts_json.clone(),
        );
        push_json_input(
            &mut inputs,
            Some(run.id),
            "variables",
            "input_variables",
            "输入变量",
            run.input_variables_json.clone(),
        );
        push_json_input(
            &mut inputs,
            Some(run.id),
            "source_text",
            "source_refs",
            "上游文本与资产",
            run.source_refs_json.clone(),
        );
        push_json_input(
            &mut inputs,
            Some(run.id),
            "media",
            "media_refs",
            "参考媒体",
            run.media_refs_json.clone(),
        );
        push_json_input(
            &mut inputs,
            Some(run.id),
            "request",
            "request",
            "模型请求",
            run.request_json.clone(),
        );
        push_json_input(
            &mut inputs,
            Some(run.id),
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
    run_id: Option<i64>,
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
        id: format!("{}:{}:{}", run_id.unwrap_or_default(), group, kind),
        run_id,
        group: group.to_string(),
        kind: kind.to_string(),
        label: label.to_string(),
        content_type: "json".to_string(),
        text: None,
        value: Some(value),
        source_record_id: None,
    });
}
