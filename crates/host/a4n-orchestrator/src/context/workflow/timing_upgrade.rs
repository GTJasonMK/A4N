//! One-time reuse of migration-marked calls; ordinary fingerprint matching stays strict.

use a4n_domain::shooting_script::compat::{
    HashUpgrades,
    checkpoint::{TIMING_UPGRADE_KEY, is_timing_upgrade_step},
};
use serde_json::Value;

use super::*;

const OLD_DESIGN: &str = include_str!("timing_upgrade/design_v1.md");
const NEW_DESIGN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../prompts/novel/05_analysis/chapter_shooting_design_unit.md"
));
const OLD_TASK: &str =
    "根据当前单个导演镜头的动作节拍、自然耗时和候选片段时长，生成该镜头下的逐段视频片段规划";
const NEW_TASK: &str = "根据当前单个导演镜头的动作节拍和已确认时间范围，生成该镜头的视频片段规划";
const OLD_SPLIT_RULE: &str = "可以按用户选择时长把当前镜头拆成多个相邻 segments，但不得合并其他镜头、改写镜头顺序、把后续镜头动作提前或重新发明另一套导演方案。";
const NEW_SPLIT_RULE: &str = "每个导演镜头只生成一个 segment，时长取自导演时间轴；不得拆分或合并镜头、改写镜头顺序、把后续镜头动作提前或重新发明另一套导演方案。";

impl OrchestratorContext {
    pub(crate) fn current_shooting_design_prompt(prompt: String) -> Result<String, AppError> {
        let prompt = prompt.replace(template_body(OLD_DESIGN), template_body(NEW_DESIGN));
        let compact = prompt
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>();
        if compact.contains("\"estimated_seconds\":")
            || compact.contains("\"estimated_natural_seconds\":")
        {
            return Err(AppError::ConfigError("自定义 chapter_shooting_design_unit 模板仍要求旧估时字段，请更新输出合同后继续；已保存进度保留".into()));
        }
        Ok(prompt)
    }

    pub(super) async fn load_timing_upgraded_checkpoint(
        &self,
        workflow_run_id: i64,
        call_key: &str,
        input: &Value,
    ) -> Result<Option<CompletedLlmCallCheckpoint>, AppError> {
        if !is_timing_upgrade_step(call_key) {
            return Ok(None);
        }
        let candidate = self
            .workflow_checkpoint_models(workflow_run_id)
            .await?
            .into_iter()
            .find(|row| {
                row.step_key == call_key
                    && row.status == "completed"
                    && row
                        .artifacts_json
                        .as_ref()
                        .and_then(|a| a.get(TIMING_UPGRADE_KEY))
                        .is_some()
            });
        let Some(row) = candidate else {
            return Ok(None);
        };
        let artifacts = row
            .artifacts_json
            .as_ref()
            .expect("selected upgrade receipt");
        let checkpoint = a4n_db::workflow::completed_llm_call_checkpoint_from_model(&row, call_key)
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let receipt = &artifacts[TIMING_UPGRADE_KEY];
        if request_material(&receipt["upgraded_input_fingerprint_json"])?
            != request_material(input)?
        {
            return Ok(None);
        }
        let previous_input = row
            .input_fingerprint_json
            .as_ref()
            .ok_or_else(|| AppError::InvalidArgument(format!("{call_key} 升级断点缺少请求指纹")))?;
        let fingerprint = a4n_domain::generation::canonical_json_sha256(previous_input)?;
        if row.input_fingerprint_sha256.as_deref() != Some(fingerprint.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "{call_key} 升级断点的原始请求指纹不一致"
            )));
        }
        self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: call_key.into(),
            call_order: row.step_order,
            input_fingerprint_json: input.clone(),
            checkpoint: checkpoint.clone(),
        })
        .await?;
        tracing::info!(
            workflow_run_id,
            call_key,
            "已升级并复用旧合同调用断点，保留原始请求和响应"
        );
        Ok(Some(checkpoint))
    }
}

fn template_body(text: &str) -> &str {
    text.strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n").map(|(_, body)| body))
        .unwrap_or(text)
        .trim()
}

fn request_material(value: &Value) -> Result<Value, AppError> {
    let mut value = value.clone();
    HashUpgrades::default()
        .upgrade_embedded(&mut value)
        .map_err(AppError::InvalidArgument)?;
    normalize_fields(&mut value)?;
    Ok(value)
}

fn normalize_fields(value: &mut Value) -> Result<(), AppError> {
    match value {
        Value::Array(items) => {
            for item in items {
                normalize_fields(item)?;
            }
        }
        Value::Object(fields) => {
            // Cache boundaries and accounting hashes are derived from the fully compared
            // rendered messages. Keep model/configuration hashes and source identities.
            if fields.contains_key("messages") {
                fields.remove("cache_breakpoints");
                fields.remove("cache_scope_key");
            }
            if let Some(summary) = fields
                .get_mut("context_manifest_summary")
                .and_then(Value::as_object_mut)
            {
                for key in [
                    "dynamic_user_chars",
                    "dynamic_user_tokens_estimate",
                    "output_contract_chars",
                    "output_contract_tokens_estimate",
                ] {
                    summary.remove(key);
                }
                if let Some(packs) = summary.get_mut("packs").and_then(Value::as_array_mut) {
                    for pack in packs {
                        if let Some(variants) =
                            pack.get_mut("variants").and_then(Value::as_array_mut)
                        {
                            for variant in variants {
                                if let Some(variant) = variant.as_object_mut() {
                                    for key in ["content_sha256", "chars", "tokens_estimate"] {
                                        variant.remove(key);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(trace) = fields
                .get_mut("trace_context")
                .and_then(Value::as_object_mut)
            {
                for key in [
                    "input_fingerprint_sha256",
                    "generation_run_id",
                    "context_plan",
                    "context_pack_refs",
                ] {
                    trace.remove(key);
                }
            }
            if let Some(messages) = fields.get_mut("messages").and_then(Value::as_array_mut) {
                for message in messages {
                    if let Some(content) = message
                        .get("content")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                    {
                        message["content"] = Value::String(normalize_text(&content)?);
                    }
                }
            }
            for (key, item) in fields {
                if matches!(
                    key.as_str(),
                    "dynamic_user_content" | "output_contract" | "system_prompt"
                ) {
                    if let Some(text) = item.as_str() {
                        *item = Value::String(normalize_text(text)?);
                    }
                } else if key == "task" && item.as_str() == Some(OLD_TASK) {
                    *item = Value::String(NEW_TASK.into());
                } else if key == "shooting_script_projection_rule" {
                    if let Some(text) = item.as_str().filter(|text| text.ends_with(OLD_SPLIT_RULE))
                    {
                        *item = Value::String(text.replace(OLD_SPLIT_RULE, NEW_SPLIT_RULE));
                    }
                }
                normalize_fields(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn normalize_text(text: &str) -> Result<String, AppError> {
    let text = text.replace(template_body(OLD_DESIGN), template_body(NEW_DESIGN));
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.starts_with("- 片段数量和可拆分范围只服从 `output_contract.segment_count_rule`。")
        {
            lines.push(line.replace("存在 `current_shot.director_constraints` 时，", ""));
            continue;
        }
        if line.starts_with("只设计 narrative_unit_index=") {
            lines.push(line.replace("响应仍为 ShootingSceneV1", "响应仍为 ShootingSceneV2"));
            continue;
        }
        // Context packs and the video request template render one complete JSON value per
        // line. Do not parse/alter free text inside the source content of those values.
        if let Ok(mut value @ (Value::Object(_) | Value::Array(_))) =
            serde_json::from_str::<Value>(line)
        {
            let original = value.clone();
            HashUpgrades::default()
                .upgrade_embedded(&mut value)
                .map_err(AppError::InvalidArgument)?;
            normalize_fields(&mut value)?;
            if value != original || is_video_payload(&value) {
                lines.push(a4n_domain::generation::canonical_json_text(&value)?);
                continue;
            }
        }
        lines.push(line.to_owned());
    }
    let mut result = lines.join("\n");
    if text.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}

fn is_video_payload(value: &Value) -> bool {
    value.get("current_shot").is_some()
        || matches!(
            value.get("schema_version").and_then(Value::as_str),
            Some("video_execution_projection_v3") | Some("chapter_shooting_script_v2")
        )
        || value.get("source_shot_id").is_some() && value.get("action_beats").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_the_known_schema_change_matches_old_request_material() {
        let old = json!({"messages": [{"role": "system", "content": template_body(OLD_DESIGN)}, {"role": "user", "content": "只设计 narrative_unit_index=1 的 target_shot_id=shot-1。响应仍为 ShootingSceneV1"}], "temperature": 0.35, "context_manifest_summary": {"packs": [{"config_hash": "original-model"}]}});
        let mut new = old.clone();
        new["messages"][0]["content"] = json!(template_body(NEW_DESIGN));
        new["messages"][1]["content"] = json!(
            "只设计 narrative_unit_index=1 的 target_shot_id=shot-1。响应仍为 ShootingSceneV2"
        );
        assert_eq!(
            request_material(&old).unwrap(),
            request_material(&new).unwrap()
        );
        new["temperature"] = json!(0.8);
        assert_ne!(
            request_material(&old).unwrap(),
            request_material(&new).unwrap()
        );
        new["temperature"] = json!(0.35);
        new["context_manifest_summary"]["packs"][0]["config_hash"] = json!("another-model");
        assert_ne!(
            request_material(&old).unwrap(),
            request_material(&new).unwrap()
        );
    }
}
