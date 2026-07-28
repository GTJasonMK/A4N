use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecretMaterialScope {
    None,
    LocalSidecarOnly,
    TrustedRemote,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SecretForwardingPolicy {
    pub scope: SecretMaterialScope,
    pub allow_system_secret: bool,
    pub allow_user_secret: bool,
    pub allow_upload_token: bool,
}

impl SecretForwardingPolicy {
    pub fn deny_all() -> Self {
        Self {
            scope: SecretMaterialScope::None,
            allow_system_secret: false,
            allow_user_secret: false,
            allow_upload_token: false,
        }
    }

    pub fn local_user_secret() -> Self {
        Self {
            scope: SecretMaterialScope::LocalSidecarOnly,
            allow_system_secret: false,
            allow_user_secret: true,
            allow_upload_token: false,
        }
    }

    pub fn trusted_remote_no_secrets() -> Self {
        Self {
            scope: SecretMaterialScope::TrustedRemote,
            allow_system_secret: false,
            allow_user_secret: false,
            allow_upload_token: false,
        }
    }

    pub fn denies_all_secrets(&self) -> bool {
        !self.allow_system_secret && !self.allow_user_secret && !self.allow_upload_token
    }

    pub fn is_secret_free(&self) -> bool {
        self.denies_all_secrets()
            && matches!(
                self.scope,
                SecretMaterialScope::None | SecretMaterialScope::TrustedRemote
            )
    }

    pub fn is_deny_all(&self) -> bool {
        self.scope == SecretMaterialScope::None && self.denies_all_secrets()
    }

    pub fn is_local_user_secret_only(&self) -> bool {
        self.scope == SecretMaterialScope::LocalSidecarOnly
            && !self.allow_system_secret
            && self.allow_user_secret
            && !self.allow_upload_token
    }
}
