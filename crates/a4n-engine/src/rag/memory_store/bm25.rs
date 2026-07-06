use std::collections::{HashMap, HashSet};

use dashmap::DashMap;
use parking_lot::RwLock;

use a4n_core::error::AppError;

use super::{scoring, vector_store_error};

/// BM25 倒排索引和文档长度统计。
pub(super) struct Bm25Index {
    postings: DashMap<(String, String), Vec<(String, usize)>>,
    doc_lengths: DashMap<(String, String), usize>,
    avg_doc_len: RwLock<HashMap<String, f32>>,
    k1: f32,
    b: f32,
}

impl Bm25Index {
    pub(super) fn new() -> Self {
        Self {
            postings: DashMap::new(),
            doc_lengths: DashMap::new(),
            avg_doc_len: RwLock::new(HashMap::new()),
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
        let avg_len = self
            .avg_doc_len
            .read()
            .get(collection)
            .copied()
            .ok_or_else(|| vector_store_error(format!("集合缺少 BM25 平均长度：{collection}")))?
            .max(1.0);
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

        self.remove_document(collection, id);

        let mut term_counts: HashMap<String, usize> = HashMap::new();
        for token in &tokens {
            *term_counts.entry(token.clone()).or_insert(0) += 1;
        }

        for (term, tf) in term_counts {
            self.postings
                .entry((collection.to_string(), term))
                .or_default()
                .push((id.to_string(), tf));
        }

        self.doc_lengths
            .insert((collection.to_string(), id.to_string()), tokens.len());
        self.refresh_avg_doc_len(collection);
        Ok(())
    }

    pub(super) fn remove_documents(&self, collection: &str, ids: &HashSet<String>) {
        if ids.is_empty() {
            return;
        }
        for id in ids {
            self.doc_lengths
                .remove(&(collection.to_string(), id.to_string()));
        }

        let terms = self.terms_for_collection(collection);
        for term in terms {
            if let Some(mut entry) = self.postings.get_mut(&(collection.to_string(), term)) {
                entry.retain(|(doc_id, _)| !ids.contains(doc_id));
            }
        }
        self.refresh_avg_doc_len(collection);
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

        self.avg_doc_len.write().remove(collection);
    }

    fn remove_document(&self, collection: &str, id: &str) {
        self.doc_lengths
            .remove(&(collection.to_string(), id.to_string()));

        let terms = self.terms_for_document(collection, id);
        for term in terms {
            if let Some(mut entry) = self.postings.get_mut(&(collection.to_string(), term)) {
                entry.retain(|(doc_id, _)| doc_id != id);
            }
        }
    }

    fn terms_for_document(&self, collection: &str, id: &str) -> Vec<String> {
        self.postings
            .iter()
            .filter(|entry| {
                entry.key().0 == collection && entry.value().iter().any(|(doc_id, _)| doc_id == id)
            })
            .map(|entry| entry.key().1.clone())
            .collect()
    }

    fn terms_for_collection(&self, collection: &str) -> Vec<String> {
        self.postings
            .iter()
            .filter(|entry| entry.key().0 == collection)
            .map(|entry| entry.key().1.clone())
            .collect()
    }

    fn refresh_avg_doc_len(&self, collection: &str) {
        let lengths = self
            .doc_lengths
            .iter()
            .filter(|entry| entry.key().0 == collection)
            .map(|entry| *entry.value())
            .collect::<Vec<_>>();

        if lengths.is_empty() {
            self.avg_doc_len.write().remove(collection);
            return;
        }

        let total: usize = lengths.iter().sum();
        let average = total as f32 / lengths.len() as f32;
        self.avg_doc_len
            .write()
            .insert(collection.to_string(), average);
    }
}
