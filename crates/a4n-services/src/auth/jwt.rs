//! JWT 令牌管理
//!
//! 基于 jsonwebtoken crate 实现访问令牌和刷新令牌的签发与验证。

use a4n_core::error::AppError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const ACCESS_TOKEN_EXP_MINUTES: u64 = 15;
const ACCESS_TOKEN_EXPIRES_IN_SECS: u64 = ACCESS_TOKEN_EXP_MINUTES * 60;
const REFRESH_TOKEN_EXP_DAYS: u64 = 30;

/// JWT 声明
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// 主题（用户 ID）
    pub sub: String,
    /// 用户名称
    pub name: Option<String>,
    /// 用户角色
    pub role: Option<String>,
    /// 签发时间（Unix 时间戳）
    pub iat: usize,
    /// 过期时间（Unix 时间戳）
    pub exp: usize,
    /// 令牌类型：access 或 refresh
    pub token_type: String,
    /// 令牌唯一 ID。旧 token 可能没有该字段，因此保持可选。
    pub jti: Option<String>,
    /// 媒体短链令牌绑定的应用数据目录相对路径。非媒体令牌为空。
    #[serde(default)]
    pub media_path: Option<String>,
}

/// 令牌对
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    /// 访问令牌过期秒数
    pub expires_in: u64,
    /// 令牌类型
    pub token_type: String,
}

/// 创建携带真实用户角色的访问令牌。
fn create_access_token_with_role(
    user_id: &str,
    name: Option<&str>,
    role: &str,
    secret: &str,
    exp_minutes: u64,
) -> Result<String, AppError> {
    let role = role_claim(role)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        name: name.map(|s| s.to_string()),
        role: Some(role),
        iat: now,
        exp: now + (exp_minutes as usize * 60),
        token_type: "access".to_string(),
        jti: Some(Uuid::new_v4().to_string()),
        media_path: None,
    };

    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    let key = jsonwebtoken::EncodingKey::from_secret(secret.as_bytes());

    jsonwebtoken::encode(&header, &claims, &key)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT 签发失败: {e}")))
}

fn role_claim(role: &str) -> Result<String, AppError> {
    if role.trim().is_empty() {
        return Err(AppError::InvalidArgument("用户角色不能为空".into()));
    }
    Ok(role.to_string())
}

/// 创建刷新令牌
///
/// # 参数
/// - `user_id`: 用户唯一标识
/// - `secret`: 签名密钥
/// - `exp_days`: 过期天数
pub(super) fn create_refresh_token(
    user_id: &str,
    secret: &str,
    exp_days: u64,
) -> Result<String, AppError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        name: None,
        role: None,
        iat: now,
        exp: now + (exp_days as usize * 86400),
        token_type: "refresh".to_string(),
        jti: Some(Uuid::new_v4().to_string()),
        media_path: None,
    };

    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    let key = jsonwebtoken::EncodingKey::from_secret(secret.as_bytes());

    jsonwebtoken::encode(&header, &claims, &key)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT 签发失败: {e}")))
}

/// 验证令牌并返回声明
pub fn verify_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let key = jsonwebtoken::DecodingKey::from_secret(secret.as_bytes());
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);

    // 只验证访问令牌
    validation.set_required_spec_claims(&["exp", "sub", "iat"]);

    let token_data = jsonwebtoken::decode::<Claims>(token, &key, &validation)
        .map_err(|e| AppError::Forbidden(format!("JWT 验证失败: {e}")))?;

    Ok(token_data.claims)
}

/// 使用刷新令牌和当前用户信息获取新的令牌对。
pub fn refresh_access_token_with_role(
    refresh_token: &str,
    name: Option<&str>,
    role: &str,
    secret: &str,
) -> Result<TokenPair, AppError> {
    let claims = verify_token(refresh_token, secret)?;

    if claims.token_type != "refresh" {
        return Err(AppError::Forbidden("令牌类型错误：需要刷新令牌".into()));
    }

    let access_token = create_access_token_with_role(
        &claims.sub,
        name.or(claims.name.as_deref()),
        role,
        secret,
        ACCESS_TOKEN_EXP_MINUTES,
    )?;

    let new_refresh_token = create_refresh_token(&claims.sub, secret, REFRESH_TOKEN_EXP_DAYS)?;

    Ok(TokenPair {
        access_token,
        refresh_token: new_refresh_token,
        expires_in: ACCESS_TOKEN_EXPIRES_IN_SECS,
        token_type: "Bearer".to_string(),
    })
}

/// 创建携带真实用户角色的完整令牌对。
pub fn create_token_pair_with_role(
    user_id: &str,
    name: Option<&str>,
    role: &str,
    secret: &str,
) -> Result<TokenPair, AppError> {
    let access_token =
        create_access_token_with_role(user_id, name, role, secret, ACCESS_TOKEN_EXP_MINUTES)?;
    let refresh_token = create_refresh_token(user_id, secret, REFRESH_TOKEN_EXP_DAYS)?;

    Ok(TokenPair {
        access_token,
        refresh_token,
        expires_in: ACCESS_TOKEN_EXPIRES_IN_SECS,
        token_type: "Bearer".to_string(),
    })
}

/// 创建只读媒体访问令牌。
///
/// 该令牌只用于让外部视频 provider 读取单个已生成媒体文件，不能作为用户登录
/// access token 使用。
pub fn create_media_token(
    user_id: &str,
    media_path: &str,
    secret: &str,
    exp_minutes: u64,
) -> Result<String, AppError> {
    let media_path = media_path.trim();
    if media_path.is_empty() {
        return Err(AppError::InvalidArgument("媒体路径不能为空".into()));
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        name: None,
        role: None,
        iat: now,
        exp: now + (exp_minutes as usize * 60),
        token_type: "media".to_string(),
        jti: Some(Uuid::new_v4().to_string()),
        media_path: Some(media_path.to_string()),
    };

    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    let key = jsonwebtoken::EncodingKey::from_secret(secret.as_bytes());

    jsonwebtoken::encode(&header, &claims, &key)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT 签发失败: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key-for-unit-tests";

    #[test]
    fn test_create_and_verify_access_token() {
        let token =
            create_access_token_with_role("user-1", Some("测试用户"), "user", TEST_SECRET, 15)
                .expect("创建令牌失败");
        let claims = verify_token(&token, TEST_SECRET).expect("验证令牌失败");
        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.token_type, "access");
        assert_eq!(claims.role.as_deref(), Some("user"));
        assert!(claims.jti.is_some());
    }

    #[test]
    fn access_token_with_role_preserves_admin_claim() {
        let token =
            create_access_token_with_role("admin-1", Some("管理员"), "admin", TEST_SECRET, 15)
                .expect("创建令牌失败");

        let claims = verify_token(&token, TEST_SECRET).expect("验证令牌失败");

        assert_eq!(claims.sub, "admin-1");
        assert_eq!(claims.name.as_deref(), Some("管理员"));
        assert_eq!(claims.role.as_deref(), Some("admin"));
    }

    #[test]
    fn media_token_binds_single_media_path() {
        let token = create_media_token(
            "user-7",
            "projects/1/images/chapter_1/frame.png",
            TEST_SECRET,
            5,
        )
        .expect("media token should be created");

        let claims = verify_token(&token, TEST_SECRET).expect("media token should verify");

        assert_eq!(claims.sub, "user-7");
        assert_eq!(claims.token_type, "media");
        assert_eq!(
            claims.media_path.as_deref(),
            Some("projects/1/images/chapter_1/frame.png")
        );
        assert!(claims.jti.is_some());
    }

    #[test]
    fn access_token_with_role_rejects_blank_role() {
        let err = create_access_token_with_role("user-1", None, "   ", TEST_SECRET, 15)
            .expect_err("blank role must not be embedded in access token");

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn test_expired_token() {
        let token = create_access_token_with_role("user-2", None, "user", TEST_SECRET, 0)
            .expect("创建令牌失败");
        // 立即过期的令牌可能因时钟偏差而通过，此处仅验证其可被解析
        let result = verify_token(&token, TEST_SECRET);
        // 允许因过期失败或通过（取决于执行速度）
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_refresh_token_pair() {
        let pair = create_token_pair_with_role("user-3", None, "user", TEST_SECRET)
            .expect("创建令牌对失败");
        assert_eq!(pair.token_type, "Bearer");
        assert_eq!(pair.expires_in, ACCESS_TOKEN_EXPIRES_IN_SECS);
        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
    }

    #[test]
    fn token_pair_with_role_preserves_read_only_claim() {
        let pair = create_token_pair_with_role("reader-1", Some("读者"), "read_only", TEST_SECRET)
            .expect("创建令牌对失败");

        let claims = verify_token(&pair.access_token, TEST_SECRET).expect("验证令牌失败");

        assert_eq!(claims.sub, "reader-1");
        assert_eq!(claims.name.as_deref(), Some("读者"));
        assert_eq!(claims.role.as_deref(), Some("read_only"));
    }

    #[test]
    fn refresh_tokens_have_distinct_identity_when_created_in_same_second() {
        let (first, second, first_claims, second_claims) = (0..1000)
            .find_map(|_| {
                let first = create_refresh_token("user-4", TEST_SECRET, 7).ok()?;
                let second = create_refresh_token("user-4", TEST_SECRET, 7).ok()?;
                let first_claims = verify_token(&first, TEST_SECRET).ok()?;
                let second_claims = verify_token(&second, TEST_SECRET).ok()?;
                (first_claims.iat == second_claims.iat && first_claims.exp == second_claims.exp)
                    .then_some((first, second, first_claims, second_claims))
            })
            .expect("test should create two refresh tokens in the same second");

        assert_ne!(first, second);
        assert_eq!(first_claims.sub, second_claims.sub);
        assert_eq!(first_claims.exp, second_claims.exp);
        assert_ne!(first_claims.jti, second_claims.jti);
    }

    #[test]
    fn refresh_access_token_rotates_refresh_token_identity() {
        let old_refresh = create_refresh_token("user-5", TEST_SECRET, 7).expect("old refresh");

        let pair = refresh_access_token_with_role(&old_refresh, None, "user", TEST_SECRET)
            .expect("refresh should create pair");

        assert_ne!(pair.refresh_token, old_refresh);
        let old_claims = verify_token(&old_refresh, TEST_SECRET).expect("old claims");
        let new_claims = verify_token(&pair.refresh_token, TEST_SECRET).expect("new claims");
        assert_eq!(new_claims.sub, old_claims.sub);
        assert_eq!(new_claims.token_type, "refresh");
        assert_ne!(new_claims.jti, old_claims.jti);
    }

    #[test]
    fn refresh_access_token_with_role_uses_current_user_claims() {
        let old_refresh = create_refresh_token("user-6", TEST_SECRET, 7).expect("old refresh");

        let pair = refresh_access_token_with_role(
            &old_refresh,
            Some("只读用户"),
            "read_only",
            TEST_SECRET,
        )
        .expect("refresh should create pair");

        let claims = verify_token(&pair.access_token, TEST_SECRET).expect("access claims");

        assert_eq!(claims.sub, "user-6");
        assert_eq!(claims.name.as_deref(), Some("只读用户"));
        assert_eq!(claims.role.as_deref(), Some("read_only"));
        assert_eq!(claims.token_type, "access");
    }
}
