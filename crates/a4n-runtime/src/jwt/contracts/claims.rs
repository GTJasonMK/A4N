use serde::{Deserialize, Serialize};

/// JWT 声明
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub name: Option<String>,
    pub role: Option<String>,
    pub iat: usize,
    pub exp: usize,
    pub token_type: String,
    pub jti: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub media_path: Option<String>,
}
