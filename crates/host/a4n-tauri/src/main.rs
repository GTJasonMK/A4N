fn main() {
    #[cfg(target_os = "linux")]
    for variable in [
        "WEBKIT_GST_DISABLE_GL_SINK",
        "WEBKIT_DISABLE_DMABUF_RENDERER",
    ] {
        // NVIDIA/Wayland 的 GL 视频 sink 会在 seek 时闪黑；DMABuf 通道
        // 还会在暂停后缩放窗口时丢失画面。保留用户显式设置。
        if std::env::var_os(variable).is_none() {
            // SAFETY: 进程入口尚未启动 Tauri、日志或其他工作线程。
            unsafe { std::env::set_var(variable, "1") };
        }
    }
    a4n_tauri_lib::run();
}
