use a4n_domain::json_contract::{OutputSchemaValidator, parse_strict_json_value};
use a4n_error::AppError;
use serde_json::{Value, json};

use super::super::{ChatMessage, MessageRole};
use super::ChatRequest;

const START: &str = "[A4N_OUTPUT_CONTRACT]";
const END: &str = "[/A4N_OUTPUT_CONTRACT]";

impl ChatRequest {
    /// Called after editable prompt assembly and before context budgeting/encoding.
    pub fn protect_output_contract(&mut self) -> Result<(), AppError> {
        self.protect_contract(None)
    }

    /// Some existing business contracts return arrays, which JSON-object mode cannot represent.
    pub fn with_prompt_output_contract(
        mut self,
        name: &str,
        schema: Value,
    ) -> Result<Self, AppError> {
        self.protect_prompt_output_contract(name, schema)?;
        Ok(self)
    }

    pub fn protect_prompt_output_contract(
        &mut self,
        name: &str,
        schema: Value,
    ) -> Result<(), AppError> {
        self.protect_contract(Some(json!({"name":name,"schema":schema})))
    }

    fn protect_contract(&mut self, required: Option<Value>) -> Result<(), AppError> {
        let (contract, found) = self.resolve_output_contract(required)?;
        let Some(contract) = contract else {
            return Ok(());
        };
        let validator =
            OutputSchemaValidator::new(&contract["schema"]).map_err(AppError::ConfigError)?;
        for message in &self.messages {
            if matches!(message.role, MessageRole::System) {
                validate_declared_examples(&message.content, &validator)?;
            }
        }
        if !found {
            let instruction = format!(
                "{START}\n{contract}\n{END}\n以上为系统输出合同。仅输出符合该 Schema 的完整 JSON；required 字段不可省略，null、空对象和缺少字段含义不同。来源引用与业务规则仍以任务输入为准。"
            );
            if self
                .messages
                .iter()
                .any(|message| matches!(message.role, MessageRole::System))
            {
                // Stage-specific schemas must not rewrite the shared system/source prefix.
                self.messages.push(ChatMessage::system(instruction));
            } else {
                self.messages.insert(0, ChatMessage::system(instruction));
                for breakpoint in &mut self.cache_breakpoints {
                    breakpoint.message_index =
                        breakpoint.message_index.checked_add(1).ok_or_else(|| {
                            AppError::InvalidArgument("输出合同缓存断点索引溢出".into())
                        })?;
                }
            }
            if let Some(trace) = self.trace_context.as_mut() {
                trace.context_plan_json = None;
                trace.context_pack_refs_json = None;
            }
        }
        Ok(())
    }

    fn resolve_output_contract(
        &self,
        required: Option<Value>,
    ) -> Result<(Option<Value>, bool), AppError> {
        let native = self
            .response_format
            .as_ref()
            .and_then(|format| format.json_schema.as_ref())
            .map(|schema| json!({"name":schema.name,"schema":schema.schema}));
        let mut contract = None;
        for candidate in [required, native].into_iter().flatten() {
            merge_contract(&mut contract, candidate)?;
        }
        let mut found = false;
        for message in &self.messages {
            if !matches!(message.role, MessageRole::System) {
                continue;
            }
            let mut remaining = message.content.as_str();
            while let Some((before, after)) = remaining.split_once(START) {
                reject_orphan_end_marker(before)?;
                let (declared, rest) = after
                    .split_once(END)
                    .ok_or_else(|| AppError::InvalidArgument("输出合同块未闭合".into()))?;
                let declared =
                    parse_strict_json_value(declared.trim()).map_err(AppError::Serialization)?;
                merge_contract(&mut contract, declared)?;
                found = true;
                remaining = rest;
            }
            reject_orphan_end_marker(remaining)?;
        }
        Ok((contract, found))
    }

    pub fn output_validator(&self) -> Result<Option<OutputSchemaValidator>, AppError> {
        let (contract, _) = self.resolve_output_contract(None)?;
        contract
            .map(|contract| {
                OutputSchemaValidator::new(&contract["schema"]).map_err(AppError::ConfigError)
            })
            .transpose()
    }
}

fn merge_contract(current: &mut Option<Value>, declared: Value) -> Result<(), AppError> {
    if declared.as_object().is_none_or(|fields| fields.len() != 2)
        || declared["name"]
            .as_str()
            .is_none_or(|name| name.trim().is_empty())
        || declared.get("schema").is_none()
    {
        return Err(AppError::InvalidArgument(
            "输出合同块必须包含非空 name 和 schema".into(),
        ));
    }
    if current.as_ref().is_some_and(|current| current != &declared) {
        return Err(AppError::InvalidArgument(
            "输出合同块与系统合同冲突；请保留创作指令并删除冲突合同块".into(),
        ));
    }
    *current = Some(declared);
    Ok(())
}

fn reject_orphan_end_marker(text: &str) -> Result<(), AppError> {
    if text.contains(END) {
        return Err(AppError::InvalidArgument(
            "输出合同块存在多余的结束标记".into(),
        ));
    }
    Ok(())
}

fn validate_declared_examples(
    text: &str,
    validator: &OutputSchemaValidator,
) -> Result<(), AppError> {
    for label in [
        "只返回严格 JSON 对象：",
        "只返回严格 JSON 数组：",
        "输出严格 JSON：",
        "输出 JSON：",
        "输出示例：",
        "JSON 示例：",
        "返回格式：",
    ] {
        let mut remaining = text;
        while let Some((_, after)) = remaining.split_once(label) {
            remaining = after;
            let candidate = after
                .trim_start()
                .strip_prefix("```json")
                .unwrap_or(after.trim_start())
                .trim_start();
            if !candidate.starts_with(['{', '[']) {
                continue;
            }
            let mut decoder = serde_json::Deserializer::from_str(candidate).into_iter::<Value>();
            let Some(Ok(_)) = decoder.next() else {
                continue;
            };
            let raw = &candidate[..decoder.byte_offset()];
            let example = parse_strict_json_value(raw).map_err(AppError::Serialization)?;
            validator.validate(&example).map_err(|error|AppError::InvalidArgument(format!("提示词声明的 JSON 输出示例与系统合同冲突：{error}；请删除旧示例或按系统合同修正，创作指令可保留")))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> Value {
        json!({"type":"object","required":["id","state"],"properties":{"id":{"type":"integer","minimum":1},"state":{"type":"object"}},"additionalProperties":false})
    }

    #[test]
    fn protected_contract_survives_custom_instructions_snapshot_and_reapplication() {
        let mut request = ChatRequest::system_user("自定义创作方向", "输入正文", None, None)
            .with_json_schema_response_format("scene", schema(), false)
            .unwrap();
        assert!(request.messages[0].content.starts_with("自定义创作方向"));
        let snapshot = request.snapshot_value().unwrap();
        request = ChatRequest::from_snapshot_value(snapshot).unwrap();
        request.protect_output_contract().unwrap();
        assert_eq!(
            request
                .messages
                .iter()
                .map(|message| message.content.matches(START).count())
                .sum::<usize>(),
            1
        );
        assert_eq!(request.messages[0].content, "自定义创作方向");
        let validator = request.output_validator().unwrap().unwrap();
        assert!(validator.validate(&json!({"id":1,"state":{}})).is_ok());
        for invalid in [
            json!({"id":"1","state":{}}),
            json!({"id":1}),
            json!({"id":1,"state":null}),
        ] {
            assert!(validator.validate(&invalid).is_err());
        }
        assert!(request.output_schema_tokens(str::len) > 0);
    }

    #[test]
    fn conflicting_parseable_contracts_are_rejected_before_provider_call() {
        let custom = format!("创作要求\n{START}\n{{\"schema\":{{\"type\":\"string\"}}}}\n{END}");
        assert!(
            ChatRequest::system_user(custom, "input", None, None)
                .with_json_schema_response_format("scene", schema(), false)
                .is_err()
        );
        assert!(
            ChatRequest::system_user(START, "input", None, None)
                .with_json_schema_response_format("scene", schema(), false)
                .is_err()
        );
    }

    #[test]
    fn parseable_output_examples_must_match_the_actual_contract() {
        for system in [
            "只返回严格 JSON 对象：\n{\"id\":\"1\",\"state\":{}}",
            "输出严格 JSON：\n```json\n{\"id\":1,\"state\":{},\"legacy\":true}\n```",
        ] {
            let error = ChatRequest::system_user(system, "input", None, None)
                .with_json_schema_response_format("scene", schema(), false)
                .unwrap_err();
            assert!(error.to_string().contains("输出示例与系统合同冲突"));
        }
        assert!(
            ChatRequest::system_user("输入 JSON：\n{\"id\":\"source\"}", "input", None, None)
                .with_json_schema_response_format("scene", schema(), false)
                .is_ok()
        );
    }

    #[test]
    fn validation_and_dispatch_reject_conflicts_in_restored_native_requests() {
        let mut request = ChatRequest::system_user("system", "input", None, None)
            .with_json_schema_response_format("scene", schema(), false)
            .unwrap();
        request
            .response_format
            .as_mut()
            .unwrap()
            .json_schema
            .as_mut()
            .unwrap()
            .schema = json!({"type":"string"});
        let snapshot = request.snapshot_value().unwrap();
        let mut restored = ChatRequest::from_snapshot_value(snapshot).unwrap();
        assert!(restored.output_validator().is_err());
        assert!(restored.protect_output_contract().is_err());
        assert!(
            restored
                .protect_prompt_output_contract("other", json!({"type":"integer"}))
                .is_err()
        );
    }

    #[test]
    fn prompt_only_contracts_are_rechecked_after_prompt_changes() {
        let mut request = ChatRequest::system_user("system", "input", None, None)
            .with_prompt_output_contract("scene", schema())
            .unwrap();
        let count = request.messages.len();
        request.protect_output_contract().unwrap();
        assert_eq!(request.messages.len(), count);
        assert!(
            request
                .output_validator()
                .unwrap()
                .unwrap()
                .validate(&json!({"id":1}))
                .is_err()
        );
        request.messages[0].content = "输出示例：{\"id\":\"1\",\"state\":{}}".into();
        assert!(request.protect_output_contract().is_err());
    }

    #[test]
    fn incomplete_contracts_and_orphan_markers_are_rejected_consistently() {
        for text in [
            END.to_string(),
            format!("{START}{{\"schema\":true}}{END}"),
            format!("{START}{{\"name\":\" \",\"schema\":true}}{END}"),
            format!("{START}{{\"name\":\"test\"}}{END}"),
            format!("{END}{START}{{\"name\":\"test\",\"schema\":true}}{END}"),
        ] {
            let mut request = ChatRequest::system_user(text, "input", None, None);
            assert!(request.output_validator().is_err());
            assert!(request.protect_output_contract().is_err());
        }
    }
}
