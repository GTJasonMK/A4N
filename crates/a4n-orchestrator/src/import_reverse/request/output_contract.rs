//! 反推请求输出 JSON 合同。

use serde_json::{Value, json};

use self::blueprint::reverse_blueprint_output_contract;
use self::outlines::{reverse_outline_output_contract, reverse_part_outline_output_contract};

mod blueprint;
mod outlines;
#[cfg(test)]
mod tests;

pub(super) fn reverse_output_contract(prompt_name: &str) -> Value {
    match prompt_name {
        "reverse_blueprint" => reverse_blueprint_output_contract(),
        "reverse_outline" => reverse_outline_output_contract(),
        "reverse_part_outline" => reverse_part_outline_output_contract(),
        _ => json!({
            "format": "json_object"
        }),
    }
}
