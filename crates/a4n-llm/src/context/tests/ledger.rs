use super::super::*;
use serde_json::json;

#[test]
fn ledger_preserves_pack_order_and_hashes_config_identity() {
    let mut ledger = LlmContextLedger::new();
    ledger
        .add_pack(LlmContextLedgerPackInput {
            scope_kind: ("chapter").into(),
            pack_kind: ("first").into(),
            source_refs_json: None,
            config_identity_json: json!({"version": 1}),
            required: true,
            stable_prefix: true,
            variants: vec![LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({"body": "first"}),
            )],
        })
        .expect("first pack");
    ledger
        .add_pack(LlmContextLedgerPackInput {
            scope_kind: ("chapter").into(),
            pack_kind: ("second").into(),
            source_refs_json: None,
            config_identity_json: json!({"version": 1}),
            required: false,
            stable_prefix: true,
            variants: vec![LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({"body": "second"}),
            )],
        })
        .expect("second pack");

    let manifest = ledger.into_manifest("dynamic", None);

    assert_eq!(manifest.packs[0].pack_kind, "first");
    assert_eq!(manifest.packs[1].pack_kind, "second");
    assert_ne!(manifest.packs[0].config_hash, manifest.packs[1].config_hash);
}

#[test]
fn ledger_rejects_duplicate_compression_levels() {
    let mut ledger = LlmContextLedger::new();
    let err = ledger
        .add_pack(LlmContextLedgerPackInput {
            scope_kind: ("chapter").into(),
            pack_kind: ("source").into(),
            source_refs_json: None,
            config_identity_json: json!({"version": 1}),
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, json!({"a": 1})),
                LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, json!({"a": 2})),
            ],
        })
        .expect_err("duplicate level should fail");

    assert!(err.to_string().contains("重复 compression_level"));
}
