use super::super::ProjectMutationError;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(super) fn to_json<T>(value: &T) -> Result<Value, ProjectMutationError>
where
    T: Serialize,
{
    serde_json::to_value(value)
        .map_err(|err| ProjectMutationError::Internal(format!("源快照序列化失败: {err}")))
}

pub(super) fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(super) fn hash_json(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).expect("serde_json::Value serialization cannot fail");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

pub(crate) fn shooting_script_source_hash(script: &Value) -> String {
    a4n_domain::shooting_script::source_hash(script)
        .expect("serde_json::Value serialization cannot fail")
}

#[cfg(test)]
mod tests {
    use super::shooting_script_source_hash;
    use serde_json::json;

    #[test]
    fn shooting_source_hash_ignores_snapshot_identity_but_tracks_content() {
        let live = json!({
            "visual_plan_source": {"plan_id": 1, "plan_version": 1, "plan_sha256": "plan"},
            "scenes": [{"title": "第一场"}],
        });
        let mut frozen = live.clone();
        frozen["visual_plan_source"]["plan_id"] = json!(2);
        frozen["visual_plan_source"]["plan_version"] = json!(3);
        assert_eq!(
            shooting_script_source_hash(&live),
            shooting_script_source_hash(&frozen)
        );
        frozen["scenes"][0]["title"] = json!("修改后的场景");
        assert_ne!(
            shooting_script_source_hash(&live),
            shooting_script_source_hash(&frozen)
        );
        frozen = live.clone();
        frozen["visual_plan_source"]["plan_sha256"] = json!("new-plan");
        assert_ne!(
            shooting_script_source_hash(&live),
            shooting_script_source_hash(&frozen)
        );
    }
}
