//! 反推请求输出 JSON 合同。

use a4n_api_contract::import_reverse::ReverseEngineeringTarget;
use serde_json::Value;

use self::blueprint::reverse_blueprint_output_contract;
use self::outlines::{reverse_outline_output_contract, reverse_part_outline_output_contract};

mod blueprint;
mod outlines;
#[cfg(test)]
mod tests;

pub(super) fn reverse_output_contract(target: ReverseEngineeringTarget) -> Value {
    match target {
        ReverseEngineeringTarget::Blueprint => reverse_blueprint_output_contract(),
        ReverseEngineeringTarget::Outline => reverse_outline_output_contract(),
        ReverseEngineeringTarget::PartOutline => reverse_part_outline_output_contract(),
    }
}

pub(super) fn reverse_output_schema(target: ReverseEngineeringTarget) -> Value {
    use serde_json::json;
    let positive = json!({"type":"integer","minimum":1,"maximum":i32::MAX});
    match target {
        ReverseEngineeringTarget::Blueprint => {
            let mut schema = a4n_domain::project::blueprint_output_schema();
            schema["properties"]["total_chapters"] = positive;
            schema["properties"]["needs_part_outlines"] = json!({"type":"boolean"});
            for (array, optional) in [("characters", "arc_stages"), ("relationships", "evolution")]
            {
                schema["properties"][array]["items"]["required"]
                    .as_array_mut()
                    .expect("item required")
                    .retain(|field| field != optional);
            }
            schema
        }
        ReverseEngineeringTarget::Outline => {
            let chapter = a4n_domain::chapter::chapter_outline_output_schema(None);
            let mut item = chapter["properties"]["chapters"]["items"].clone();
            item["properties"]
                .as_object_mut()
                .expect("properties")
                .retain(|key, _| ["title", "summary", "word_count_target"].contains(&key.as_str()));
            item["properties"]["chapter_number"] = positive;
            item["properties"]["word_count_target"] =
                json!({"type":"integer","minimum":1,"maximum":i32::MAX});
            item["required"]
                .as_array_mut()
                .expect("required")
                .push(json!("chapter_number"));
            json!({"type":"object","required":["outlines"],"additionalProperties":false,"properties":{"outlines":{"type":"array","items":item}}})
        }
        ReverseEngineeringTarget::PartOutline => {
            let mut item = a4n_domain::chapter::part_outline_output_schema();
            item["properties"]["key_events"]
                .as_object_mut()
                .expect("key_events")
                .remove("minItems");
            for field in ["part_number", "start_chapter", "end_chapter"] {
                item["properties"][field] = positive.clone();
                item["required"]
                    .as_array_mut()
                    .expect("required")
                    .push(json!(field));
            }
            json!({"type":"object","required":["parts"],"additionalProperties":false,"properties":{"parts":{"type":"array","items":item}}})
        }
    }
}

#[cfg(test)]
#[test]
fn model_contract_matrix() {
    for target in [
        ReverseEngineeringTarget::Blueprint,
        ReverseEngineeringTarget::Outline,
        ReverseEngineeringTarget::PartOutline,
    ] {
        a4n_test_support::model_contract::emit_schema(
            target.prompt_name(),
            reverse_output_schema(target),
        );
    }
}
