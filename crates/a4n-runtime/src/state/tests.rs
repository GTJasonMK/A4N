use a4n_rag_core::vector_store::VectorStore;

use super::profile::RuntimeProfile;
use super::rag::DisabledVectorStore;

#[test]
fn runtime_profile_parse_accepts_implemented_profiles() {
    assert_eq!(RuntimeProfile::parse("full").unwrap(), RuntimeProfile::Full);
    assert_eq!(
        RuntimeProfile::parse(" minimal ").unwrap(),
        RuntimeProfile::Minimal
    );
    assert_eq!(
        RuntimeProfile::parse("sidecar").unwrap(),
        RuntimeProfile::Sidecar
    );

    let err = RuntimeProfile::parse("unknown").expect_err("unknown profile should fail");
    assert!(err.to_string().contains("可选值: full, minimal, sidecar"));
}

#[test]
fn runtime_profile_default_matches_compiled_capabilities() {
    #[cfg(feature = "embedded-capabilities")]
    assert_eq!(RuntimeProfile::default_for_build(), RuntimeProfile::Full);

    #[cfg(not(feature = "embedded-capabilities"))]
    assert_eq!(RuntimeProfile::default_for_build(), RuntimeProfile::Sidecar);

    let error = RuntimeProfile::parse(" ").expect_err("blank profile must fail explicitly");
    assert!(error.to_string().contains("A4N_RUNTIME_PROFILE 不能为空"));
}

#[tokio::test]
async fn disabled_vector_store_keeps_reads_explicit_and_drop_idempotent() {
    let store = DisabledVectorStore::new(RuntimeProfile::Minimal);

    let err = store
        .metadata_by_ids("missing", &["missing".to_string()])
        .await
        .expect_err("disabled RAG must fail visibly");

    assert!(err.to_string().contains("RAG 能力未启用"));
    assert!(err.to_string().contains("runtime profile=minimal"));
    store
        .drop_collection("missing")
        .await
        .expect("disabled RAG has no collection to delete");
}

#[cfg(not(all(
    feature = "embedded-prompt-git",
    feature = "embedded-llm-provider",
    feature = "local-embedding",
    feature = "embedded-rag-index",
    feature = "embedded-media-providers"
)))]
#[test]
fn full_profile_requires_all_embedded_capabilities_before_startup() {
    let error = RuntimeProfile::Full
        .validate_build_capabilities()
        .expect_err("full profile must reject an incomplete embedded build");
    let message = error.to_string();

    assert!(message.contains("full runtime profile 要求完整 embedded capabilities"));
    assert!(message.contains("--features embedded-capabilities"));
}
