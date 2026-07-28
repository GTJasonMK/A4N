use a4n_core::models::{generation::generation_spec, project::source_snapshot_profile};
use serde_json::{Map, Value};
use std::collections::HashMap;

use super::super::super::snapshot_profile_key;
use super::scalars::{optional_params_string, required_params_i64, required_params_string};
use crate::galgame::GALGAME_CG_IMPORTANCE;

pub(in super::super) fn validate_optional_galgame_cg_importance(
    object: &Map<String, Value>,
    spec_id: i64,
) -> Result<(), String> {
    let Some(importance) = optional_params_string(object, "cg_importance", spec_id)? else {
        return Ok(());
    };
    ensure_package!(
        GALGAME_CG_IMPORTANCE.contains(&importance.as_str()),
        "项目族导入包 Galgame 生成意图参数 cg_importance 无效: generation_spec_id={}, cg_importance={}",
        spec_id,
        importance
    );
    Ok(())
}

pub(in super::super) fn validate_generation_spec_canonical_key(
    spec: &generation_spec::Model,
    params: &Map<String, Value>,
    expected_key: &str,
) -> Result<(), String> {
    let canonical_key = required_params_string(params, "canonical_key", spec.id)?;
    ensure_package!(
        canonical_key == expected_key,
        "项目族导入包 Galgame 生成意图 canonical_key 与目标不一致: generation_spec_id={}, canonical_key={}, expected={}",
        spec.id,
        canonical_key,
        expected_key
    );
    Ok(())
}

pub(in super::super) fn validate_params_id_equals(
    spec: &generation_spec::Model,
    object: &Map<String, Value>,
    path: &str,
    expected_id: i64,
) -> Result<(), String> {
    let actual_id = required_params_i64(object, path, spec.id)?;
    ensure_package!(
        actual_id == expected_id,
        "项目族导入包 Galgame 生成意图参数 {path} 与目标不一致: generation_spec_id={}, actual={}, expected={}",
        spec.id,
        actual_id,
        expected_id
    );
    Ok(())
}

pub(in super::super) fn validate_snapshot_character_profile_id(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    profile_id: i64,
    snapshot_profiles: &HashMap<(i64, String), &source_snapshot_profile::Model>,
) -> Result<(), String> {
    ensure_package!(
        snapshot_profiles.contains_key(&(
            source_snapshot_id,
            snapshot_profile_key("character_profile", profile_id),
        )),
        "项目族导入包 Galgame 生成意图参数角色不属于源快照: generation_spec_id={}, profile_id={}, source_snapshot_id={}",
        spec.id,
        profile_id,
        source_snapshot_id
    );
    Ok(())
}
