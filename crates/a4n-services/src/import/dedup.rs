//! 文本去重——基于 SimHash 的近似重复检测
//!
//! 使用 64 位 SimHash 指纹和汉明距离进行高效近似去重。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// SimHash 64 位指纹
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SimHash(pub u64);

impl SimHash {
    /// 计算与另一个指纹的汉明距离
    pub fn hamming_distance(&self, other: &SimHash) -> u32 {
        (self.0 ^ other.0).count_ones()
    }
}

/// 对文本计算 SimHash 指纹
///
/// 算法流程：
/// 1. 对文本进行 n-gram 分词
/// 2. 对每个 n-gram 计算哈希值
/// 3. 按位累加（1 则 +1，0 则 -1）
/// 4. 最终每位 >0 则为 1，否则为 0
pub fn compute_simhash(text: &str, n: usize) -> SimHash {
    let tokens = tokenize(text, n);
    if tokens.is_empty() {
        return SimHash(0);
    }

    // 64 位累加器
    let mut weights: [i64; 64] = [0; 64];

    for token in &tokens {
        let hash = hash_token(token);
        for (i, weight) in weights.iter_mut().enumerate() {
            if (hash >> i) & 1 == 1 {
                *weight += 1;
            } else {
                *weight -= 1;
            }
        }
    }

    let mut fingerprint: u64 = 0;
    for (i, &w) in weights.iter().enumerate() {
        if w > 0 {
            fingerprint |= 1 << i;
        }
    }

    SimHash(fingerprint)
}

/// 将文本切分为 n-gram 词元
fn tokenize(text: &str, n: usize) -> Vec<String> {
    if text.is_empty() || n == 0 {
        return Vec::new();
    }

    let chars: Vec<char> = text.chars().collect();
    if chars.len() < n {
        return vec![text.to_string()];
    }

    chars
        .windows(n)
        .map(|window| window.iter().collect::<String>())
        .collect()
}

/// 对词元计算哈希值
fn hash_token(token: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_texts() {
        let text = "这是一段测试文本，用于验证 SimHash 算法。";
        let dist = compute_simhash(text, 3).hamming_distance(&compute_simhash(text, 3));
        assert_eq!(dist, 0);
    }

    #[test]
    fn empty_text_has_zero_fingerprint() {
        assert_eq!(compute_simhash("", 3), SimHash(0));
    }

    #[test]
    fn zero_ngram_size_does_not_panic() {
        assert_eq!(compute_simhash("abc", 0), SimHash(0));
    }

    #[test]
    fn test_similar_texts() {
        let a = "这是一段测试文本，用于验证 SimHash 算法。";
        let b = "这是一段测试文本，用于验证 SimHash 算法！";
        let dist = compute_simhash(a, 3).hamming_distance(&compute_simhash(b, 3));
        // 相似文本应有较小汉明距离
        assert!(dist < 10);
    }
}
