const ENCODED_PROMPT_FILENAME_PREFIX: &str = "__a4n_prompt_hex__";

pub(super) fn sanitize_filename(name: &str) -> String {
    if is_plain_prompt_filename_name(name) {
        format!("{name}.md")
    } else {
        format!(
            "{ENCODED_PROMPT_FILENAME_PREFIX}{}.md",
            hex_encode(name.as_bytes())
        )
    }
}

fn is_plain_prompt_filename_name(name: &str) -> bool {
    !name.is_empty()
        && !name.ends_with(".md")
        && !name.starts_with(ENCODED_PROMPT_FILENAME_PREFIX)
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
