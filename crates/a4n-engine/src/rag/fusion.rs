//! 检索结果融合算法
//!
//! 当前实现 RRF（Reciprocal Rank Fusion）：将多路检索的排序合并为单一排序，
//! 不要求各路分数同分布，仅依赖排名位置。

use super::vector_store::SearchResult;
use std::collections::HashMap;

/// RRF 默认平滑常数
pub const RRF_DEFAULT_K: usize = 60;

/// 通过 RRF 融合向量检索与 BM25 检索结果
///
/// - `vector_results`: 向量语义检索结果，按相似度倒序
/// - `bm25_results`: BM25 全文检索结果，按相关性倒序
/// - `top_k`: 返回前 K 条
/// - `k`: RRF 平滑常数（推荐 60）
///
/// 输出按 `vector_score + bm25_score` 倒序，并保留原始内容与元数据。
/// 当同一 id 同时命中两路时，content/metadata 优先取向量路。
pub fn rrf_fuse(
    vector_results: &[SearchResult],
    bm25_results: &[SearchResult],
    top_k: usize,
    k: usize,
) -> Vec<SearchResult> {
    let mut accum: HashMap<String, FusedEntry> = HashMap::new();

    for (rank, r) in vector_results.iter().enumerate() {
        let score = 1.0 / (k as f32 + rank as f32 + 1.0);
        let entry = accum
            .entry(r.id.clone())
            .or_insert_with(|| FusedEntry::from_result(r));
        entry.vector_score = score;
        entry.content = r.content.clone();
        entry.metadata = r.metadata.clone();
    }

    for (rank, r) in bm25_results.iter().enumerate() {
        let score = 1.0 / (k as f32 + rank as f32 + 1.0);
        let entry = accum
            .entry(r.id.clone())
            .or_insert_with(|| FusedEntry::from_result(r));
        entry.bm25_score = score;
    }

    let mut fused: Vec<FusedEntry> = accum.into_values().collect();
    fused.sort_by(|a, b| {
        let sa = a.vector_score + a.bm25_score;
        let sb = b.vector_score + b.bm25_score;
        sb.total_cmp(&sa)
    });

    fused
        .into_iter()
        .take(top_k)
        .map(|e| SearchResult {
            id: e.id,
            content: e.content,
            score: e.vector_score + e.bm25_score,
            vector_score: Some(e.vector_score),
            bm25_score: Some(e.bm25_score),
            metadata: e.metadata,
        })
        .collect()
}

struct FusedEntry {
    id: String,
    content: String,
    metadata: serde_json::Value,
    vector_score: f32,
    bm25_score: f32,
}

impl FusedEntry {
    fn from_result(r: &SearchResult) -> Self {
        Self {
            id: r.id.clone(),
            content: r.content.clone(),
            metadata: r.metadata.clone(),
            vector_score: 0.0,
            bm25_score: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(id: &str, score: f32) -> SearchResult {
        SearchResult {
            id: id.to_string(),
            content: format!("c-{id}"),
            score,
            vector_score: Some(score),
            bm25_score: None,
            metadata: serde_json::json!({"id": id}),
        }
    }

    #[test]
    fn fuses_disjoint_ranks() {
        let v = vec![make("a", 0.9), make("b", 0.8)];
        let b = vec![make("c", 0.7), make("a", 0.6)];
        let r = rrf_fuse(&v, &b, 3, RRF_DEFAULT_K);
        assert_eq!(r.len(), 3);
        // 命中两路的 a 应当排第一
        assert_eq!(r[0].id, "a");
    }

    #[test]
    fn respects_top_k() {
        let v: Vec<SearchResult> = (0..10).map(|i| make(&format!("v{i}"), 1.0)).collect();
        let b: Vec<SearchResult> = (0..10).map(|i| make(&format!("b{i}"), 1.0)).collect();
        let r = rrf_fuse(&v, &b, 5, RRF_DEFAULT_K);
        assert_eq!(r.len(), 5);
    }
}
