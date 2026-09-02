use std::ffi::OsString;
use std::path::PathBuf;

use crate::error::AppError;

/// 读取可选字符串环境变量。未设置返回 `None`，已设置为空则明确报错。
pub fn optional_non_empty_env_string(key: &str) -> Result<Option<String>, AppError> {
    match std::env::var(key) {
        Ok(value) => {
            let value = value.trim().to_string();
            if value.is_empty() {
                Err(AppError::ConfigError(format!("{key} 不能为空")))
            } else {
                Ok(Some(value))
            }
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(AppError::ConfigError(format!("{key} 无法读取: {error}"))),
    }
}

pub fn required_non_empty_env_string(key: &str) -> Result<String, AppError> {
    optional_non_empty_env_string(key)?
        .ok_or_else(|| AppError::ConfigError(format!("{key} 未设置")))
}

pub fn env_flag_enabled(key: &str) -> Result<bool, AppError> {
    let Some(value) = optional_non_empty_env_string(key)? else {
        return Ok(false);
    };
    parse_env_flag(key, &value)
}

fn parse_env_flag(key: &str, value: &str) -> Result<bool, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(AppError::ConfigError(format!(
            "{key} 必须是 1/0、true/false、yes/no 或 on/off"
        ))),
    }
}

pub fn validate_postgres_database_url(key: &str, url: &str) -> Result<(), AppError> {
    if url.trim().starts_with("postgres://") || url.trim().starts_with("postgresql://") {
        Ok(())
    } else {
        Err(AppError::ConfigError(format!(
            "{key} 必须是 postgres:// 或 postgresql:// 连接串"
        )))
    }
}

pub fn required_public_app_endpoint(path: &str) -> Result<String, AppError> {
    let value = required_non_empty_env_string("A4N_PUBLIC_APP_URL")?;
    public_app_endpoint(&value, path)
}

fn public_app_endpoint(value: &str, path: &str) -> Result<String, AppError> {
    let url = url::Url::parse(value)
        .map_err(|error| AppError::ConfigError(format!("A4N_PUBLIC_APP_URL 无效: {error}")))?;
    let Some(host) = url.host_str() else {
        return Err(AppError::ConfigError(
            "A4N_PUBLIC_APP_URL 必须是 http:// 或 https:// URL".to_string(),
        ));
    };
    if !matches!(url.scheme(), "http" | "https") {
        return Err(AppError::ConfigError(
            "A4N_PUBLIC_APP_URL 必须是 http:// 或 https:// URL".to_string(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::ConfigError(
            "A4N_PUBLIC_APP_URL 不能包含用户名或密码".to_string(),
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(AppError::ConfigError(
            "A4N_PUBLIC_APP_URL 不能包含查询参数或 fragment".to_string(),
        ));
    }
    let host = host.trim_end_matches('.');
    let forbidden_host = host.eq_ignore_ascii_case("localhost")
        || url.host().is_some_and(|host| match host {
            url::Host::Ipv4(ip) => ip.is_loopback() || ip.is_unspecified(),
            url::Host::Ipv6(ip) => ip.is_loopback() || ip.is_unspecified(),
            url::Host::Domain(_) => false,
        });
    if forbidden_host {
        return Err(AppError::ConfigError(
            "A4N_PUBLIC_APP_URL 不能使用 localhost、loopback 或 unspecified 地址".to_string(),
        ));
    }
    Ok(format!("{}{}", url.as_str().trim_end_matches('/'), path))
}

/// 读取可选路径环境变量。未设置返回 `None`，已设置为空则明确报错。
pub fn optional_non_empty_env_path(key: &str) -> Result<Option<PathBuf>, AppError> {
    parse_optional_non_empty_env_path(key, std::env::var_os(key))
}

fn parse_optional_non_empty_env_path(
    key: &str,
    value: Option<OsString>,
) -> Result<Option<PathBuf>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.to_string_lossy().trim().is_empty() {
        return Err(AppError::ConfigError(format!("{key} 不能为空")));
    }
    Ok(Some(PathBuf::from(value)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_app_endpoint_has_one_strict_url_contract() {
        assert_eq!(
            public_app_endpoint("https://media.example.com/a4n/", "/api/media").unwrap(),
            "https://media.example.com/a4n/api/media"
        );
        for value in [
            "ftp://media.example.com",
            "https://localhost",
            "https://127.0.0.10",
            "https://[::1]",
            "https://[::]",
            "https://user:secret@media.example.com",
            "https://media.example.com?token=x",
        ] {
            assert!(public_app_endpoint(value, "/api/media").is_err(), "{value}");
        }
    }

    #[test]
    fn optional_path_distinguishes_unset_empty_and_configured_values() {
        assert_eq!(
            parse_optional_non_empty_env_path("A4N_PATH", None).unwrap(),
            None
        );

        let error = parse_optional_non_empty_env_path("A4N_PATH", Some(OsString::from("   ")))
            .expect_err("configured empty path must fail");
        assert!(error.to_string().contains("A4N_PATH 不能为空"));

        assert_eq!(
            parse_optional_non_empty_env_path("A4N_PATH", Some(OsString::from("data/cache")),)
                .unwrap(),
            Some(PathBuf::from("data/cache"))
        );
    }

    #[test]
    fn postgres_database_url_requires_postgres_scheme() {
        for value in [
            "postgres://user:password@localhost:5432/a4n",
            "postgresql://user:password@localhost:5432/a4n",
        ] {
            validate_postgres_database_url("A4N_DATABASE_URL", value).unwrap();
        }
        for value in ["", "sqlite://data/a4n.db", "mysql://localhost/a4n"] {
            assert!(validate_postgres_database_url("A4N_DATABASE_URL", value).is_err());
        }
    }

    #[test]
    fn environment_flag_has_one_strict_vocabulary() {
        for value in ["1", "true", "YES", " on "] {
            assert!(parse_env_flag("A4N_FLAG", value).unwrap(), "{value}");
        }
        for value in ["0", "false", "NO", " off "] {
            assert!(!parse_env_flag("A4N_FLAG", value).unwrap(), "{value}");
        }
        assert!(parse_env_flag("A4N_FLAG", "enabled").is_err());
    }
}
