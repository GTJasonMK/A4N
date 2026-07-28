use super::GalgameProjectInput;
use crate::AppState;

pub(super) type GalgameAssetPlan = a4n_orchestrator::galgame::GalgameAssetPlan;

mod execution;
pub use a4n_orchestrator::galgame::{
    GalgameAssetGenerationApplicationInput as RunGalgameAssetGenerationInput,
    GalgameAssetGenerationApplicationItem as GalgameAssetGenerationItem,
    GalgameAssetGenerationApplicationSummary as GalgameAssetGenerationSummary,
};
pub use execution::{plan_galgame_assets, run_galgame_asset_generation};
