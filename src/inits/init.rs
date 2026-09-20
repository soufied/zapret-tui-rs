use crate::inits::ServiceManager;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct InitManager {
    name: &'static str,
}

impl InitManager {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }

    fn script_path(&self) -> String {
        format!("/etc/init.d/{}", self.name)
    }

    fn run_init_script(&self, action: &str) -> Result<(), String> {
        let script_path = self.script_path();
        let output = Command::new(&script_path)
            .arg(action)
            .output()
            .map_err(|e| format!("{}{}", rust_i18n::t!("err_exec_init"), e))?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!(
                "{} {} failed: {}",
                script_path,
                action,
                if stderr.is_empty() {
                    format!("exit code {:?}", output.status.code())
                } else {
                    stderr
                }
            ))
        }
    }

    fn register_service(&self) -> Result<(), String> {
        if Command::new("which")
            .arg("update-rc.d")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            let output = Command::new("update-rc.d")
                .arg(self.name)
                .arg("defaults")
                .output()
                .map_err(|e| format!("{}{}", rust_i18n::t!("err_exec_update_rc"), e))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return Err(format!("update-rc.d failed: {}", stderr));
            }
        }

        else if Command::new("which")
            .arg("chkconfig")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            let output = Command::new("chkconfig")
                .arg("--add")
                .arg(self.name)
                .output()
                .map_err(|e| format!("{}{}", rust_i18n::t!("err_exec_chkconfig"), e))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return Err(format!("chkconfig --add failed: {}", stderr));
            }
        }
        Ok(())
    }

    fn unregister_service(&self) -> Result<(), String> {
        if Command::new("which")
            .arg("update-rc.d")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            let _ = Command::new("update-rc.d")
                .args(["-f", self.name, "remove"])
                .output();
        } else if Command::new("which")
            .arg("chkconfig")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            let _ = Command::new("chkconfig").arg("--del").arg(self.name).output();
        }
        Ok(())
    }
}

impl ServiceManager for InitManager {
    fn is_installed(&self) -> bool {
        Path::new(&self.script_path()).exists()
    }

    fn is_active(&self) -> bool {
        if !self.is_installed() {
            return false;
        }
        let output = Command::new(self.script_path()).arg("status").output();
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
            r#"#!/bin/sh
### BEGIN INIT INFO
# Provides:          {name}
# Required-Start:    $network $local_fs
# Required-Stop:     $network $local_fs
# Default-Start:     2 3 4 5
# Default-Stop:      0 1 6
# Short-Description: {desc}
### END INIT INFO

DESC="{desc}"
NAME="{name}"
DAEMON="{}"
DAEMON_ARGS="--config {} --cache-dir {}"
PIDFILE="/var/run/$NAME.pid"

case "$1" in
    start)
        echo "Starting $DESC"
        start-stop-daemon --start --background --make-pidfile --pidfile "$PIDFILE" --exec "$DAEMON" -- $DAEMON_ARGS
        ;;
    stop)
        echo "Stopping $DESC"
        start-stop-daemon --stop --pidfile "$PIDFILE" --retry 5
        ;;
    restart)
        $0 stop
        $0 start
        ;;
    status)
        if [ -f "$PIDFILE" ] && kill -0 $(cat "$PIDFILE") 2>/dev/null; then
            echo "$DESC is running"
            exit 0
        else
            echo "$DESC is stopped"
            exit 3
        fi
        ;;
    *)
        echo "Usage: $0 {{start|stop|restart|status}}"
        exit 1
        ;;
esac
"#,
            exe_str,
            config_str,
            cache_str,
            name = self.name,
            desc = crate::inits::service_description(self.name)
        );

        let script_path = self.script_path();
        fs::write(&script_path, script_content)
            .map_err(|e| format!("{}{}", rust_i18n::t!("err_write_sysv"), e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("{}{}", rust_i18n::t!("err_chmod_sysv"), e))?;
        }

        self.register_service()?;

        Ok(())
    }

    fn uninstall(&self) -> Result<(), String> {
        let _ = self.run_init_script("stop");
        let _ = self.unregister_service();

        let script_path = self.script_path();
        if Path::new(&script_path).exists() {
            fs::remove_file(&script_path).map_err(|e| format!("{}{}", rust_i18n::t!("err_rm_sysv"), e))?;
        }

        Ok(())
    }

    fn start(&self) -> Result<(), String> {
        self.run_init_script("start")
    }

    fn stop(&self) -> Result<(), String> {
        self.run_init_script("stop")
    }

    fn restart(&self) -> Result<(), String> {
        self.run_init_script("restart")
    }
}
