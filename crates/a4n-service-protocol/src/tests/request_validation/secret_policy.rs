use super::*;

#[test]
fn default_secret_policy_denies_all() {
    let policy = SecretForwardingPolicy::deny_all();

    assert_eq!(policy.scope, SecretMaterialScope::None);
    assert!(!policy.allow_system_secret);
    assert!(!policy.allow_user_secret);
    assert!(!policy.allow_upload_token);
    assert!(policy.is_deny_all());
    assert!(policy.is_secret_free());
}

#[test]
fn trusted_remote_content_policy_is_secret_free_but_not_local_deny_all() {
    let policy = SecretForwardingPolicy::trusted_remote_no_secrets();
    assert!(policy.denies_all_secrets());
    assert!(policy.is_secret_free());
    assert!(!policy.is_deny_all());
    assert_eq!(policy.scope, SecretMaterialScope::TrustedRemote);
}

#[test]
fn local_user_secret_policy_is_minimal_and_not_deny_all() {
    let policy = SecretForwardingPolicy::local_user_secret();

    assert!(!policy.is_deny_all());
    assert_eq!(policy.scope, SecretMaterialScope::LocalSidecarOnly);
    assert!(!policy.allow_system_secret);
    assert!(policy.allow_user_secret);
    assert!(!policy.allow_upload_token);
    assert!(policy.is_local_user_secret_only());
}

#[test]
fn user_secret_forwarding_requires_exact_minimal_policy() {
    let local_policy = SecretForwardingPolicy::local_user_secret();
    assert!(local_policy.is_local_user_secret_only());

    let deny_all_policy = SecretForwardingPolicy::deny_all();
    assert!(!deny_all_policy.is_local_user_secret_only());

    let trusted_remote_policy = SecretForwardingPolicy {
        scope: SecretMaterialScope::TrustedRemote,
        allow_system_secret: false,
        allow_user_secret: true,
        allow_upload_token: false,
    };
    assert!(!trusted_remote_policy.is_local_user_secret_only());

    let local_policy_with_upload_token = SecretForwardingPolicy {
        allow_upload_token: true,
        ..SecretForwardingPolicy::local_user_secret()
    };
    assert!(!local_policy_with_upload_token.is_local_user_secret_only());

    let local_policy_with_system_secret = SecretForwardingPolicy {
        allow_system_secret: true,
        ..SecretForwardingPolicy::local_user_secret()
    };
    assert!(!local_policy_with_system_secret.is_local_user_secret_only());
}
