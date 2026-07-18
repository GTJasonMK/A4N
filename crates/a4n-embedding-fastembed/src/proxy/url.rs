pub(crate) fn proxy_url(scheme: &str, host: &str, port: u16) -> Option<String> {
    let host = host.trim();
    if host.is_empty() || port == 0 {
        None
    } else {
        Some(format!("{scheme}://{host}:{port}"))
    }
}
