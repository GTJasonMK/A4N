#[cfg(test)]
mod tests;

pub fn resolve_epub_href(opf_path: &str, href: &str) -> String {
    let href = href
        .split('#')
        .next()
        .unwrap_or(href)
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string();
    if href.starts_with('/') {
        return normalize_epub_path(href.trim_start_matches('/'));
    }
    match opf_path.rsplit_once('/') {
        Some((base, _)) if !base.is_empty() => normalize_epub_path(&format!("{base}/{href}")),
        _ => normalize_epub_path(&href),
    }
}

pub fn normalize_epub_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for part in normalized.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    parts.join("/").to_lowercase()
}

pub(super) fn is_html_path(path: &str) -> bool {
    let path = path.to_lowercase();
    path.ends_with(".xhtml") || path.ends_with(".html") || path.ends_with(".htm")
}
