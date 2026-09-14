use a4n_rag_core::vector_store::EmbeddingRecord;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IngestPayload {
    pub(crate) collection: String,
    pub(crate) records: Vec<EmbeddingRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rag_payload_rejects_unknown_fields() {
        let error = serde_json::from_value::<a4n_rag_sidecar_contract::DropCollectionPayload>(
            serde_json::json!({"collection": "1", "create_if_missing": true}),
        )
        .err()
        .expect("unknown RAG payload fields must fail");
        assert!(
            error
                .to_string()
                .contains("unknown field `create_if_missing`")
        );
    }

    #[test]
    fn rag_ingest_requires_string_content() {
        for record in [
            serde_json::json!({
                "id": "chapter:1:1",
                "embedding": [1.0, 0.0],
                "metadata": {}
            }),
            serde_json::json!({
                "id": "chapter:1:1",
                "embedding": [1.0, 0.0],
                "metadata": {},
                "content": null
            }),
        ] {
            serde_json::from_value::<IngestPayload>(serde_json::json!({
                "collection": "1",
                "records": [record]
            }))
            .err()
            .expect("missing or null content must fail at the sidecar boundary");
        }
    }
}
