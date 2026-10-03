use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use serde::{Serialize, de::DeserializeOwned};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    structured_llm::{
        StructuredLlmProgress, ensure_checkpoint_parsed_output_matches, parse_strict_json_response,
    },
    workflow::{llm_call_checkpoint_fingerprint_sha256, workflow_checkpoint_contract_app_error},
};

use super::contracts::{
    ShootingScriptProgressReporter, ShootingScriptStageResult, ShootingScriptStreamReporter,
};

mod persistence;
mod raw_call;

const RAW_RESPONSE_ONLY_ARTIFACT: &str = "shooting_script_raw_response_only_v1";

impl OrchestratorContext {
    pub(super) async fn run_shooting_script_stage<T>(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        step_order: i32,
        mut request: ChatRequest,
        label: &str,
        structured_progress: StructuredLlmProgress,
        progress_total: u32,
        progress: ShootingScriptProgressReporter<'_>,
        stream_reporter: ShootingScriptStreamReporter<'_>,
        cancel_token: &CancellationToken,
        validate_output: impl Fn(&T) -> Result<(), AppError>,
    ) -> Result<ShootingScriptStageResult<T>, AppError>
    where
        T: Serialize + DeserializeOwned,
    {
        self.ensure_shooting_script_running(workflow_run_id).await?;
        let fingerprint_json = a4n_llm_trace::context_aware_chat_request_trace_json(&request)?;
        let fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)
            .map_err(workflow_checkpoint_contract_app_error)?;
        if let Some(trace) = request.trace_context.as_mut() {
            trace.input_fingerprint_sha256 = Some(fingerprint_sha256);
        }
        let raw = self
            .run_or_restore_raw_shooting_call(
                workflow_run_id,
                step_key,
                step_order,
                &fingerprint_json,
                request,
                label,
                structured_progress,
                progress_total,
                progress,
                stream_reporter,
                cancel_token,
            )
            .await?;
        let restored = raw.restored;
        let response_only_checkpoint = raw.response_only_checkpoint;
        let parsed = parse_shooting_script_output::<T>(
            &raw.response_content,
            label,
            step_key,
            raw.upgraded_contract,
        );
        let output = parsed.and_then(|output| {
            validate_output(&output)?;
            if let Some(saved) = raw
                .restored_parsed_output
                .as_ref()
                .filter(|_| !response_only_checkpoint)
            {
                ensure_checkpoint_parsed_output_matches(
                    saved,
                    &serde_json::to_value(&output)?,
                    &format!("分镜脚本 {step_key}"),
                )?;
            }
            Ok(output)
        });
        let output = self
            .validate_llm_call_output(workflow_run_id, step_key, &fingerprint_json, output)
            .await?;
        if !restored || response_only_checkpoint {
            self.persist_parsed_shooting_call(
                workflow_run_id,
                step_key,
                step_order,
                fingerprint_json,
                &raw.response_content,
                &output,
                raw.usage,
                raw.finish_reason,
            )
            .await?;
        }
        Ok(ShootingScriptStageResult { output, restored })
    }
}

fn parse_shooting_script_output<T>(
    raw: &str,
    label: &str,
    call_key: &str,
    upgraded_contract: bool,
) -> Result<T, AppError>
where
    T: DeserializeOwned,
{
    if upgraded_contract {
        let mut value = parse_strict_json_response::<serde_json::Value>(raw, label)?;
        a4n_domain::shooting_script::compat::HashUpgrades::default()
            .upgrade_embedded(&mut value)
            .map_err(AppError::InvalidArgument)?;
        normalize_preserved_sound_arrays(&mut value, call_key);
        return serde_json::from_value::<T>(value).map_err(AppError::Serialization);
    }

    match parse_strict_json_response::<T>(raw, label) {
        Ok(output) => Ok(output),
        Err(error) => {
            let mut value = parse_strict_json_response::<serde_json::Value>(raw, label)?;
            if normalize_preserved_sound_arrays(&mut value, call_key) == 0 {
                return Err(error);
            }
            serde_json::from_value::<T>(value).map_err(|error| {
                AppError::InvalidArgument(format!("{label} 响应字段规范化后仍无效: {error}"))
            })
        }
    }
}

fn normalize_preserved_sound_arrays(value: &mut serde_json::Value, call_key: &str) -> usize {
    let Some(transitions) = value
        .get_mut("transition_contracts")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return 0;
    };

    let mut normalized = 0;
    for (index, transition) in transitions.iter_mut().enumerate() {
        let Some(parts) = transition
            .get("preserved_sound")
            .and_then(serde_json::Value::as_array)
        else {
            continue;
        };
        let Some(parts) = parts
            .iter()
            .map(serde_json::Value::as_str)
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let item_count = parts.len();
        let sound = parts.join("；");
        tracing::warn!(
            target: "a4n::shooting_script",
            call_key,
            transition_index = index,
            item_count,
            "模型将 preserved_sound 返回为字符串数组，已规范为单个描述"
        );
        transition["preserved_sound"] = if sound.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(sound)
        };
        normalized += 1;
    }

    normalized
}

#[cfg(test)]
mod tests {
    use a4n_domain::shooting_script::ShootingTransitionContractV1;
    use serde::Deserialize;
    use serde_json::json;

    use super::{normalize_preserved_sound_arrays, parse_shooting_script_output};

    #[derive(Debug, Deserialize)]
    struct TransitionResponse {
        transition_contracts: Vec<ShootingTransitionContractV1>,
    }

    #[test]
    fn parses_a_sound_string_array_from_the_model_as_one_description() {
        let response = r#"{
            "transition_contracts": [{
                "to_narrative_unit_index": 2,
                "transition_type": "sound_bridge",
                "preserved_action": null,
                "preserved_sound": ["钟声延续", "脚步声渐弱"],
                "preserved_knowledge": [],
                "required_cut_point": null
            }]
        }"#;

        let parsed = parse_shooting_script_output::<TransitionResponse>(
            response,
            "拍摄设计镜头",
            "test-call",
            false,
        )
        .expect("string arrays at preserved_sound are normalized before deserialization");

        assert_eq!(
            parsed.transition_contracts[0].preserved_sound.as_deref(),
            Some("钟声延续；脚步声渐弱")
        );
    }

    #[test]
    fn rejects_non_string_items_in_a_sound_array() {
        let response = r#"{"transition_contracts":[{
            "to_narrative_unit_index":2,
            "transition_type":"sound_bridge",
            "preserved_action":null,
            "preserved_sound":[null],
            "preserved_knowledge":[],
            "required_cut_point":null
        }]}"#;

        let error = parse_shooting_script_output::<TransitionResponse>(
            response,
            "拍摄设计镜头",
            "test-call",
            false,
        )
        .expect_err("non-string array entries still violate the field contract");

        assert!(error.to_string().contains("expected a string"));
    }

    #[test]
    fn normalizes_sound_string_arrays_and_leaves_other_shapes_for_validation() {
        let mut value = json!({
            "transition_contracts": [
                {"preserved_sound": ["钟声延续", "脚步声渐弱"]},
                {"preserved_sound": []},
                {"preserved_sound": "单个声音描述"},
                {"preserved_sound": [null]}
            ],
            "shots": [{"sound": {"effects": ["撞击声"]}}]
        });

        assert_eq!(normalize_preserved_sound_arrays(&mut value, "test-call"), 2);
        assert_eq!(
            value["transition_contracts"][0]["preserved_sound"],
            "钟声延续；脚步声渐弱"
        );
        assert!(value["transition_contracts"][1]["preserved_sound"].is_null());
        assert_eq!(
            value["transition_contracts"][2]["preserved_sound"],
            "单个声音描述"
        );
        assert_eq!(
            value["transition_contracts"][3]["preserved_sound"],
            json!([null])
        );
        assert_eq!(value["shots"][0]["sound"]["effects"], json!(["撞击声"]));
    }
}
