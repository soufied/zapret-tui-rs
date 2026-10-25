#[cfg(target_os = "linux")]
pub mod backends;

#[cfg(target_os = "windows")]
pub mod windivert;

pub const NFQUEUE_NUM: u32 = 200;
pub const FWMARK_HEX: &str = "0x40000000";
pub const FWMARK_MASK: &str = "0x40000000/0x40000000";

pub trait FirewallBackend {
    fn setup(&self, tcp_ports: &str, udp_ports: &str, interface: &str) -> Result<(), String>;
    fn clear(&self) -> Result<(), String>;
}

#[cfg(target_os = "linux")]
pub use backends::LinuxBackend;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub mod unsupported;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub use unsupported::UnsupportedBackend;
