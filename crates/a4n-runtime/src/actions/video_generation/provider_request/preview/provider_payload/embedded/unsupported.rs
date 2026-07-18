pub(super) fn unsupported_backend_error(backend: &str) -> String {
    format!("未知视频 backend={backend}，无法展示真实 provider 请求体")
}
