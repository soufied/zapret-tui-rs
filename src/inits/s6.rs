use crate::inits::ServiceManager;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct S6Manager {
    name: &'static str,
}

impl S6Manager {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }

    fn service_dir(&self) -> String {
        format!("/etc/s6/services/{}", self.name)
    }

    fn run_s6_svc(&self, flag: &str) -> Result<(), String> {
        let service_dir = self.service_dir();
        let output = Command::new("s6-svc")
            .arg(flag)
            .arg(&service_dir)
            .output()
            .map_err(|e| format!("{}{}", rust_i18n::t!("err_exec_s6"), e))?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!(
                "s6-svc {} {} failed: {}",
                flag,
                service_dir,
                if stderr.is_empty() {
                    format!("exit code {:?}", output.status.code())
                } else {
                    stderr
                }
            ))
        }
    }
}

impl ServiceManager for S6Manager {
    fn is_installed(&self) -> bool {
        Path::new(&self.service_dir()).exists()
    }

    fn is_active(&self) -> bool {
        let output = Command::new("s6-svstat").arg(self.service_dir()).output();
        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout.contains("up (pid")
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

        let service_dir = self.service_dir();
        fs::create_dir_all(&service_dir).map_err(|e| format!("{}{}", rust_i18n::t!("err_mkdir_s6"), e))?;

        let run_path = Path::new(&service_dir).join("run");
        let run_content = format!(
            r#"#!/bin/sh
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

        Ok(())
    }

    fn uninstall(&self) -> Result<(), String> {
        let _ = self.run_s6_svc("-d");

        let service_dir = self.service_dir();
        if Path::new(&service_dir).exists() {
            fs::remove_dir_all(&service_dir).map_err(|e| format!("{}{}", rust_i18n::t!("err_rm_s6"), e))?;
        }

        Ok(())
    }

    fn start(&self) -> Result<(), String> {
        self.run_s6_svc("-u")
    }

    fn stop(&self) -> Result<(), String> {
        self.run_s6_svc("-d")
    }

    fn restart(&self) -> Result<(), String> {
        self.run_s6_svc("-r")
    }
}
