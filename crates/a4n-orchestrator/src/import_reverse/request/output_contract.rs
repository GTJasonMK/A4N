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
