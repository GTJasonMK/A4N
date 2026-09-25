use std::time::{SystemTime, UNIX_EPOCH};

use a4n_api_contract::auth::{AuthResult, OAuth2AuthorizeInput, OAuth2CallbackInput};
use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::json;
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

use super::ApiClient;
use crate::{
    client::redact_error_values,
    config::{PendingOAuth, StoredConfig, lock_config, normalize_api_base},
    routing,
};

impl ApiClient {
    pub(crate) async fn oauth2_authorize(
        &mut self,
        provider: String,
        redirect_uri: Option<String>,
    ) -> Result<String> {
        self.reject_environment_auth_mutation("发起 OAuth2 登录")?;
        let state = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let verifier = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let input = OAuth2AuthorizeInput {
            provider: provider.clone(),
            redirect_uri,
            state: state.clone(),
            code_challenge: URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())),
        };
        let value = self
            .send_json(routing::command_request(
                "oauth2_authorize_url",
                json!({ "input": input }),
            )?)
            .await?;
        let authorize: String =
            serde_json::from_value(value).context("OAuth2 授权地址响应合同无效")?;
        let url = Url::parse(&authorize).context("OAuth2 授权地址无效")?;
        let redirect_uri = url
            .query_pairs()
            .find(|(key, _)| key == "redirect_uri")
            .map(|(_, value)| value.into_owned())
            .context("OAuth2 授权地址缺少 redirect_uri")?;
        let _lock = lock_config(&self.config_path)?;
        let mut stored = StoredConfig::load_unlocked(&self.config_path)?;
        if stored
            .server
            .as_deref()
            .map(normalize_api_base)
            .transpose()?
            .is_some_and(|server| server != self.api_base)
        {
            bail!("CLI Server 已变更，拒绝保存旧服务器的 OAuth2 登录请求");
        }
        stored.server = Some(self.api_base.clone());
        stored.pending_oauth = Some(PendingOAuth {
            provider,
            state,
            code_verifier: verifier,
            redirect_uri,
            expires_at: now()? + 600,
        });
        stored.save_unlocked(&self.config_path)?;
        self.stored = stored;
        Ok(authorize)
    }

    pub(crate) async fn oauth2_callback(
        &mut self,
        provider: String,
        callback_url: String,
    ) -> Result<AuthResult> {
        self.reject_environment_auth_mutation("OAuth2 登录")?;
        let callback = Url::parse(callback_url.trim())
            .map_err(|_| anyhow::anyhow!("请通过 stdin 提供浏览器返回的完整回调 URL"))?;
        let input = {
            let _lock = lock_config(&self.config_path)?;
            let mut stored = StoredConfig::load_unlocked(&self.config_path)?;
            let pending = stored
                .pending_oauth
                .as_ref()
                .context("没有进行中的 OAuth2 登录，请先运行 auth oauth-authorize")?;
            if stored
                .server
                .as_deref()
                .map(normalize_api_base)
                .transpose()?
                .as_deref()
                != Some(self.api_base.as_str())
            {
                bail!("CLI Server 已变更，拒绝使用旧的 OAuth2 登录请求");
            }
            let redirect =
                Url::parse(&pending.redirect_uri).context("保存的 OAuth2 回调地址无效")?;
            let states = callback
                .query_pairs()
                .filter(|(key, _)| key == "state")
                .map(|(_, value)| value.into_owned())
                .collect::<Vec<_>>();
            if provider != pending.provider
                || states != [pending.state.clone()]
                || callback.origin() != redirect.origin()
                || callback.path() != redirect.path()
                || callback.fragment().is_some()
            {
                bail!("OAuth2 回调与当前登录请求不匹配");
            }
            if pending.expires_at <= now()? {
                stored.pending_oauth = None;
                stored.save_unlocked(&self.config_path)?;
                bail!("OAuth2 登录请求已过期，请重新授权");
            }
            let codes = callback
                .query_pairs()
                .filter(|(key, _)| key == "code")
                .map(|(_, value)| value.into_owned())
                .collect::<Vec<_>>();
            if callback.query_pairs().any(|(key, _)| key == "error") {
                stored.pending_oauth = None;
                stored.save_unlocked(&self.config_path)?;
                bail!("第三方授权已取消或被拒绝");
            }
            if codes.len() != 1 || codes[0].trim().is_empty() {
                bail!("OAuth2 回调必须包含一个有效的 code");
            }
            let input = OAuth2CallbackInput {
                provider,
                code: codes[0].clone(),
                redirect_uri: pending.redirect_uri.clone(),
                code_verifier: pending.code_verifier.clone(),
            };
            stored.pending_oauth = None;
            stored.save_unlocked(&self.config_path)?;
            self.stored = stored;
            input
        };
        let secrets = [input.code.clone(), input.code_verifier.clone()];
        let value = self
            .send_json(routing::command_request(
                "oauth2_callback",
                json!({ "input": input }),
            )?)
            .await
            .map_err(|error| redact_error_values(error, &secrets))?;
        let auth: AuthResult = serde_json::from_value(value).context("OAuth2 登录响应合同无效")?;
        self.store_tokens(&auth.tokens)?;
        Ok(auth)
    }
}

fn now() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("系统时间早于 Unix epoch")?
        .as_secs())
}
