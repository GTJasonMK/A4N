pub(super) fn split_long_paragraph(paragraph: &str) -> Vec<String> {
    const TARGET_MIN_CHARS: usize = 160;
    const HARD_MAX_CHARS: usize = 360;

    let trimmed = paragraph.trim();
    if trimmed.chars().count() <= HARD_MAX_CHARS {
        return vec![trimmed.to_string()];
    }

    let chars: Vec<char> = trimmed.chars().collect();
    let mut paragraphs = Vec::new();
    let mut current = String::new();
    let mut current_len = 0usize;
    let mut index = 0usize;

    while index < chars.len() {
        let ch = chars[index];
        current.push(ch);
        current_len += 1;

        if is_sentence_terminal(ch) {
            while index + 1 < chars.len() && is_closing_punctuation(chars[index + 1]) {
                index += 1;
                current.push(chars[index]);
                current_len += 1;
            }

            if current_len >= TARGET_MIN_CHARS {
                push_non_empty_paragraph(&mut paragraphs, &mut current);
                current_len = 0;
            }
        } else if current_len >= HARD_MAX_CHARS {
            push_non_empty_paragraph(&mut paragraphs, &mut current);
            current_len = 0;
        }

        index += 1;
    }

    push_non_empty_paragraph(&mut paragraphs, &mut current);
    paragraphs
}

fn push_non_empty_paragraph(paragraphs: &mut Vec<String>, current: &mut String) {
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        paragraphs.push(trimmed.to_string());
    }
    current.clear();
}

fn is_sentence_terminal(ch: char) -> bool {
    matches!(ch, '。' | '！' | '？' | '；' | '…' | '!' | '?' | ';')
}

fn is_closing_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '”' | '’' | '"' | '\'' | '」' | '』' | ')' | '）' | ']' | '】' | '》'
    )
}
