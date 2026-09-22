mod configuration;
mod foundation;
mod lifecycle;
mod remaining;
mod state;

use configuration::load;
use foundation::start as start_foundation;
use lifecycle::run as run_lifecycle;
use remaining::start as start_remaining;

pub(crate) fn run() -> anyhow::Result<()> {
    #[cfg(feature = "test-embedded-processes")]
    if super::embedded::run_if_requested()?.is_some() {
        return Ok(());
    }
    if onnxruntime_environment_required() {
        configure_onnxruntime_environment()?;
    }
    let config = load()?;
    let foundation = start_foundation(&config)?;
    let runtime = start_remaining(config, foundation)?;
    run_lifecycle(runtime)
}

fn onnxruntime_environment_required() -> bool {
    let full_runtime = std::env::var("A4N_RUNTIME_PROFILE")
        .is_ok_and(|value| value.trim().eq_ignore_ascii_case("full"));
    let managed_sidecars = std::env::var("A4N_MANAGED_SIDECARS").is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    });
    full_runtime || managed_sidecars
}

fn configure_onnxruntime_environment() -> anyhow::Result<()> {
    let library_name = onnxruntime_library_name();
    let root = std::env::var_os("A4N_ONNXRUNTIME_ROOT")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("A4N_SIDECAR_BIN_DIR")
                .map(std::path::PathBuf::from)
                .filter(|path| path.join(library_name).is_file())
        })
        .or_else(|| {
            std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .map(|home| {
                    home.join(".local/share/a4n/onnxruntime")
                        .join(onnxruntime_version())
                })
        })
        .ok_or_else(|| {
            anyhow::anyhow!("无法确定 ONNX Runtime 目录；请设置 A4N_ONNXRUNTIME_ROOT")
        })?;
    let library_dir = if root.join("lib").join(library_name).is_file() {
        root.join("lib")
    } else {
        root.clone()
    };
    let library = library_dir.join(library_name);
    if !library.is_file() {
        return Err(anyhow::anyhow!(
            "缺少 ONNX Runtime 动态库: {}；请先运行 ./scripts/dev/setup-onnxruntime.sh",
            library.display()
        ));
    }
    let mut loader_paths = vec![library_dir.clone()];
    if let Some(current) = std::env::var_os(runtime_loader_path_var()) {
        loader_paths.extend(std::env::split_paths(&current));
    }
    let loader_path = std::env::join_paths(loader_paths)?;
    // SAFETY: server startup runs before any managed sidecar is spawned.
    unsafe {
        std::env::set_var("A4N_ONNXRUNTIME_ROOT", &root);
        std::env::set_var("ORT_LIB_PATH", &library_dir);
        std::env::set_var("ORT_PREFER_DYNAMIC_LINK", "1");
        std::env::set_var(runtime_loader_path_var(), loader_path);
    }
    Ok(())
}

fn onnxruntime_version() -> &'static str {
    include_str!("../../../../../tools/onnxruntime/VERSION").trim()
}

#[cfg(target_os = "linux")]
fn onnxruntime_library_name() -> &'static str {
    "libonnxruntime.so.1"
}

#[cfg(target_os = "macos")]
fn onnxruntime_library_name() -> &'static str {
    "libonnxruntime.dylib"
}

#[cfg(windows)]
fn onnxruntime_library_name() -> &'static str {
    "onnxruntime.dll"
}

#[cfg(target_os = "linux")]
fn runtime_loader_path_var() -> &'static str {
    "LD_LIBRARY_PATH"
}

#[cfg(target_os = "macos")]
fn runtime_loader_path_var() -> &'static str {
    "DYLD_LIBRARY_PATH"
}

#[cfg(windows)]
fn runtime_loader_path_var() -> &'static str {
    "PATH"
}
