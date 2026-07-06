use super::model::SystemProxy;

#[cfg(target_os = "linux")]
use super::linux::read_gnome_system_proxy;
#[cfg(target_os = "macos")]
use super::macos::read_macos_system_proxy;
#[cfg(target_os = "windows")]
use super::windows::read_windows_system_proxy;

pub(super) fn read_system_proxy() -> Option<SystemProxy> {
    read_platform_system_proxy().filter(|proxy| !proxy.is_empty())
}

#[cfg(target_os = "linux")]
fn read_platform_system_proxy() -> Option<SystemProxy> {
    read_gnome_system_proxy()
}

#[cfg(target_os = "macos")]
fn read_platform_system_proxy() -> Option<SystemProxy> {
    read_macos_system_proxy()
}

#[cfg(target_os = "windows")]
fn read_platform_system_proxy() -> Option<SystemProxy> {
    read_windows_system_proxy()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn read_platform_system_proxy() -> Option<SystemProxy> {
    None
}
