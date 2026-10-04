use std::collections::BTreeMap;

use a4n_error::AppError;

use crate::galgame::{
    GALGAME_TARGET_BACKGROUND, GALGAME_TARGET_CG, GALGAME_TARGET_PORTRAIT_EXPRESSION,
    OrchestratorContext,
};

use super::{
    GalgameAssetPlan, collection::collect_galgame_asset_plan_drafts,
    persistence::assemble_galgame_asset_plan,
};

impl OrchestratorContext {
    pub(crate) async fn read_galgame_asset_plan(
        &self,
        project_id: i64,
    ) -> Result<Option<GalgameAssetPlan>, AppError> {
        let context = self.load_galgame_context(project_id).await?;
        let specs = self
            .galgame_asset_generation_specs(
                project_id,
                &[
                    GALGAME_TARGET_BACKGROUND,
                    GALGAME_TARGET_CG,
                    GALGAME_TARGET_PORTRAIT_EXPRESSION,
                ],
                &[],
                None,
            )
            .await?;
        if specs.is_empty() {
            return Ok(None);
        }
        let read_model = a4n_db::galgame::galgame_asset_planning_read_model(
            self.database.executor(),
            project_id,
        )
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| stale_plan("剧本不存在"))?;
        let drafts = collect_galgame_asset_plan_drafts(&context, read_model)?;
        let mut specs_by_key = BTreeMap::new();
        for spec in specs {
            let snapshot = spec
                .source_snapshot_id
                .ok_or_else(|| stale_plan("生成意图缺少源快照"))?;
            let key = spec
                .params_json
                .get("canonical_key")
                .and_then(|value| value.as_str())
                .ok_or_else(|| stale_plan("生成意图缺少身份"))?
                .to_owned();
            if specs_by_key.insert((snapshot, key), spec).is_some() {
                return Err(stale_plan("生成意图身份重复"));
            }
        }
        let mut items = Vec::with_capacity(drafts.len());
        for draft in drafts {
            let key = (
                draft.record_input.spec.source_snapshot_id,
                draft.canonical_key.clone(),
            );
            let spec = specs_by_key
                .remove(&key)
                .ok_or_else(|| stale_plan("部分素材尚未规划"))?;
            if spec.params_json != draft.record_input.spec.params_json {
                return Err(stale_plan("素材输入已变更"));
            }
            items.push((draft, spec, None));
        }
        if !specs_by_key.is_empty() {
            return Err(stale_plan("包含已不再使用的素材"));
        }
        assemble_galgame_asset_plan(items).map(Some)
    }
}

fn stale_plan(reason: &str) -> AppError {
    AppError::InvalidState {
        current: format!("Galgame 视觉资产计划需要重新规划：{reason}"),
        operation: "读取视觉资产计划".to_owned(),
    }
}
