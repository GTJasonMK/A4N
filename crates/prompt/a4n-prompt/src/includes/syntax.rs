use regex::Regex;

/// 匹配 `<!-- @include relative/path -->`，路径可被单/双引号包裹。
pub(super) fn include_re() -> &'static Regex {
    use std::sync::OnceLock;

    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<!--\s*@include\s+(.+?)\s*-->").unwrap())
}

pub(super) fn unquote(raw: &str) -> &str {
    let value = raw.trim();
    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}
