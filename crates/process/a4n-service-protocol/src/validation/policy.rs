#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequiredSecretPolicy {
    AllowAny,
    DenyAll { rejection_message: &'static str },
}
