use super::GalgameProjectInput;
use crate::AppState;

use super::{GalgameAssetGenerationSummary, GalgameAssetPlan, RunGalgameAssetGenerationInput};

mod execution;
pub use execution::{plan_galgame_assets, run_galgame_asset_generation};
