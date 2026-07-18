use super::json::{SENSITIVE_TEXT_KEYS, redact_sensitive_value};

pub fn redact_sensitive_message(message: impl AsRef<str>) -> String {
    let message = message.as_ref();
    if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(message) {
        redact_sensitive_value(&mut value);
        return value.to_string();
    }

    let message = redact_url_substrings(message);
    redact_sensitive_key_values(&message)
}

fn redact_url_substrings(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        let rest = &input[index..];
        if rest.starts_with("http://") || rest.starts_with("https://") {
            output.push_str("[REDACTED_URL]");
            while let Some((ch, next)) = next_char_at(input, index) {
                index = next;
                if is_url_delimiter(ch) {
                    output.push(ch);
                    break;
                }
            }
            continue;
        }

        let Some((ch, next)) = next_char_at(input, index) else {
            break;
        };
        output.push(ch);
        index = next;
    }
    output
}

fn redact_sensitive_key_values(input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let mut output = String::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        if let Some(key) = SENSITIVE_TEXT_KEYS
            .iter()
            .find(|key| lower[index..].starts_with(**key))
        {
            let after_key = index + key.len();
            let mut delimiter_index = after_key;
            while let Some((ch, next)) = next_char_at(input, delimiter_index) {
                if !ch.is_whitespace() {
                    break;
                }
                delimiter_index = next;
            }

            if let Some((delimiter, after_delimiter)) = next_char_at(input, delimiter_index)
                && matches!(delimiter, '=' | ':')
            {
                let mut value_start = after_delimiter;
                while let Some((ch, next)) = next_char_at(input, value_start) {
                    if !ch.is_whitespace() {
                        break;
                    }
                    value_start = next;
                }

                let mut value_end = value_start;
                while let Some((ch, next)) = next_char_at(input, value_end) {
                    if is_key_value_delimiter(ch) {
                        break;
                    }
                    value_end = next;
                }

                output.push_str(&input[index..value_start]);
                output.push_str("[REDACTED]");
                index = value_end;
                continue;
            }
        }

        let Some((ch, next)) = next_char_at(input, index) else {
            break;
        };
        output.push(ch);
        index = next;
    }
    output
}

fn next_char_at(input: &str, index: usize) -> Option<(char, usize)> {
    input[index..]
        .chars()
        .next()
        .map(|ch| (ch, index + ch.len_utf8()))
}

fn is_url_delimiter(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '"' | '\'' | ',' | ';' | ')' | ']' | '}' | '<' | '>')
}

fn is_key_value_delimiter(ch: char) -> bool {
    matches!(
        ch,
        '"' | '\'' | ',' | ';' | '&' | ')' | ']' | '}' | '<' | '>'
    )
}
