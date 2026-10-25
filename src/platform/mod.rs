pub mod launcher;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(unix)]
pub mod linux;

#[cfg(target_os = "windows")]
pub use windows::ensure_admin;

#[cfg(unix)]
pub use linux::ensure_admin;

#[cfg(not(any(target_os = "windows", unix)))]
pub fn ensure_admin() {}

#[cfg(target_os = "windows")]
pub use windows::setup_console;

#[cfg(not(target_os = "windows"))]
pub fn setup_console() {}

#[cfg(target_os = "windows")]
pub use windows::is_nfqws_running;

#[cfg(unix)]
pub use linux::is_nfqws_running;

#[cfg(not(any(target_os = "windows", unix)))]
pub fn is_nfqws_running() -> bool {
    false
}
