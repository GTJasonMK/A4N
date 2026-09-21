use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub(super) fn tokenize(text: &str, n: usize) -> Vec<String> {
    if text.is_empty() || n == 0 {
        return Vec::new();
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < n {
        return vec![text.to_string()];
    }
    chars
        .windows(n)
        .map(|window| window.iter().collect())
        .collect()
}

pub(super) fn hash_token(token: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    hasher.finish()
}
