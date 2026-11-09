use super::*;
use a4n_llm_provider_contract::{ChatMessage, ChatResponse};
use a4n_test_support::model_contract::assert_case;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn response(value: &Value) -> ChatResponse {
    ChatResponse {
        message: ChatMessage::assistant(value.to_string()),
        usage: None,
        finish_reason: Some("stop".into()),
        reasoning_content: None,
    }
}

fn duplicate_object_key(value: &Value) -> Option<String> {
    match value {
        Value::Object(fields) if !fields.is_empty() => {
            let raw = value.to_string();
            let (key, value) = fields.iter().next().unwrap();
            Some(format!(
                "{},{}:{value}}}",
                &raw[..raw.len() - 1],
                serde_json::to_string(key).unwrap()
            ))
        }
        Value::Array(values) => values.iter().enumerate().find_map(|(index, child)| {
            let duplicate = duplicate_object_key(child)?;
            Some(format!(
                "[{}]",
                values
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        if i == index {
                            duplicate.clone()
                        } else {
                            item.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            ))
        }),
        _ => None,
    }
}

#[tokio::test]
#[ignore = "run through model-fast/model-full with exported production contract vectors"]
async fn model_contract_matrix_production_recovery() {
    let path =
        std::env::var("A4N_MODEL_CONTRACT_CASES").expect("model runner must provide case inputs");
    let document: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let call_id = document["call_id"].as_str().unwrap();
    let schema = document["schema"].clone();
    let base = a4n_llm_provider_contract::ChatRequest::system_user(
        "custom creative instruction",
        "original source context",
        None,
        Some(20_000),
    );
    let request = if schema["type"] == "object" {
        base.with_json_schema_response_format(call_id, schema, false)
            .unwrap()
    } else {
        base.with_prompt_output_contract(call_id, schema).unwrap()
    };
    let cases = document["cases"].as_array().unwrap();
    assert!(!cases.is_empty(), "empty mutation matrix must fail");
    for case in cases {
        let counter = Arc::new(AtomicUsize::new(0));
        let seen = counter.clone();
        let invalid = case["value"].clone();
        let result = correct_llm_response(
            &request,
            response(&invalid),
            call_id,
            None,
            |_| Ok(()),
            move |_| async move {
                seen.fetch_add(1, Ordering::SeqCst);
                Ok(response(&invalid))
            },
        )
        .await;
        let expected = case["expected"].as_bool().unwrap();
        let actual =
            json!({"accepted":result.is_ok(),"corrections":counter.load(Ordering::SeqCst)});
        assert_case(
            call_id,
            "runtime-schema",
            case["vector_key"].as_str().unwrap(),
            "recovery-enforcement",
            json!({"accepted":expected,"corrections":if expected {0} else {1}}),
            actual,
            json!({"vector_case_id":case["case_id"],"error":result.err().map(|error|error.to_string().chars().take(500).collect::<String>())}),
        );
    }
    let valid = &document["valid"];
    let invalid = cases
        .iter()
        .find(|case| case["expected"] == false)
        .expect("contract must have negative cases")["value"]
        .clone();
    let corrected = correct_llm_response(
        &request,
        response(&invalid),
        call_id,
        None,
        |_| Ok(()),
        |corrective| async move {
            assert!(
                corrective
                    .messages
                    .iter()
                    .any(|message| message.content.contains("model_output_correction"))
            );
            assert!(
                corrective
                    .messages
                    .iter()
                    .any(|message| message.content.contains("A4N_OUTPUT_CONTRACT"))
            );
            Ok(response(valid))
        },
    )
    .await;
    assert_case(
        call_id,
        "runtime-schema",
        "correction",
        "valid-replacement",
        json!(true),
        json!(corrected.is_ok()),
        json!({"adopted":corrected.as_ref().map(|(response,_)|response.message.content.clone()).ok()}),
    );
    // These must stay raw: parsing into Value first would erase duplicate object keys.
    for (fault, raw) in [
        (
            "duplicate-keys",
            duplicate_object_key(valid).expect("business fixture contains an object"),
        ),
        ("trailing-json", format!("{valid} {{}}")),
        ("markdown-fence", format!("```json\n{valid}\n```")),
        ("malformed-json", "{".to_string()),
    ] {
        for recovers in [false, true] {
            let counter = Arc::new(AtomicUsize::new(0));
            let seen = counter.clone();
            let mut initial = response(valid);
            initial.message.content = raw.clone();
            let repeated = initial.clone();
            let result = correct_llm_response(
                &request,
                initial,
                call_id,
                None,
                |_| Ok(()),
                |corrective| async move {
                    seen.fetch_add(1, Ordering::SeqCst);
                    let feedback: Value =
                        serde_json::from_str(&corrective.messages.last().unwrap().content).unwrap();
                    assert_eq!(feedback["json_rules"]["unique_object_keys"], true);
                    assert!(feedback["json_error_context"]["line"].as_u64().unwrap() > 0);
                    if fault == "duplicate-keys" {
                        assert!(
                            feedback["validation_error"]
                                .as_str()
                                .unwrap()
                                .contains("重复 JSON 字段")
                        );
                    }
                    Ok(if recovers { response(valid) } else { repeated })
                },
            )
            .await;
            assert_case(
                call_id,
                "runtime-json",
                fault,
                if recovers {
                    "corrected-replacement"
                } else {
                    "persistent-rejection"
                },
                json!({"accepted":recovers,"corrections":1}),
                json!({"accepted":result.is_ok(),"corrections":counter.load(Ordering::SeqCst)}),
                json!({"raw_json":raw,"error":result.err().map(|error|error.to_string())}),
            );
        }
    }
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    let result = correct_llm_response(
        &request,
        response(valid),
        call_id,
        Some(&cancelled),
        |_| Ok(()),
        |_| async { panic!("cancelled call must not correct") },
    )
    .await;
    assert_case(
        call_id,
        "runtime-schema",
        "cancellation",
        "no-late-adoption",
        json!(true),
        json!(matches!(result, Err(AppError::Cancelled))),
        json!({"cancelled":true}),
    );
    for reason in [None, Some("length"), Some("content_filter")] {
        let mut initial = response(valid);
        initial.finish_reason = reason.map(str::to_string);
        let result = correct_llm_response(
            &request,
            initial,
            call_id,
            None,
            |_| Ok(()),
            |_| async { panic!("terminal transport failures must not trigger output correction") },
        )
        .await;
        assert_case(
            call_id,
            "runtime-schema",
            reason.unwrap_or("missing-finish"),
            "terminal-before-validation",
            json!(true),
            json!(result.is_err()),
            json!({"reason":reason}),
        );
    }
}
