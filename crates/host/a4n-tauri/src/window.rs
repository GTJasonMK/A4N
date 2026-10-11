use a4n_config::app_config::AppConfig;
use a4n_storage::paths::AppPaths;
use tauri::Manager;
use url::Url;

const MAIN_WINDOW_LABEL: &str = "main";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StandaloneWindowSpec {
    label: &'static str,
    route: &'static str,
    title: &'static str,
}

const STANDALONE_WINDOWS: &[StandaloneWindowSpec] = &[
    StandaloneWindowSpec {
        label: "test-center",
        route: "/admin/test-center",
        title: "A4N Test Center",
    },
    StandaloneWindowSpec {
        label: "agent-center",
        route: "/agents",
        title: "A4N Agent Workbench",
    },
];

pub(crate) fn create_main_window(
    app: &mut tauri::App,
    config: &AppConfig,
    api_base_url: Option<&str>,
) -> tauri::Result<()> {
    let webview_data_dir = AppPaths::new(
        config.data_dir.clone(),
        config.cache_dir.clone(),
        config.temp_dir.clone(),
        config.log_dir.clone(),
    )
    .webview_dir();
    std::fs::create_dir_all(&webview_data_dir)?;
    let main_window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MAIN_WINDOW_LABEL)
        .ok_or(tauri::Error::WindowNotFound)?;
    let standalone_window_configs = STANDALONE_WINDOWS
        .iter()
        .map(|spec| {
            let mut config = (*main_window_config).clone();
            config.label = spec.label.to_string();
            config.title = spec.title.to_string();
            config.create = true;
            (*spec, config)
        })
        .collect::<Vec<_>>();

    let builder = tauri::WebviewWindowBuilder::from_config(app.handle(), main_window_config)?
        .data_directory(webview_data_dir.clone())
        .on_new_window({
            let app_handle = app.handle().clone();
            let webview_data_dir = webview_data_dir.clone();
            let api_base_url = api_base_url.map(str::to_string);
            let main_window_config = standalone_window_configs;
            move |url, features| {
                let Some(main_window) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) else {
                    tracing::warn!(%url, "拒绝独立窗口请求：主窗口不存在");
                    return tauri::webview::NewWindowResponse::Deny;
                };
                let main_url = match main_window.url() {
                    Ok(url) => url,
                    Err(error) => {
                        tracing::warn!(%error, "拒绝独立窗口请求：无法读取主窗口 URL");
                        return tauri::webview::NewWindowResponse::Deny;
                    }
                };
                let Some((spec, window_config)) = main_window_config
                    .iter()
                    .find(|(spec, _)| is_standalone_url(&url, &main_url, spec.route))
                else {
                    tracing::warn!(%url, "拒绝未登记的独立窗口路由");
                    return tauri::webview::NewWindowResponse::Deny;
                };

                if let Some(window) = app_handle.get_webview_window(spec.label) {
                    if let Err(error) = window.show() {
                        tracing::warn!(%error, label = spec.label, "显示已有独立窗口失败");
                    }
                    if let Err(error) = window.set_focus() {
                        tracing::warn!(%error, label = spec.label, "聚焦已有独立窗口失败");
                    }
                    return tauri::webview::NewWindowResponse::Deny;
                }

                let initialization_script = match initialization_script(
                    Some(&url),
                    api_base_url.as_deref(),
                    spec.label,
                ) {
                    Ok(script) => script,
                    Err(error) => {
                        tracing::error!(%error, label = spec.label, "生成独立窗口初始化脚本失败");
                        return tauri::webview::NewWindowResponse::Deny;
                    }
                };
                let builder =
                    match tauri::WebviewWindowBuilder::from_config(&app_handle, window_config) {
                        Ok(builder) => builder
                            .window_features(features)
                            .data_directory(webview_data_dir.clone())
                            .initialization_script(initialization_script),
                        Err(error) => {
                            tracing::error!(%error, label = spec.label, "创建独立窗口配置失败");
                            return tauri::webview::NewWindowResponse::Deny;
                        }
                    };
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                let builder = builder.incognito(true);

                match builder.visible(true).focused(true).build() {
                    Ok(window) => tauri::webview::NewWindowResponse::Create { window },
                    Err(error) => {
                        tracing::error!(%error, label = spec.label, "创建独立窗口失败");
                        tauri::webview::NewWindowResponse::Deny
                    }
                }
            }
        });
    let script = initialization_script(None, api_base_url, MAIN_WINDOW_LABEL)?;
    let builder = if script.is_empty() {
        builder
    } else {
        builder.initialization_script(script)
    };
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    let builder = builder.incognito(true);
    let window = builder.visible(true).focused(true).build()?;
    window.show()?;
    window.set_focus()?;
    Ok(())
}

fn is_standalone_url(url: &Url, main_url: &Url, route: &str) -> bool {
    let path = url.path().trim_end_matches('/');
    same_origin(url, main_url)
        && url.username().is_empty()
        && url.password().is_none()
        && (path == route || path.ends_with(route))
}

fn same_origin(url: &Url, main_url: &Url) -> bool {
    let url_origin = url.origin();
    let main_origin = main_url.origin();
    if url_origin.is_tuple() && main_origin.is_tuple() {
        return url_origin == main_origin;
    }

    url.scheme() == main_url.scheme()
        && url.host_str().is_some()
        && url.host_str() == main_url.host_str()
        && url.port() == main_url.port()
}

fn initialization_script(
    route: Option<&Url>,
    api_base_url: Option<&str>,
    window_label: &str,
) -> Result<String, serde_json::Error> {
    let mut scripts = Vec::with_capacity(3);
    scripts.push(format!(
        "Object.defineProperty(window, '__A4N_WINDOW_LABEL__', {{ value: {}, configurable: false }});",
        serde_json::to_string(window_label)?
    ));
    if let Some(route) = route {
        let route = serde_json::to_string(route.as_str())?;
        scripts.push(format!(
            "if (window === window.top && (window.location.pathname === '/' || window.location.pathname.endsWith('/index.html'))) {{ window.history.replaceState(null, \"\", {route}); }}"
        ));
    }
    if let Some(api_base_url) = api_base_url {
        scripts.push(format!(
            "Object.defineProperty(window, '__A4N_API_BASE_URL__', {{ value: {} }});",
            serde_json::to_string(api_base_url)?
        ));
    }
    Ok(scripts.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::{is_standalone_url, same_origin};
    use url::Url;

    #[test]
    fn allows_registered_standalone_routes_on_the_main_window_origin() {
        let main_url = Url::parse("http://127.0.0.1:5173/").unwrap();
        let test_center = Url::parse("http://127.0.0.1:5173/admin/test-center").unwrap();
        let agent = Url::parse("http://127.0.0.1:5173/agents?thread=main#activity").unwrap();

        assert!(is_standalone_url(
            &test_center,
            &main_url,
            "/admin/test-center"
        ));
        assert!(is_standalone_url(&agent, &main_url, "/agents"));
    }

    #[test]
    fn allows_standalone_routes_with_a_router_base_path() {
        let main_url = Url::parse("tauri://localhost/index.html").unwrap();
        let test_center = Url::parse("tauri://localhost/app/admin/test-center/").unwrap();
        let agent = Url::parse("tauri://localhost/app/agents/").unwrap();

        assert!(is_standalone_url(
            &test_center,
            &main_url,
            "/admin/test-center"
        ));
        assert!(is_standalone_url(&agent, &main_url, "/agents"));
    }

    #[test]
    fn rejects_other_origins_and_unregistered_routes() {
        let main_url = Url::parse("http://127.0.0.1:5173/").unwrap();
        let external_url = Url::parse("https://example.com/admin/test-center").unwrap();
        let other_route = Url::parse("http://127.0.0.1:5173/admin/users").unwrap();
        let custom_scheme_other_host = Url::parse("tauri://example.com/admin/test-center").unwrap();

        assert!(!is_standalone_url(
            &external_url,
            &main_url,
            "/admin/test-center"
        ));
        assert!(!is_standalone_url(
            &other_route,
            &main_url,
            "/admin/test-center"
        ));
        assert!(!same_origin(
            &custom_scheme_other_host,
            &Url::parse("tauri://localhost/index.html").unwrap()
        ));
    }

    #[test]
    fn rejects_credentials_even_when_origin_matches() {
        let main_url = Url::parse("http://127.0.0.1:5173/").unwrap();
        let requested_url =
            Url::parse("http://user:secret@127.0.0.1:5173/admin/test-center").unwrap();

        assert!(!is_standalone_url(
            &requested_url,
            &main_url,
            "/admin/test-center"
        ));
    }
}
