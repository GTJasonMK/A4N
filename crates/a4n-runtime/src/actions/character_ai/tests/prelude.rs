//! 角色 AI 测试公共导入（避免污染 production 入口的 orch 透传壳统计）。

pub(super) use super::super::state_projection::fold_attribute_event_tuples;
pub(super) use a4n_domain::character::apply_attribute_to_state;
