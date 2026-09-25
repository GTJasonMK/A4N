use std::{net::IpAddr, process::Stdio, sync::Arc, time::Duration};

use a4n_api_contract::auth::OAuth2BrowserInput;
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    routing::get,
};
use parking_lot::Mutex;
use reqwest::Url;
use serde::Deserialize;
use tokio::{net::TcpListener, process::Command, sync::oneshot};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub(crate) struct OAuthBrowserState(Mutex<Option<(String, CancellationToken)>>);

impl OAuthBrowserState {
    pub(crate) fn cancel_window(&self, label: &str) {
        if let Some((owner, token)) = self.0.lock().as_ref()
            && owner == label
        {
            token.cancel();
        }
    }
}

#[tauri::command]
pub(crate) fn cancel_oauth2_browser_login(
    window: tauri::Window,
    state: tauri::State<'_, OAuthBrowserState>,
) {
    state.cancel_window(window.label());
}

#[tauri::command]
pub(crate) async fn oauth2_browser_login(
    window: tauri::Window,
    state: tauri::State<'_, OAuthBrowserState>,
    input: OAuth2BrowserInput,
) -> Result<String, String> {
    let cancel = CancellationToken::new();
    {
        let mut current = state.0.lock();
        if current.is_some() {
            return Err("已有浏览器授权正在进行，请先取消该次登录".into());
        }
        *current = Some((window.label().to_string(), cancel.clone()));
    }
    let result = receive_authorization(input, cancel, open_browser).await;
    *state.0.lock() = None;
    result
}

struct CallbackState {
    nonce: String,
    redirect_uri: Url,
    sender: Mutex<Option<oneshot::Sender<String>>>,
}

#[derive(Deserialize)]
struct CallbackQuery {
    state: String,
    code: Option<String>,
    error: Option<String>,
}

async fn receive_callback(
    State(state): State<Arc<CallbackState>>,
    Query(query): Query<CallbackQuery>,
) -> (StatusCode, &'static str) {
    if query.state != state.nonce || query.code.is_some() == query.error.is_some() {
        return (StatusCode::BAD_REQUEST, "授权回调与当前登录请求不匹配");
    }
    let mut callback = state.redirect_uri.clone();
    callback
        .query_pairs_mut()
        .append_pair("state", &query.state);
    if let Some(code) = query.code {
        callback.query_pairs_mut().append_pair("code", &code);
    }
    if let Some(error) = query.error {
        callback.query_pairs_mut().append_pair("error", &error);
    }
    let Some(sender) = state.sender.lock().take() else {
        return (StatusCode::GONE, "本次授权回调已处理");
    };
    if sender.send(callback.into()).is_err() {
        return (StatusCode::GONE, "登录请求已关闭");
    }
    (
        StatusCode::OK,
        "授权结果已返回 A4N。你可以关闭此页面，回到应用查看登录结果。",
    )
}

async fn receive_authorization<F, Fut>(
    input: OAuth2BrowserInput,
    cancel: CancellationToken,
    open: F,
) -> Result<String, String>
where
    F: FnOnce(String) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let authorize = Url::parse(&input.authorization_url).map_err(|_| "OAuth2 授权地址无效")?;
    if authorize.scheme() != "https"
        || !matches!(
            (authorize.host_str(), authorize.path()),
            (Some("accounts.google.com"), "/o/oauth2/v2/auth")
                | (Some("github.com"), "/login/oauth/authorize")
        )
        || !authorize.username().is_empty()
        || authorize.password().is_some()
    {
        return Err("OAuth2 授权地址不属于支持的登录服务".into());
    }
    let redirect = Url::parse(&input.redirect_uri).map_err(|_| "OAuth2 回调地址无效")?;
    let host = redirect.host_str().ok_or("OAuth2 回调地址缺少主机")?;
    let address: IpAddr = match host {
        "localhost" | "127.0.0.1" => "127.0.0.1".parse().expect("loopback IPv4"),
        "[::1]" => "::1".parse().expect("loopback IPv6"),
        _ => return Err("桌面 OAuth2 回调必须使用本机 loopback 地址".into()),
    };
    let port = redirect.port().ok_or("桌面 OAuth2 回调必须显式配置端口")?;
    if port == 0
        || redirect.scheme() != "http"
        || redirect.path() != "/auth/callback"
        || redirect.query().is_some()
        || redirect.fragment().is_some()
        || !redirect.username().is_empty()
        || redirect.password().is_some()
    {
        return Err("桌面 OAuth2 回调必须是本机 HTTP /auth/callback".into());
    }
    let params: std::collections::HashMap<_, _> = authorize.query_pairs().collect();
    if params.get("redirect_uri").map(|value| value.as_ref()) != Some(input.redirect_uri.as_str()) {
        return Err("OAuth2 授权地址和监听回调地址不一致".into());
    }
    let nonce = params
        .get("state")
        .filter(|value| !value.is_empty())
        .ok_or("OAuth2 授权缺少 state")?
        .to_string();
    let listener = TcpListener::bind((address, port))
        .await
        .map_err(|error| format!("无法监听 OAuth2 回调端口 {port}: {error}"))?;
    let (sender, receiver) = oneshot::channel();
    let router = Router::new()
        .route("/auth/callback", get(receive_callback))
        .with_state(Arc::new(CallbackState {
            nonce,
            redirect_uri: redirect,
            sender: Mutex::new(Some(sender)),
        }));
    let shutdown = CancellationToken::new();
    let shutdown_signal = shutdown.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal.cancelled_owned())
            .await
    });
    let opening = async {
        open(input.authorization_url).await?;
        std::future::pending::<Result<String, String>>().await
    };
    let result = tokio::select! {
        result = receiver => result.map_err(|_| "OAuth2 回调监听已关闭".into()),
        result = opening => result,
        _ = cancel.cancelled() => Err("已取消浏览器授权".into()),
        _ = tokio::time::sleep(Duration::from_secs(600)) => Err("浏览器授权已超时，请重新登录".into()),
    };
    shutdown.cancel();
    server
        .await
        .map_err(|error| format!("OAuth2 回调监听异常: {error}"))?
        .map_err(|error| format!("OAuth2 回调监听失败: {error}"))?;
    result
}

async fn open_browser(url: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32");
        command.arg("url.dll,FileProtocolHandler");
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut command = Command::new("xdg-open");
    let status = command
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map_err(|error| format!("无法打开系统浏览器: {error}"))?;
    if !status.success() {
        return Err(format!("系统浏览器启动失败: {status}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn callback_receiver_checks_state_and_releases_its_port() {
        let reservation = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = reservation.local_addr().unwrap();
        drop(reservation);
        let redirect_uri = format!("http://{address}/auth/callback");
        let mut authorize = Url::parse("https://accounts.google.com/o/oauth2/v2/auth").unwrap();
        authorize
            .query_pairs_mut()
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("state", "expected-state");
        let callback = redirect_uri.clone();
        let result = receive_authorization(
            OAuth2BrowserInput {
                authorization_url: authorize.to_string(),
                redirect_uri,
            },
            CancellationToken::new(),
            move |_| async move {
                let wrong = reqwest::get(format!("{callback}?state=wrong&code=one-time-code"))
                    .await
                    .unwrap();
                assert_eq!(wrong.status(), StatusCode::BAD_REQUEST);
                let _response = reqwest::get(format!(
                    "{callback}?state=expected-state&code=one-time-code"
                ))
                .await;
                Ok(())
            },
        )
        .await
        .unwrap();
        let url = Url::parse(&result).unwrap();
        assert!(
            url.query_pairs()
                .any(|(key, value)| key == "code" && value == "one-time-code")
        );
        let listener = TcpListener::bind(address).await.unwrap();
        drop(listener);
        let cancel = CancellationToken::new();
        cancel.cancel();
        let error = receive_authorization(
            OAuth2BrowserInput {
                authorization_url: authorize.to_string(),
                redirect_uri: callback_uri(address),
            },
            cancel,
            |_| async { std::future::pending::<Result<(), String>>().await },
        )
        .await
        .unwrap_err();
        assert!(error.contains("已取消"));
        assert!(TcpListener::bind(address).await.is_ok());
    }

    fn callback_uri(address: std::net::SocketAddr) -> String {
        format!("http://{address}/auth/callback")
    }
}
