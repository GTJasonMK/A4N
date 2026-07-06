use std::collections::{BTreeSet, HashMap};

use a4n_core::error::AppError;
use a4n_engine::prompt::engine::PromptEngine;
use serde_json::{Value, json};

use crate::chapter::source_spans::{SourceSpan, source_span_prompt_values};
use crate::llm_context::{
    COMPRESSION_COMPRESSED_SCENE, COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW,
    COMPRESSION_MINIMAL, LlmContextLedger, LlmContextLedgerPackDraft, LlmContextManifest,
};
use crate::source::source_content_sha256;

use super::super::ChapterVideoSourceContext;
use super::final_video_prompt_public_context;

const VIDEO_CONTEXT_SCOPE: &str = "chapter";
const VIDEO_CONTEXT_LEDGER_VERSION: &str = "video_context_ledger_v1";
const VIDEO_BUILDER_RULES_TEMPLATE: &str = "chapter_video_builder_rules";

pub(in crate::video) const VIDEO_PREVIOUS_OUTPUT_SHOT_ANALYSIS: &str =
    "video_previous_output:shot_analysis";
pub(in crate::video) const VIDEO_PREVIOUS_OUTPUT_BEAT_EXTRACTION: &str =
    "video_previous_output:beat_extraction";
pub(in crate::video) const VIDEO_PREVIOUS_OUTPUT_SEGMENT_PLAN: &str =
    "video_previous_output:segment_plan";

pub(in crate::video) struct VideoContextPreviousOutput<'a> {
    pub pack_kind: &'a str,
    pub prompt_name: &'a str,
    pub parsed_output: &'a Value,
    pub raw_output: Option<&'a str>,
    pub placeholder: bool,
}

pub(in crate::video) struct VideoContextManifestInput<'a> {
    pub prompt_engine: &'a dyn PromptEngine,
    pub project_id: i64,
    pub variables: &'a HashMap<String, Value>,
    pub source: &'a ChapterVideoSourceContext<'a>,
    pub source_spans: &'a [SourceSpan],
    pub prompt_names: &'a [&'a str],
    pub previous_outputs: &'a [VideoContextPreviousOutput<'a>],
    pub project_context: Option<&'a Value>,
    pub asset_context: Option<&'a Value>,
    pub dynamic_user_content: String,
}

pub(in crate::video) fn build_video_context_manifest(
    input: VideoContextManifestInput<'_>,
) -> Result<LlmContextManifest, AppError> {
    let VideoContextManifestInput {
        prompt_engine,
        project_id,
        variables,
        source,
        source_spans,
        prompt_names,
        previous_outputs,
        project_context,
        asset_context,
        dynamic_user_content,
    } = input;
    let mut ledger = LlmContextLedger::new();
    add_chapter_source_pack(&mut ledger, source, source_spans)?;
    add_builder_rules_pack(&mut ledger, prompt_engine, project_id)?;
    add_prompt_rule_packs(
        &mut ledger,
        prompt_engine,
        project_id,
        variables,
        prompt_names,
    )?;
    for previous_output in previous_outputs {
        add_previous_output_pack(&mut ledger, previous_output)?;
    }
    if let Some(project_context) = project_context {
        add_optional_value_pack(
            &mut ledger,
            "video_project_context",
            json!({"ledger_version": VIDEO_CONTEXT_LEDGER_VERSION, "kind": "project_context"}),
            project_context,
        )?;
    }
    if let Some(asset_context) = asset_context {
        add_optional_value_pack(
            &mut ledger,
            "video_asset_bindings",
            json!({"ledger_version": VIDEO_CONTEXT_LEDGER_VERSION, "kind": "asset_context"}),
            asset_context,
        )?;
    }
    Ok(ledger.into_manifest(dynamic_user_content, None))
}

pub(in crate::video) fn build_final_video_context_manifest(
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
    variables: &HashMap<String, Value>,
    source: &ChapterVideoSourceContext<'_>,
    prompt_names: &[&str],
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    let mut ledger = LlmContextLedger::new();
    add_public_chapter_source_pack(&mut ledger, source)?;
    add_prompt_rule_hash_pack(
        &mut ledger,
        prompt_engine,
        project_id,
        variables,
        prompt_names,
    )?;
    Ok(ledger.into_manifest(dynamic_user_content, None))
}

fn add_chapter_source_pack(
    ledger: &mut LlmContextLedger,
    source: &ChapterVideoSourceContext<'_>,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let content_sha256 = source_content_sha256(source.content);
    let full = json!({
        "kind": "chapter_source",
        "content": source.content,
        "summary": trimmed_optional(source.summary),
        "analysis": source.analysis,
        "source_spans": source_span_prompt_values(source_spans)
    });
    let focused = json!({
        "kind": "chapter_source",
        "summary": trimmed_optional(source.summary),
        "analysis": source.analysis,
        "source_spans": source_span_prompt_values(source_spans)
    });
    let compressed = json!({
        "kind": "chapter_source",
        "summary": trimmed_optional(source.summary),
        "analysis": source.analysis,
        "source_span_count": source_spans.len(),
        "content_sha256": content_sha256
    });
    let minimal = json!({
        "kind": "chapter_source",
        "summary": trimmed_optional(source.summary),
        "source_span_count": source_spans.len(),
        "content_sha256": content_sha256
    });
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: (VIDEO_CONTEXT_SCOPE).into(),
        pack_kind: ("video_chapter_source").into(),
        source_refs_json: Some(json!({
            "content_sha256": content_sha256,
            "source_span_count": source_spans.len()
        })),
        config_identity_json: json!({
            "ledger_version": VIDEO_CONTEXT_LEDGER_VERSION,
            "kind": "chapter_source",
            "content_sha256": content_sha256
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, full),
            LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, focused),
            LlmContextLedgerPackDraft::new(COMPRESSION_COMPRESSED_SCENE, compressed),
            LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, minimal),
        ],
    })
}

fn add_public_chapter_source_pack(
    ledger: &mut LlmContextLedger,
    source: &ChapterVideoSourceContext<'_>,
) -> Result<(), AppError> {
    let content_sha256 = source_content_sha256(source.content);
    let public_analysis = source.analysis.map(final_video_prompt_public_context);
    let full = json!({
        "kind": "chapter_source_public",
        "content": source.content,
        "summary": trimmed_optional(source.summary),
        "analysis": public_analysis
    });
    let focused = json!({
        "kind": "chapter_source_public",
        "summary": trimmed_optional(source.summary),
        "analysis": public_analysis,
        "content_sha256": content_sha256
    });
    let minimal = json!({
        "kind": "chapter_source_public",
        "summary": trimmed_optional(source.summary),
        "content_sha256": content_sha256
    });
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: (VIDEO_CONTEXT_SCOPE).into(),
        pack_kind: ("video_chapter_source_public").into(),
        source_refs_json: Some(json!({
            "content_sha256": content_sha256
        })),
        config_identity_json: json!({
            "ledger_version": VIDEO_CONTEXT_LEDGER_VERSION,
            "kind": "chapter_source_public",
            "content_sha256": content_sha256
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, full),
            LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, focused),
            LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, minimal),
        ],
    })
}

fn add_builder_rules_pack(
    ledger: &mut LlmContextLedger,
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
) -> Result<(), AppError> {
    let rendered = prompt_engine.render(
        VIDEO_BUILDER_RULES_TEMPLATE,
        &HashMap::new(),
        Some(project_id),
    )?;
    let rendered_sha256 = source_content_sha256(&rendered);
    let rules = serde_json::from_str::<Value>(&rendered).map_err(|err| {
        AppError::ConfigError(format!(
            "视频 builder 规则模板必须渲染为合法 JSON 对象：{err}"
        ))
    })?;
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: (VIDEO_CONTEXT_SCOPE).into(),
        pack_kind: ("video_workflow_rules").into(),
        source_refs_json: Some(json!({"prompt_name": VIDEO_BUILDER_RULES_TEMPLATE})),
        config_identity_json: json!({
            "ledger_version": VIDEO_CONTEXT_LEDGER_VERSION,
            "prompt_name": VIDEO_BUILDER_RULES_TEMPLATE,
            "rendered_sha256": rendered_sha256
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "prompt_name": VIDEO_BUILDER_RULES_TEMPLATE,
                    "rules": rules
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "prompt_name": VIDEO_BUILDER_RULES_TEMPLATE,
                    "rendered_sha256": rendered_sha256
                }),
            ),
        ],
    })
}

fn add_prompt_rule_hash_pack(
    ledger: &mut LlmContextLedger,
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
    variables: &HashMap<String, Value>,
    prompt_names: &[&str],
) -> Result<(), AppError> {
    let builder_rules = prompt_engine.render(
        VIDEO_BUILDER_RULES_TEMPLATE,
        &HashMap::new(),
        Some(project_id),
    )?;
    let mut prompts = Vec::new();
    prompts.push(json!({
        "prompt_name": VIDEO_BUILDER_RULES_TEMPLATE,
        "rendered_sha256": source_content_sha256(&builder_rules)
    }));
    let mut seen = BTreeSet::new();
    for prompt_name in prompt_names {
        if !seen.insert(*prompt_name) {
            continue;
        }
        let rendered = prompt_engine.render(prompt_name, variables, Some(project_id))?;
        prompts.push(json!({
            "prompt_name": prompt_name,
            "rendered_sha256": source_content_sha256(&rendered)
        }));
    }
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: (VIDEO_CONTEXT_SCOPE).into(),
        pack_kind: ("video_prompt_rule_hashes").into(),
        source_refs_json: Some(json!({
            "prompt_names": prompts
                .iter()
                .filter_map(|prompt| prompt.get("prompt_name").and_then(Value::as_str))
                .collect::<Vec<_>>()
        })),
        config_identity_json: json!({
            "ledger_version": VIDEO_CONTEXT_LEDGER_VERSION,
            "kind": "prompt_rule_hashes",
            "prompt_names": prompts
                .iter()
                .filter_map(|prompt| prompt.get("prompt_name").and_then(Value::as_str))
                .collect::<Vec<_>>()
        }),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_MINIMAL,
            json!({
                "kind": "prompt_rule_hashes",
                "prompts": prompts
            }),
        )],
    })
}

fn add_prompt_rule_packs(
    ledger: &mut LlmContextLedger,
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
    variables: &HashMap<String, Value>,
    prompt_names: &[&str],
) -> Result<(), AppError> {
    let mut seen = BTreeSet::new();
    for prompt_name in prompt_names {
        if !seen.insert(*prompt_name) {
            continue;
        }
        let rendered = prompt_engine.render(prompt_name, variables, Some(project_id))?;
        let rendered_sha256 = source_content_sha256(&rendered);
        let pack_kind = format!("video_prompt_rules:{prompt_name}");
        ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
            scope_kind: (VIDEO_CONTEXT_SCOPE).into(),
            pack_kind,
            source_refs_json: Some(json!({"prompt_name": prompt_name})),
            config_identity_json: json!({
                "ledger_version": VIDEO_CONTEXT_LEDGER_VERSION,
                "prompt_name": prompt_name,
                "rendered_sha256": rendered_sha256
            }),
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_FULL_RAW,
                    json!({
                        "prompt_name": prompt_name,
                        "rendered_prompt": rendered
                    }),
                ),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_MINIMAL,
                    json!({
                        "prompt_name": prompt_name,
                        "rendered_sha256": rendered_sha256
                    }),
                ),
            ],
        })?;
    }
    Ok(())
}

fn add_previous_output_pack(
    ledger: &mut LlmContextLedger,
    output: &VideoContextPreviousOutput<'_>,
) -> Result<(), AppError> {
    let raw_sha256 = output.raw_output.map(source_content_sha256);
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: (VIDEO_CONTEXT_SCOPE).into(),
        pack_kind: (output.pack_kind).into(),
        source_refs_json: Some(json!({
            "prompt_name": output.prompt_name,
            "placeholder": output.placeholder
        })),
        config_identity_json: json!({
            "ledger_version": VIDEO_CONTEXT_LEDGER_VERSION,
            "prompt_name": output.prompt_name,
            "pack_kind": output.pack_kind,
            "raw_sha256": raw_sha256,
            "placeholder": output.placeholder
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "prompt_name": output.prompt_name,
                    "placeholder": output.placeholder,
                    "raw_output": output.raw_output,
                    "parsed_output": output.parsed_output
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "prompt_name": output.prompt_name,
                    "placeholder": output.placeholder,
                    "parsed_output": output.parsed_output
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "prompt_name": output.prompt_name,
                    "placeholder": output.placeholder,
                    "raw_sha256": raw_sha256
                }),
            ),
        ],
    })
}

fn add_optional_value_pack(
    ledger: &mut LlmContextLedger,
    pack_kind: &str,
    config_identity: Value,
    value: &Value,
) -> Result<(), AppError> {
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: (VIDEO_CONTEXT_SCOPE).into(),
        pack_kind: (pack_kind).into(),
        source_refs_json: None,
        config_identity_json: config_identity,
        required: false,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, value.clone()),
            LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, json!({
                "value_sha256": source_content_sha256(&serde_json::to_string(value).map_err(AppError::Serialization)?)
            })),
        ],
    })
}

fn trimmed_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_engine::prompt::tera_engine::TeraPromptEngine;
    use std::path::Path;

    const PROJECT_ID: i64 = 1;

    fn prompt_engine() -> TeraPromptEngine {
        let prompts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
        TeraPromptEngine::from_directory(&prompts_dir).expect("prompts should load")
    }

    fn contains_json_key(value: &Value, target: &str) -> bool {
        match value {
            Value::Array(items) => items.iter().any(|item| contains_json_key(item, target)),
            Value::Object(object) => {
                object.contains_key(target)
                    || object
                        .values()
                        .any(|value| contains_json_key(value, target))
            }
            _ => false,
        }
    }

    #[test]
    fn final_video_context_manifest_excludes_internal_source_fields() {
        let engine = prompt_engine();
        let analysis = json!({
            "scene": "教室窗边对白",
            "source_span_ids": ["ch1:s1:f1"],
            "nested": {
                "source_span_id": "ch1:s1:f1",
                "dialogue_beats": [{
                    "speaker": "林清辞",
                    "line": "等等",
                    "delivery": "低声",
                    "visible_action": "嘴唇微动",
                    "listener_reaction": "对方停步",
                    "timing": "中段",
                    "source_span_ids": ["ch1:s1:f1"]
                }]
            }
        });
        let source = ChapterVideoSourceContext {
            content: "林清辞站在窗边低声说：“等等。”",
            summary: Some("窗边对白"),
            analysis: Some(&analysis),
        };
        let manifest = build_final_video_context_manifest(
            &engine,
            PROJECT_ID,
            &HashMap::new(),
            &source,
            &["chapter_video_prompt", "chapter_video_prompt_review"],
            "请根据以下 JSON 输入生成视频提示词".to_string(),
        )
        .expect("final video context manifest should build");
        let manifest_json = serde_json::to_value(&manifest).expect("manifest should serialize");
        let manifest_text = serde_json::to_string(&manifest_json).expect("manifest text");

        assert!(
            manifest
                .packs
                .iter()
                .all(|pack| !pack.pack_kind.starts_with("video_previous_output:")),
            "final video manifest must not include raw previous-output packs"
        );
        for key in [
            "source_span_id",
            "source_span_ids",
            "key_source_span_ids",
            "source_span_range",
            "reference_image_ids",
        ] {
            assert!(
                !contains_json_key(&manifest_json, key),
                "final video context manifest must not expose internal key {key}"
            );
            assert!(
                !manifest_text.contains(key),
                "final video context manifest text must not expose internal field name {key}"
            );
        }
        assert!(
            manifest
                .packs
                .iter()
                .any(|pack| pack.pack_kind == "video_chapter_source_public"),
            "final video context should keep public chapter source for dialogue extraction"
        );
        assert!(
            manifest
                .packs
                .iter()
                .any(|pack| pack.pack_kind == "video_prompt_rule_hashes"),
            "final video context should keep prompt hashes for checkpoint invalidation"
        );
    }
}
