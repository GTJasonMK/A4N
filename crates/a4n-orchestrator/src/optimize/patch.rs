mod apply;
mod contracts;
mod response;
mod validation;

pub use apply::{apply_optimization_patches_to_content, optimization_content_hash};
pub use contracts::{OptimizationPatch, OptimizationPatchProposal};
pub(super) use response::{
    content_optimization_patch_schema, parse_content_optimization_patch_response,
    parse_content_optimization_patch_value, restore_content_optimization_patch_proposal,
};
