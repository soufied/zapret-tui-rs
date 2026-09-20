use crate::inits::ServiceManager;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct OpenRcManager {
    name: &'static str,
}

impl OpenRcManager {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }

    fn script_path(&self) -> String {
        format!("/etc/init.d/{}", self.name)
    }

    fn run_rc_service(&self, action: &str) -> Result<(), String> {
        let output = Command::new("rc-service")
            .arg(self.name)
            .arg(action)
            .output()
            .map_err(|e| format!("{}{}", rust_i18n::t!("err_exec_rc_svc"), e))?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!(
                "rc-service {} {} failed: {}",
                self.name,
                action,
                if stderr.is_empty() {
                    format!("exit code {:?}", output.status.code())
                } else {
                    stderr
                }
            ))
        }
    }

    fn run_rc_update(&self, action: &str, runlevel: &str) -> Result<(), String> {
        let output = Command::new("rc-update")
            .arg(action)
            .arg(self.name)
            .arg(runlevel)
            .output()
            .map_err(|e| format!("{}{}", rust_i18n::t!("err_exec_rc_update"), e))?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!(
                "rc-update {} {} {} failed: {}",
                action,
                self.name,
                runlevel,
                if stderr.is_empty() {
                    format!("exit code {:?}", output.status.code())
                } else {
                    stderr
                }
            ))
        }
    }
}

impl ServiceManager for OpenRcManager {
    fn is_installed(&self) -> bool {
        Path::new(&self.script_path()).exists()
    }

    fn is_active(&self) -> bool {
        let output = Command::new("rc-service").arg(self.name).arg("status").output();
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

        let script_content = format!(
            r#"#!/sbin/openrc-run

description="{}"
supervisor="supervise-daemon"
respawn_delay=5
respawn_max=10

command="{}"
command_args="--config {} --cache-dir {}"

depend() {{
    need net
    after firewall
}}
"#,
            crate::inits::service_description(self.name),
            exe_str,
            config_str,
            cache_str
        );

        let script_path = self.script_path();
        fs::write(&script_path, script_content)
            .map_err(|e| format!("{}{}", rust_i18n::t!("err_write_openrc"), e))?;

        #[cfg(unix)]
        {
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))
                    .map_err(|e| format!("{}{}", rust_i18n::t!("err_chmod_openrc"), e))?;
            }
        }

        self.run_rc_update("add", "default")?;

        Ok(())
    }

    fn uninstall(&self) -> Result<(), String> {
        let _ = self.run_rc_service("stop");
        let _ = self.run_rc_update("del", "default");

        let script_path = self.script_path();
        if Path::new(&script_path).exists() {
            fs::remove_file(&script_path).map_err(|e| format!("{}{}", rust_i18n::t!("err_rm_openrc"), e))?;
        }

        Ok(())
    }

    fn start(&self) -> Result<(), String> {
        self.run_rc_service("start")
    }

    fn stop(&self) -> Result<(), String> {
        self.run_rc_service("stop")
    }

    fn restart(&self) -> Result<(), String> {
        self.run_rc_service("restart")
    }
}
