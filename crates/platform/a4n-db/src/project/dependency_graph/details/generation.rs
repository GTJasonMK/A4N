pub(super) mod inputs;
mod media_assets;
mod prompt_artifacts;
mod specs;

pub(in crate::project::dependency_graph) use self::{
    media_assets::media_asset_detail_records, prompt_artifacts::prompt_artifact_detail_records,
    specs::generation_spec_detail_records,
};

#[cfg(test)]
mod tests;
