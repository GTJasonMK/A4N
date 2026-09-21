#[derive(Debug, Clone, Copy)]
pub struct BootstrapAdminInput<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password: &'a str,
}
