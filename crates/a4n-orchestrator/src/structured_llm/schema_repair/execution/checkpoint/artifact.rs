use serde_json::Value;

use super::contracts::STRUCTURED_REPAIR_CHECKPOINT_SCHEMA_VERSION;

const RUNTIME_REVALIDATION_SCHEMA_BINDING: &str = "runtime_revalidation";

pub(super) fn compatible_structured_repair_artifact(
    artifacts: Option<&Value>,
    base_call_key: &str,
    suffix: &str,
    base_response_sha256: &str,
    target_schema_sha256: &str,
) -> bool {
    let Some(repair) = artifacts.and_then(|value| value.get("structured_repair")) else {
        return false;
    };
    if repair.get("schema_version").and_then(Value::as_u64)
        != Some(STRUCTURED_REPAIR_CHECKPOINT_SCHEMA_VERSION)
        || repair.get("mode").and_then(Value::as_str) != Some(suffix)
        || repair.get("target_call_key").and_then(Value::as_str) != Some(base_call_key)
        || repair.get("base_response_sha256").and_then(Value::as_str) != Some(base_response_sha256)
    {
        return false;
    }
    match repair.get("target_schema_sha256") {
        Some(Value::String(value)) => value == target_schema_sha256,
        Some(Value::Null) | None => {
            suffix == "schema_repair"
                && repair.get("schema_binding").and_then(Value::as_str)
                    == Some(RUNTIME_REVALIDATION_SCHEMA_BINDING)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn compatible_repair_requires_matching_response_and_schema() {
        let artifacts = json!({
            "structured_repair": {
                "schema_version": 2,
                "mode": "schema_repair",
                "target_call_key": "unit:6",
                "base_response_sha256": "response",
                "target_schema_sha256": "schema"
            }
        });

        assert!(compatible_structured_repair_artifact(
            Some(&artifacts),
            "unit:6",
            "schema_repair",
            "response",
            "schema",
        ));
        assert!(!compatible_structured_repair_artifact(
            Some(&artifacts),
            "unit:6",
            "schema_repair",
            "changed-response",
            "schema",
        ));
        assert!(!compatible_structured_repair_artifact(
            Some(&artifacts),
            "unit:6",
            "schema_repair",
            "response",
            "changed-schema",
        ));
    }

    #[test]
    fn migrated_full_repair_requires_runtime_revalidation_binding() {
        let artifacts = json!({
            "structured_repair": {
                "schema_version": 2,
                "mode": "schema_repair",
                "target_call_key": "unit:6",
                "base_response_sha256": "response",
                "target_schema_sha256": null,
                "schema_binding": "runtime_revalidation"
            }
        });

        assert!(compatible_structured_repair_artifact(
            Some(&artifacts),
            "unit:6",
            "schema_repair",
            "response",
            "current-schema",
        ));
        assert!(!compatible_structured_repair_artifact(
            Some(&artifacts),
            "unit:6",
            "schema_patch",
            "response",
            "current-schema",
        ));
    }

    #[test]
    fn compatible_repair_rejects_missing_or_wrong_contract_identity() {
        assert!(!compatible_structured_repair_artifact(
            None,
            "unit:6",
            "schema_repair",
            "response",
            "schema",
        ));
        let wrong_version = json!({
            "structured_repair": {
                "schema_version": 1,
                "mode": "schema_repair",
                "target_call_key": "unit:6",
                "base_response_sha256": "response",
                "target_schema_sha256": "schema"
            }
        });
        assert!(!compatible_structured_repair_artifact(
            Some(&wrong_version),
            "unit:6",
            "schema_repair",
            "response",
            "schema",
        ));
    }
}
