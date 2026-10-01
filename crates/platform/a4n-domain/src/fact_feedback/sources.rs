use super::FactSourceRevision;
use std::collections::BTreeMap;

pub fn pending_fact_sources<'a>(
    current: &'a [FactSourceRevision],
    consolidated: &[FactSourceRevision],
) -> Vec<&'a FactSourceRevision> {
    let consumed: BTreeMap<_, _> = consolidated
        .iter()
        .map(|source| (source.chapter_id, source))
        .collect();
    current
        .iter()
        .filter(|source| consumed.get(&source.chapter_id).copied() != Some(*source))
        .collect()
}
