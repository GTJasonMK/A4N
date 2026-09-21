pub(in crate::sidecar_clients::service_manager) fn sidecar_executable_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}
