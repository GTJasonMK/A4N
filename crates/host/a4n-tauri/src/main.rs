fn main() {
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_GST_DISABLE_GL_SINK").is_none() {
        // WebKitGTK 的 GL 视频 sink 在 NVIDIA/Wayland 上 seek 时会闪黑。
        // 使用普通视频 sink，保留 WebView 合成加速和用户显式设置。
        // SAFETY: 进程入口尚未启动 Tauri、日志或其他工作线程。
        unsafe { std::env::set_var("WEBKIT_GST_DISABLE_GL_SINK", "1") };
    }
    a4n_tauri_lib::run();
}
