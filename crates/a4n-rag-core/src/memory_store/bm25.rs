use std::collections::{HashMap, HashSet};

use dashmap::DashMap;
use parking_lot::RwLock;

use a4n_core::error::AppError;

use super::scoring;
use crate::errors::vector_store_error;

/// BM25 倒排索引和文档长度统计。
pub(super) struct Bm25Index {
    postings: DashMap<(String, String), Vec<(String, usize)>>,
    doc_terms: DashMap<(String, String), Vec<String>>,
    doc_lengths: DashMap<(String, String), usize>,
    doc_len_stats: RwLock<HashMap<String, (usize, usize)>>,
    k1: f32,
    b: f32,
}

impl Bm25Index {
    pub(super) fn new() -> Self {
        Self {
            postings: DashMap::new(),
            doc_terms: DashMap::new(),
            doc_lengths: DashMap::new(),
            doc_len_stats: RwLock::new(HashMap::new()),
            k1: 1.5,
            b: 0.75,
        }
    }

    pub(super) fn postings(&self, collection: &str, term: &str) -> Option<Vec<(String, usize)>> {
        self.postings
            .get(&(collection.to_string(), term.to_string()))
            .map(|entry| entry.clone())
    }

    pub(super) fn score(
        &self,
        doc_id: &str,
        collection: &str,
        tf: usize,
        total_docs: usize,
        doc_freq: usize,
    ) -> Result<f32, AppError> {
        let avg_len = {
            let (total_len, doc_count) = self
                .doc_len_stats
                .read()
                .get(collection)
                .copied()
                .ok_or_else(|| {
                    vector_store_error(format!("集合缺少 BM25 平均长度：{collection}"))
                })?;
            (total_len as f32 / doc_count as f32).max(1.0)
        };
        let doc_len = self
            .doc_lengths
            .get(&(collection.to_string(), doc_id.to_string()))
            .map(|v| *v)
            .ok_or_else(|| {
                vector_store_error(format!(
                    "文档缺少 BM25 长度：collection={collection}, id={doc_id}"
                ))
            })? as f32;

        let idf =
            ((total_docs as f32 - doc_freq as f32 + 0.5) / (doc_freq as f32 + 0.5) + 1.0).ln();
        let numerator = tf as f32 * (self.k1 + 1.0);
        let denominator = tf as f32 + self.k1 * (1.0 - self.b + self.b * doc_len / avg_len);

        Ok(idf * numerator / denominator.max(1e-9))
    }

    pub(super) fn index_document(
        &self,
        collection: &str,
        id: &str,
        content: &str,
    ) -> Result<(), AppError> {
        let tokens = scoring::tokenize(content);
        if tokens.is_empty() {
            return Err(vector_store_error(format!(
                "文档无法生成 BM25 token：collection={collection}, id={id}"
            )));
        }

        let old_len = self.remove_document(collection, id);

        let mut term_counts: HashMap<String, usize> = HashMap::new();
        for token in &tokens {
            *term_counts.entry(token.clone()).or_insert(0) += 1;
        }

        let mut terms = Vec::with_capacity(term_counts.len());
        for (term, tf) in term_counts {
            self.postings
                .entry((collection.to_string(), term.clone()))
                .or_default()
                .push((id.to_string(), tf));
            terms.push(term);
        }

        let doc_len = tokens.len();
        self.doc_lengths
            .insert((collection.to_string(), id.to_string()), doc_len);
        self.doc_terms
            .insert((collection.to_string(), id.to_string()), terms);
        self.adjust_doc_len_stats(collection, old_len.map(|length| (length, 1)), Some(doc_len));
        Ok(())
    }

    pub(super) fn remove_documents(&self, collection: &str, ids: &HashSet<String>) {
        if ids.is_empty() {
            return;
        }
        let mut removed_total = 0;
        let mut removed_count = 0;
        for id in ids {
            if let Some(doc_len) = self.remove_document(collection, id) {
                removed_total += doc_len;
                removed_count += 1;
            }
        }
        self.adjust_doc_len_stats(
            collection,
            (removed_count > 0).then_some((removed_total, removed_count)),
            None,
        );
    }

    pub(super) fn drop_collection(&self, collection: &str) {
        let posting_keys = self
            .postings
            .iter()
            .filter(|entry| entry.key().0 == collection)
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        for key in posting_keys {
            self.postings.remove(&key);
        }

        let doc_keys = self
            .doc_lengths
            .iter()
            .filter(|entry| entry.key().0 == collection)
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        for key in doc_keys {
            self.doc_lengths.remove(&key);
        }

        let doc_term_keys = self
            .doc_terms
            .iter()
            .filter(|entry| entry.key().0 == collection)
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        for key in doc_term_keys {
            self.doc_terms.remove(&key);
        }

        self.doc_len_stats.write().remove(collection);
    }

    fn remove_document(&self, collection: &str, id: &str) -> Option<usize> {
        let key = (collection.to_string(), id.to_string());
        let doc_len = self.doc_lengths.remove(&key).map(|(_, length)| length);
        let terms = self
            .doc_terms
            .remove(&key)
            .map(|(_, terms)| terms)
            .unwrap_or_default();
        for term in terms {
            if let Some(mut entry) = self.postings.get_mut(&(collection.to_string(), term)) {
                entry.retain(|(doc_id, _)| doc_id != id);
            }
        }
        doc_len
    }

    fn adjust_doc_len_stats(
        &self,
        collection: &str,
        removed: Option<(usize, usize)>,
        added: Option<usize>,
    ) {
        let mut stats = self.doc_len_stats.write();
        let (mut total_len, mut doc_count) = stats.get(collection).copied().unwrap_or_default();
        if let Some((removed_total, removed_count)) = removed {
            total_len = total_len.saturating_sub(removed_total);
            doc_count = doc_count.saturating_sub(removed_count);
        }
        if let Some(added_len) = added {
            total_len += added_len;
            doc_count += 1;
        }

        if doc_count == 0 {
            stats.remove(collection);
        } else {
            stats.insert(collection.to_string(), (total_len, doc_count));
        }
    }
}
