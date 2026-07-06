pub fn split_novel_sentences(text: &str) -> Vec<String> {
    text.replace('\r', "")
        .lines()
        .flat_map(split_paragraph_to_sentences)
        .collect()
}

fn split_paragraph_to_sentences(paragraph: &str) -> Vec<String> {
    let paragraph = paragraph.trim();
    if paragraph.is_empty() {
        return Vec::new();
    }

    let chars = paragraph.chars().collect::<Vec<_>>();
    let mut sentences = Vec::new();
    let mut current = String::new();
    let mut in_chinese_quote = false;
    let mut in_english_quote = false;

    for (index, ch) in chars.iter().copied().enumerate() {
        current.push(ch);

        match ch {
            '「' | '『' | '“' => in_chinese_quote = true,
            '」' | '』' | '”' => in_chinese_quote = false,
            '"' | '\'' => in_english_quote = !in_english_quote,
            _ => {}
        }

        let mut is_end = false;
        if !in_chinese_quote && !in_english_quote {
            if matches!(ch, '。' | '！' | '？') {
                let next = chars.get(index + 1).copied();
                if !next.is_some_and(is_sentence_closer) {
                    is_end = true;
                }
            } else if ch == '；' {
                is_end = true;
            }
        }

        if is_sentence_closer(ch)
            && !in_chinese_quote
            && !in_english_quote
            && current
                .chars()
                .rev()
                .nth(1)
                .is_some_and(|previous| matches!(previous, '。' | '！' | '？'))
        {
            is_end = true;
        }

        if ch == '…'
            && current.ends_with("……")
            && !in_chinese_quote
            && !in_english_quote
            && chars.get(index + 1).is_none_or(|next| {
                !matches!(next, '。' | '！' | '？') && !is_sentence_closer(*next)
            })
        {
            is_end = true;
        }

        if is_end {
            flush_sentence_for_semantic(&mut current, &mut sentences);
        }
    }

    flush_sentence_for_semantic(&mut current, &mut sentences);

    sentences
        .into_iter()
        .flat_map(|sentence| {
            if sentence.chars().count() > 200 {
                split_long_sentence_for_semantic(&sentence, 150)
            } else {
                vec![sentence]
            }
        })
        .collect()
}

fn flush_sentence_for_semantic(current: &mut String, sentences: &mut Vec<String>) {
    let sentence = current.trim();
    if sentence.chars().count() >= 2 {
        sentences.push(sentence.to_string());
    }
    current.clear();
}

fn split_long_sentence_for_semantic(sentence: &str, max_len: usize) -> Vec<String> {
    if sentence.chars().count() <= max_len {
        return vec![sentence.to_string()];
    }

    let chars = sentence.chars().collect::<Vec<_>>();
    let mut result = Vec::new();
    let mut start = 0usize;
    while start < chars.len() {
        let hard_end = (start + max_len).min(chars.len());
        if hard_end >= chars.len() {
            break;
        }

        let cut = chars[start..hard_end]
            .iter()
            .enumerate()
            .rev()
            .find_map(|(offset, ch)| is_soft_boundary(*ch).then_some(start + offset + 1));
        let Some(end) = cut.filter(|end| *end > start) else {
            return vec![sentence.to_string()];
        };
        result.push(
            chars[start..end]
                .iter()
                .collect::<String>()
                .trim()
                .to_string(),
        );
        start = end;
    }

    if start < chars.len() {
        result.push(chars[start..].iter().collect::<String>().trim().to_string());
    }
    if result.len() <= 1 {
        vec![sentence.to_string()]
    } else {
        result
    }
}

fn is_sentence_closer(ch: char) -> bool {
    matches!(ch, '"' | '\'' | '」' | '』' | '”')
}

fn is_soft_boundary(ch: char) -> bool {
    matches!(ch, '，' | ',' | '、' | '：' | ':')
}
