use super::SimHash;
use super::hashing::{hash_token, tokenize};

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
    for (i, &weight) in weights.iter().enumerate() {
        if weight > 0 {
            fingerprint |= 1 << i;
        }
    }

    SimHash(fingerprint)
}
