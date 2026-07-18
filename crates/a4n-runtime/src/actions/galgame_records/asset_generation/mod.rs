use super::media::{
    BindGalgameMediaInput, bind_galgame_media_inner, validate_galgame_media_target,
};
use super::{GalgameProjectInput, project_record_by_id};
use crate::AppState;
use crate::actions::access;
use crate::actions::image_generation::{self, GenerateGalgameImageInput, GeneratedImageSummary};
use crate::actions::portrait_generation::{self, GeneratePortraitInput, PortraitSummary};
use crate::file_utils::canonicalize_data_file_path;
use crate::prompt_renderer::RuntimePromptRenderer;
use a4n_core::models::{
    character::protagonist_profile,
    generation::{generation_spec, prompt_artifact},
};
use a4n_orchestrator::galgame::{
    GALGAME_ENTITY_CHARACTER_PROFILE, GALGAME_ENTITY_LINE, GALGAME_ENTITY_SCENE,
    GALGAME_EXPRESSIONS, GALGAME_TARGET_BACKGROUND, GALGAME_TARGET_CG,
    GALGAME_TARGET_PORTRAIT_EXPRESSION, GalgameAssetPlan,
};
use a4n_orchestrator::galgame::{
    GalgameMediaBindingResult, GalgamePortraitSpecTargetInput, GalgamePromptArtifactUpsertInput,
    GalgameTargetContext,
};
use a4n_orchestrator::media::{
    CreateMediaAssetInput, CreateMediaProvenanceInput, MEDIA_BINDING_ROLE_BACKGROUND,
    MEDIA_BINDING_ROLE_CG, MEDIA_KIND_IMAGE, MEDIA_ORIGIN_GENERATED, MEDIA_STATUS_COMPLETED,
};
use a4n_orchestrator::workflow::{
    WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;
#[cfg(test)]
use serde_json::json;
#[cfg(test)]
use std::collections::BTreeSet;
use tokio_util::sync::CancellationToken;

mod contracts;
mod execution;
mod portrait;
mod portrait_plan;
mod prompt;
mod visual;
mod visual_spec;

use contracts::{
    GALGAME_ASSET_RENDERER_VERSION, GALGAME_BACKGROUND_IMAGE_PROMPT_TEMPLATE,
    GALGAME_CG_IMPORTANCE, GALGAME_EVENT_CG_PROMPT_TEMPLATE, GALGAME_PORTRAIT_VARIANT_STYLE,
    GALGAME_PROMPT_KIND_BACKGROUND, GALGAME_PROMPT_KIND_EVENT_CG,
};
pub use contracts::{
    GalgameAssetGenerationItem, GalgameAssetGenerationSummary, RunGalgameAssetGenerationInput,
};
use execution::mark_galgame_generation_spec_status;
#[cfg(test)]
use execution::validate_run_galgame_asset_generation_input;
pub use execution::{plan_galgame_assets, run_galgame_asset_generation};

use portrait::run_galgame_portrait_spec_generation;
#[cfg(test)]
use portrait::*;
#[cfg(test)]
use prompt::*;
use visual::run_galgame_visual_spec_generation;
#[cfg(test)]
use visual::*;
use visual_spec::GalgameVisualSpecPlan;
#[cfg(test)]
use visual_spec::*;

#[cfg(test)]
mod tests;
