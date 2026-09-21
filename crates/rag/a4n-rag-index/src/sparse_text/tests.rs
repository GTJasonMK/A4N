use super::{bm25_document, bm25_query, tokenize_for_sparse};

#[test]
fn chinese_ngrams_cover_proper_noun() {
    let tokens = tokenize_for_sparse("林晚施展血契之术");
    assert!(tokens.iter().any(|t| t.contains("血契") || t == "血契"));
    assert!(tokens.iter().any(|t| t.contains("林晚") || t == "林晚"));
}

#[test]
fn bm25_document_and_query_share_indices_for_overlap() {
    let doc = bm25_document("血契之术发动");
    let query = bm25_query("血契");
    assert!(!doc.is_empty());
    assert!(!query.is_empty());
    let overlap = query
        .indices
        .iter()
        .any(|qi| doc.indices.binary_search(qi).is_ok());
    assert!(overlap, "query tokens must hit document sparse indices");
}

#[test]
fn empty_text_yields_empty_sparse() {
    assert!(bm25_document("").is_empty());
    assert!(bm25_query("   ").is_empty());
}

#[test]
fn stable_hash_across_calls() {
    let a = bm25_document("同一段正文");
    let b = bm25_document("同一段正文");
    assert_eq!(a, b);
}
