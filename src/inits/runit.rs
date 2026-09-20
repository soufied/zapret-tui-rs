use crate::inits::ServiceManager;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct RunitManager {
    name: &'static str,
}

impl RunitManager {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }

    fn sv_dir(&self) -> String {
        format!("/etc/sv/{}", self.name)
    }

    fn get_link_path(&self) -> String {
        if Path::new("/service").exists() && !Path::new("/var/service").exists() {
            format!("/service/{}", self.name)
        } else {
            format!("/var/service/{}", self.name)
        }
    }

    fn run_sv(&self, action: &str) -> Result<(), String> {
        let output = Command::new("sv")
            .arg(action)
            .arg(self.name)
            .output()
            .map_err(|e| format!("{}{}", rust_i18n::t!("err_exec_sv"), e))?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!(
                "sv {} {} failed: {}",
                action,
                self.name,
                if stderr.is_empty() {
                    format!("exit code {:?}", output.status.code())
                } else {
                    stderr
                }
            ))
        }
    }
}

impl ServiceManager for RunitManager {
    fn is_installed(&self) -> bool {
        Path::new(&self.sv_dir()).exists()
    }

    fn is_active(&self) -> bool {
        let output = Command::new("sv").arg("status").arg(self.name).output();
        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout.trim().starts_with("run:")
            }
            _ => false,
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

        let sv_dir = self.sv_dir();
        fs::create_dir_all(&sv_dir).map_err(|e| format!("{}{}", rust_i18n::t!("err_mkdir_runit"), e))?;

        let run_path = Path::new(&sv_dir).join("run");
        let run_content = format!(
            r#"#!/bin/sh
exec 2>&1
exec {} --config {} --cache-dir {}
"#,
            exe_str, config_str, cache_str
        );
        fs::write(&run_path, run_content).map_err(|e| format!("{}{}", rust_i18n::t!("err_write_run"), e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&run_path, fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("{}{}", rust_i18n::t!("err_chmod_run"), e))?;
        }

        let link_path = self.get_link_path();
        if Path::new(&link_path).exists() || fs::symlink_metadata(&link_path).is_ok() {
            let _ = fs::remove_file(&link_path);
        }

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&sv_dir, &link_path)
                .map_err(|e| format!("{}{}: {}", rust_i18n::t!("err_symlink"), link_path, e))?;
        }

        Ok(())
    }

    fn uninstall(&self) -> Result<(), String> {
        let _ = self.run_sv("stop");

        let link_path = self.get_link_path();
        if Path::new(&link_path).exists() || fs::symlink_metadata(&link_path).is_ok() {
            fs::remove_file(&link_path)
                .map_err(|e| format!("{}{}: {}", rust_i18n::t!("err_rm_symlink"), link_path, e))?;
        }

        let sv_dir = self.sv_dir();
        if Path::new(&sv_dir).exists() {
            fs::remove_dir_all(&sv_dir).map_err(|e| format!("{}{}", rust_i18n::t!("err_rm_runit"), e))?;
        }

        Ok(())
    }

    fn start(&self) -> Result<(), String> {
        self.run_sv("start")
    }

    fn stop(&self) -> Result<(), String> {
        self.run_sv("stop")
    }

    fn restart(&self) -> Result<(), String> {
        self.run_sv("restart")
    }
}
