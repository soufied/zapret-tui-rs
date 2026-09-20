use crate::inits::ServiceManager;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct DinitManager {
    name: &'static str,
}

impl DinitManager {
    const BOOT_DIR: &'static str = "/etc/dinit.d/boot.d";

    pub fn new(name: &'static str) -> Self {
        Self { name }
    }

    fn service_path(&self) -> String {
        format!("/etc/dinit.d/{}", self.name)
    }

    fn boot_link(&self) -> String {
        format!("{}/{}", Self::BOOT_DIR, self.name)
    }

    fn run_dinitctl(&self, action: &str) -> Result<(), String> {
        let output = Command::new("dinitctl")
            .arg(action)
            .arg(self.name)
            .output()
            .map_err(|e| format!("{}{}", rust_i18n::t!("err_exec_dinit"), e))?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!(
                "dinitctl {} {} failed: {}",
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

impl ServiceManager for DinitManager {
    fn is_installed(&self) -> bool {
        Path::new(&self.service_path()).exists()
    }

    fn is_active(&self) -> bool {
        let output = Command::new("dinitctl").arg("status").arg(self.name).output();
        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout.contains("Status: started") || stdout.contains("Status: running")
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

        let service_content = format!(
            r#"type = simple
command = {} --config {} --cache-dir {}
restart = true
restart-delay = 5
"#,
            exe_str, config_str, cache_str
        );

        fs::write(self.service_path(), service_content)
            .map_err(|e| format!("{}{}", rust_i18n::t!("err_write_dinit"), e))?;

        if let Err(e) = fs::create_dir_all(Self::BOOT_DIR) {
            println!(
                "  (could not create boot.d directory, auto-start might not work: {})",
                e
            );
        } else {
            let boot_link = self.boot_link();
            if Path::new(&boot_link).exists() || fs::symlink_metadata(&boot_link).is_ok() {
                let _ = fs::remove_file(&boot_link);
            }
            #[cfg(unix)]
            {
                let _ = std::os::unix::fs::symlink(format!("../{}", self.name), &boot_link);
            }
        }

        Ok(())
    }

    fn uninstall(&self) -> Result<(), String> {
        let _ = self.run_dinitctl("stop");

        let boot_link = self.boot_link();
        if Path::new(&boot_link).exists() || fs::symlink_metadata(&boot_link).is_ok() {
            let _ = fs::remove_file(&boot_link);
        }

        let service_path = self.service_path();
        if Path::new(&service_path).exists() {
            fs::remove_file(&service_path).map_err(|e| format!("{}{}", rust_i18n::t!("err_rm_dinit"), e))?;
        }

        Ok(())
    }

    fn start(&self) -> Result<(), String> {
        self.run_dinitctl("start")
    }

    fn stop(&self) -> Result<(), String> {
        self.run_dinitctl("stop")
    }

    fn restart(&self) -> Result<(), String> {
        self.run_dinitctl("restart")
    }
}
