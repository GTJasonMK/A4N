use a4n_config::optional_non_empty_env_string;
use a4n_error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeProfile {
    Full,
    Minimal,
    Sidecar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeRole {
    Host,
    AgentWorker,
}

impl RuntimeProfile {
    pub fn configured_from_env() -> Result<Option<Self>, AppError> {
        optional_non_empty_env_string("A4N_RUNTIME_PROFILE")?
            .map(|value| Self::parse(&value))
            .transpose()
    }

    pub fn default_for_build() -> Self {
        if cfg!(feature = "embedded-capabilities") {
            Self::Full
        } else {
            Self::Sidecar
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, AppError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" => Err(AppError::ConfigError(
                "A4N_RUNTIME_PROFILE 不能为空；可选值: full, minimal, sidecar".to_string(),
            )),
            "full" => Ok(Self::Full),
            "minimal" => Ok(Self::Minimal),
            "sidecar" => Ok(Self::Sidecar),
            other => Err(AppError::ConfigError(format!(
                "不支持的 A4N_RUNTIME_PROFILE: {other}，可选值: full, minimal, sidecar"
            ))),
        }
    }

    pub(crate) fn validate_build_capabilities(self) -> Result<(), AppError> {
        if self != Self::Full {
            return Ok(());
        }

        let missing = [
            ("embedded-prompt-git", cfg!(feature = "embedded-prompt-git")),
            (
                "embedded-llm-provider",
                cfg!(feature = "embedded-llm-provider"),
            ),
            ("local-embedding", cfg!(feature = "local-embedding")),
            ("embedded-rag-index", cfg!(feature = "embedded-rag-index")),
            (
                "embedded-media-providers",
                cfg!(feature = "embedded-media-providers"),
            ),
        ]
        .into_iter()
        .filter_map(|(feature, enabled)| (!enabled).then_some(feature))
        .collect::<Vec<_>>();

        if missing.is_empty() {
            return Ok(());
        }

        Err(AppError::ConfigError(format!(
            "full runtime profile 要求完整 embedded capabilities，当前构建缺少 features: {}；请使用 `--features embedded-capabilities` 构建",
            missing.join(", ")
        )))
    }

    pub(crate) fn initializes_rag(self) -> bool {
        matches!(self, Self::Full)
    }

    pub(crate) fn uses_sidecars(self) -> bool {
        matches!(self, Self::Sidecar)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Minimal => "minimal",
            Self::Sidecar => "sidecar",
        }
    }
}
