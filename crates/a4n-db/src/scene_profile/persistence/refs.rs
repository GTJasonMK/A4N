use crate::scene_profile::{SceneProfileDbError, SceneProfileDbResult};
use std::collections::BTreeMap;

pub(super) fn resolve_ref(
    refs: &BTreeMap<String, i64>,
    key: &str,
    field: &str,
) -> SceneProfileDbResult<i64> {
    refs.get(key.trim()).copied().ok_or_else(|| {
        SceneProfileDbError::InvalidArgument(format!("{field} 引用了不存在的场景/出场引用: {key}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_resolution_trims_keys_and_reports_missing_reference() {
        let refs = BTreeMap::from([("scene-a".to_string(), 7)]);

        assert_eq!(resolve_ref(&refs, " scene-a ", "field").unwrap(), 7);
        assert!(matches!(
            resolve_ref(&refs, "missing", "transition.to_scene_ref"),
            Err(SceneProfileDbError::InvalidArgument(message))
                if message == "transition.to_scene_ref 引用了不存在的场景/出场引用: missing"
        ));
    }
}
