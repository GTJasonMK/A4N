mod contracts;
mod download;
mod provider;

pub(crate) use download::{
    download_embedding_model_events, embedding_model_operation, test_embedding_config_via_sidecar,
};
pub(crate) use provider::create_embedding_provider;

#[cfg(test)]
mod tests;
