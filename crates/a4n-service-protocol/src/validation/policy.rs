#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequiredSecretPolicy {
    AllowAny,
    DenyAll { rejection_message: &'static str },
    LocalUserSecretForwarding { rejection_message: &'static str },
}
