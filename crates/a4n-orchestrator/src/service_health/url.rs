use a4n_service_manager::ManagedServiceConfig;

pub(super) struct ServiceConfigMetadata {
    pub(super) url: Option<String>,
    pub(super) port: Option<u16>,
    pub(super) health_path: String,
    pub(super) binary: Option<String>,
}

pub(super) fn service_config_metadata(config: &ManagedServiceConfig) -> ServiceConfigMetadata {
    ServiceConfigMetadata {
        url: config.url.as_deref().map(redact_service_url),
        port: config.url.as_deref().and_then(service_url_port),
        health_path: config.health_path.clone(),
        binary: config
            .binary
            .as_ref()
            .map(|path| path.display().to_string()),
    }
}

pub(crate) fn redact_service_url(value: &str) -> String {
    let trimmed = value.trim();
    let without_query = trimmed
        .split(['?', '#'])
        .next()
        .unwrap_or(trimmed)
        .trim_end_matches('/');
    if let Some((scheme, rest)) = without_query.split_once("://") {
        let (authority, path) = split_authority_and_path(rest);
        return format!(
            "{scheme}://{}{}",
            strip_userinfo(authority),
            path.unwrap_or("")
        );
    }
    let (authority, path) = split_authority_and_path(without_query);
    format!("{}{}", strip_userinfo(authority), path.unwrap_or(""))
}

pub(crate) fn service_url_port(value: &str) -> Option<u16> {
    let without_scheme = value
        .trim()
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| value.trim());
    let (authority, _) = split_authority_and_path(without_scheme);
    let authority = strip_userinfo(authority);
    if let Some(rest) = authority.strip_prefix('[') {
        let (_, after_host) = rest.split_once(']')?;
        return after_host.strip_prefix(':')?.parse().ok();
    }
    let (host, port) = authority.rsplit_once(':')?;
    if host.contains(':') {
        return None;
    }
    port.parse().ok()
}

fn split_authority_and_path(value: &str) -> (&str, Option<&str>) {
    match value.find('/') {
        Some(index) => (&value[..index], Some(&value[index..])),
        None => (value, None),
    }
}

fn strip_userinfo(authority: &str) -> &str {
    authority
        .rsplit_once('@')
        .map(|(_, host)| host)
        .unwrap_or(authority)
}
