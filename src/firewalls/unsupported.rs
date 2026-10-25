use super::FirewallBackend;

pub struct UnsupportedBackend;

impl FirewallBackend for UnsupportedBackend {
    fn setup(&self, _tcp_ports: &str, _udp_ports: &str, _interface: &str) -> Result<(), String> {
        Err(rust_i18n::t!("err_backend_unsupported").into_owned())
    }

    fn clear(&self) -> Result<(), String> {
        Ok(())
    }
}
