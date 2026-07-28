use super::tokenize_for_sparse;
use std::collections::HashMap;

/// Sparse 向量：索引与权重一一对应，索引已排序去重。
#[derive(Debug, Clone, PartialEq)]
pub struct SparseEmbedding {
    pub indices: Vec<u32>,
    pub values: Vec<f32>,
}

impl SparseEmbedding {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// BM25 参数（与 Qdrant lib/bm25 默认一致）。
#[derive(Debug, Clone, Copy)]
pub struct Bm25Params {
    pub k1: f64,
    pub b: f64,
    pub avg_doc_len: f64,
}

impl Default for Bm25Params {
    fn default() -> Self {
        Self {
            k1: 1.2,
            b: 0.75,
            avg_doc_len: 256.0,
        }
    }
}

/// 稳定 token → u32（FNV-1a 32-bit）。
fn token_index(token: &str) -> u32 {
    const FNV_OFFSET: u32 = 0x811c_9dc5;
    const FNV_PRIME: u32 = 0x0100_0193;
    let mut hash = FNV_OFFSET;
    for byte in token.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn term_frequencies(tokens: &[String]) -> HashMap<u32, u32> {
    let mut tf = HashMap::new();
    for token in tokens {
        *tf.entry(token_index(token)).or_insert(0) += 1;
    }
    tf
}

/// 文档端 BM25 权重（无语料 IDF 时用恒等 IDF=1，依赖 Qdrant sparse IDF modifier 可选增强）。
pub fn bm25_document(text: &str) -> SparseEmbedding {
    bm25_document_with_params(text, &Bm25Params::default())
}

fn bm25_document_with_params(text: &str, params: &Bm25Params) -> SparseEmbedding {
    let tokens = tokenize_for_sparse(text);
    if tokens.is_empty() {
        return SparseEmbedding {
            indices: Vec::new(),
            values: Vec::new(),
        };
    }
    let doc_len = tokens.len() as f64;
    let tf_map = term_frequencies(&tokens);
    let mut pairs: Vec<(u32, f32)> = tf_map
        .into_iter()
        .map(|(idx, tf)| {
            let tf = f64::from(tf);
            let denom = tf + params.k1 * (1.0 - params.b + params.b * doc_len / params.avg_doc_len);
            let weight = ((params.k1 + 1.0) * tf / denom) as f32;
            (idx, weight.max(0.0))
        })
        .filter(|(_, w)| *w > 0.0)
        .collect();
    pairs.sort_by_key(|(i, _)| *i);
    pairs.dedup_by_key(|(i, _)| *i);
    let (indices, values) = pairs.into_iter().unzip();
    SparseEmbedding { indices, values }
}

/// 查询端：出现过的 term 等权 1.0（经典 query TF=1）。
pub fn bm25_query(text: &str) -> SparseEmbedding {
    let tokens = tokenize_for_sparse(text);
    if tokens.is_empty() {
        return SparseEmbedding {
            indices: Vec::new(),
            values: Vec::new(),
        };
    }
    let mut indices: Vec<u32> = tokens.iter().map(|t| token_index(t)).collect();
    indices.sort_unstable();
    indices.dedup();
    let values = vec![1.0_f32; indices.len()];
    SparseEmbedding { indices, values }
}
