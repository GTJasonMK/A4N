mod factory;
mod sidecar;

pub(crate) use factory::create_embedding_provider;
pub(in crate::sidecar_clients::embedding) use sidecar::SidecarEmbeddingProvider;
