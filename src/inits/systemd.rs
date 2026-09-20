use crate::inits::ServiceManager;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct SystemdManager {
    name: &'static str,
}

impl SystemdManager {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }

    fn service_path(&self) -> String {
        format!("/etc/systemd/system/{}.service", self.name)
    }

    fn run_command(&self, args: &[&str]) -> Result<(), String> {
        let output = Command::new("systemctl")
            .args(args)
            .output()
            .map_err(|e| format!("{}{}", rust_i18n::t!("err_exec_systemctl"), e))?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!(
                "systemctl {} failed: {}",
                args.join(" "),
                if stderr.is_empty() {
                    format!("exit code {:?}", output.status.code())
                } else {
                    stderr
                }
            ))
        }
    }
}

impl ServiceManager for SystemdManager {
    fn is_installed(&self) -> bool {
        Path::new(&self.service_path()).exists()
    }

    fn is_active(&self) -> bool {
        let output = Command::new("systemctl").arg("is-active").arg(self.name).output();
        match output {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }

    fn is_enabled(&self) -> bool {
        let output = Command::new("systemctl").arg("is-enabled").arg(self.name).output();
        match output {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }

    fn install(&self, exe_path: &Path, config_path: &Path, cache_dir: &Path) -> Result<(), String> {
        let exe_str = exe_path.to_str().ok_or(rust_i18n::t!("err_invalid_exe").into_owned())?;
        let config_str = config_path
            .to_str()
            .ok_or(rust_i18n::t!("err_invalid_cfg").into_owned())?;
        let cache_str = cache_dir
            .to_str()
            .ok_or(rust_i18n::t!("err_invalid_cache").into_owned())?;

        let service_content = format!(
            r#"[Unit]
Description={}
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={} --config {} --cache-dir {}
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
"#,
            crate::inits::service_description(self.name),
            exe_str,
            config_str,
            cache_str
        );

        fs::write(self.service_path(), service_content)
            .map_err(|e| format!("{}{}", rust_i18n::t!("err_write_svc"), e))?;

        self.run_command(&["daemon-reload"])?;
        self.run_command(&["enable", self.name])?;

        Ok(())
    }

    fn uninstall(&self) -> Result<(), String> {
        let _ = self.run_command(&["stop", self.name]);
        let _ = self.run_command(&["disable", self.name]);

        let service_path = self.service_path();
        if Path::new(&service_path).exists() {
            fs::remove_file(&service_path).map_err(|e| format!("{}{}", rust_i18n::t!("err_rm_svc"), e))?;
        }

        self.run_command(&["daemon-reload"])?;
        Ok(())
    }

    fn start(&self) -> Result<(), String> {
        self.run_command(&["start", self.name])
    }

    fn stop(&self) -> Result<(), String> {
        self.run_command(&["stop", self.name])
    }

    fn restart(&self) -> Result<(), String> {
        self.run_command(&["restart", self.name])
    }
}
