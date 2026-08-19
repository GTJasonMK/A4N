/// 将文本切成 sparse 检索 token（字符 2–3 gram + 空白分词）。
pub(super) fn tokenize_for_sparse(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    // 去掉空白与控制符后再做 n-gram，避免纯空格产生噪声
    let chars: Vec<char> = text
        .chars()
        .filter(|c| !c.is_control() && !c.is_whitespace())
        .collect();

    // 字符级 bigram / trigram（覆盖中文连续正文）
    if chars.len() >= 2 {
        for window in chars.windows(2) {
            tokens.push(window.iter().collect());
        }
    }
    if chars.len() >= 3 {
        for window in chars.windows(3) {
            tokens.push(window.iter().collect());
        }
    }

    // 空白分词：英文/数字等
    for part in text.split_whitespace() {
        let cleaned: String = part
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if cleaned.chars().count() >= 2 {
            tokens.push(cleaned.to_lowercase());
        }
    }

    tokens
}
