use dialoguer::{theme::ColorfulTheme, Confirm};
use mycelial::{create_config, download_binaries, ConfigAction};
use service_manager::*;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub struct Service {}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
const CLIENT_DEST_PATH: &str = "/usr/local/bin/myceliald";
const CLIENT_CONFIG_PATH: &str = "/etc/mycelial/config.toml";
const CLIENT_DB_PATH: &str = "/var/lib/mycelial/daemon.db";
const SERVICE_LABEL: &str = "com.mycelial.daemon";
impl Service {
    pub fn new() -> Service {
        Service {}
    }
    pub async fn add_client(&self, config: Option<String>) -> Result<()> {
        self.download_client().await?;
        self.configure_client(config).await?;
        self.check_client_database()?;
        self.install_and_start()?;
        Ok(())
    }
    pub async fn remove_client(&self, purge: bool) -> Result<()> {
        self.uninstall_client()?;
        if purge {
            self.purge_client()?;
        }
        Ok(())
    }
    async fn download_client(&self) -> Result<()> {
        download_binaries(true, false).await?;
        let path = Path::new(CLIENT_DEST_PATH);
        if path.exists() {
            fs::remove_file(path)?;
        }
        fs::rename("myceliald", CLIENT_DEST_PATH)?;
        Ok(())
    }
    fn check_client_database(&self) -> Result<()> {
        if Path::new(CLIENT_DB_PATH).exists() {
            let theme = ColorfulTheme::default();
            let confirm = Confirm::with_theme(&theme)
                .with_prompt("Overwrite existing daemon database?")
                .default(false)
                .interact()?;
            if confirm {
                fs::remove_file(CLIENT_DB_PATH)?;
            }
        }
        Ok(())
    }

    async fn configure_client(&self, config: Option<String>) -> Result<()> {
        fs::create_dir_all("/var/lib/mycelial")?;
        fs::create_dir_all("/etc/mycelial")?;
        let mut config_action: Option<ConfigAction> = None;
        if Path::new(CLIENT_CONFIG_PATH).exists() {
            let theme = ColorfulTheme::default();
            let confirm = Confirm::with_theme(&theme)
                .with_prompt("Overwrite existing configuration?")
                .default(false)
                .interact()?;
            if confirm {
                config_action = Some(ConfigAction::Create);
            } else {
                config_action = Some(ConfigAction::UseExisting);
            }
        }
        let database_storage_path = Some(CLIENT_DB_PATH.to_string());
        match config {
            Some(config) => {
                fs::copy(config, CLIENT_CONFIG_PATH)?;
            }
            None => {
                create_config(
                    CLIENT_CONFIG_PATH.to_string(),
                    database_storage_path,
                    config_action,
                    None,
                    None,
                )
                .await?;
            }
        }
        Ok(())
    }
    fn install_and_start(&self) -> Result<()> {
        let label: ServiceLabel = SERVICE_LABEL.parse()?;
        let manager = <dyn ServiceManager>::native().expect("Failed to detect management platform");
        manager
            .install(ServiceInstallCtx {
                label: label.clone(),
                program: PathBuf::from(CLIENT_DEST_PATH),
                args: vec![OsString::from(format!("--config={}", CLIENT_CONFIG_PATH))],
                contents: None, // Optional String for system-specific service content.
                username: None, // Optional String for alternative user to run service.
                working_directory: None, // Optional String for the working directory for the service process.
                environment: None, // Optional list of environment variables to supply the service process.
            })
            .expect("Failed to install");
        manager
            .start(ServiceStartCtx {
                label: label.clone(),
            })
            .expect("Failed to start");
        println!("Mycelial daemon installed and started");
        Ok(())
    }
    fn uninstall_client(&self) -> Result<()> {
        let label: ServiceLabel = SERVICE_LABEL.parse()?;
        let manager = <dyn ServiceManager>::native().expect("Failed to detect management platform");
        match manager.stop(ServiceStopCtx {
            label: label.clone(),
        }) {
            Ok(_) => println!("Mycelial daemon stopped"),
            Err(_) => println!("Mycelial daemon not running"),
        }
        manager
            .uninstall(ServiceUninstallCtx {
                label: label.clone(),
            })
            .expect("Failed to uninstall");
        println!("daemon service removed");
        Ok(())
    }
    fn purge_client(&self) -> Result<()> {
        fs::remove_file(CLIENT_CONFIG_PATH)?;
        println!("daemon configuration deleted {}", CLIENT_CONFIG_PATH);
        fs::remove_file(CLIENT_DEST_PATH)?;
        println!("daemon binary deleted {}", CLIENT_DEST_PATH);
        fs::remove_file(CLIENT_DB_PATH)?;
        println!("daemon database deleted {}", CLIENT_DB_PATH);
        Ok(())
    }
    pub fn status_client(&self) -> Result<()> {
        match std::env::consts::OS {
            "macos" => self.status_client_launchctrl()?,
            "linux" => self.status_client_systemd()?,
            _ => {}
        }
        Ok(())
    }
    fn status_client_launchctrl(&self) -> Result<()> {
        let mut is_first_line = true;
        let output = Command::new("launchctl")
            .arg("list")
            .stdout(Stdio::piped())
            .spawn()?
            .wait_with_output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if is_first_line {
                println!("{}", line);
                is_first_line = false;
            } else if line.contains(SERVICE_LABEL) {
                println!("{}", line);
            }
        }
        Ok(())
    }
    fn status_client_systemd(&self) -> Result<()> {
        let label: ServiceLabel = SERVICE_LABEL.parse()?;
        let script_name = label.to_script_name();
        let output = Command::new("systemctl")
            .arg("status")
            .arg(script_name)
            .stdout(Stdio::piped())
            .spawn()?
            .wait_with_output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            println!("{}", line);
        }
        Ok(())
    }
    pub fn start_client(&self) -> Result<()> {
        match std::env::consts::OS {
            "macos" => self.start_client_launchctrl()?,
            "linux" => self.start_client_systemd()?,
            _ => {}
        }
        Ok(())
    }
    // service_manager crate doesn't support launchctrl properly
    // so we're using the command line tool directly
    fn start_client_launchctrl(&self) -> Result<()> {
        let plist_path = format!("/Library/LaunchDaemons/{}.plist", SERVICE_LABEL);
        Command::new("launchctl")
            .arg("load")
            .arg(plist_path)
            .stdout(Stdio::piped())
            .spawn()?
            .wait()?;
        Ok(())
    }
    pub fn start_client_systemd(&self) -> Result<()> {
        let label: ServiceLabel = SERVICE_LABEL.parse()?;
        let manager = <dyn ServiceManager>::native().expect("Failed to detect management platform");
        manager
            .start(ServiceStartCtx {
                label: label.clone(),
            })
            .expect("Failed to start");
        Ok(())
    }
    pub fn stop_client(&self) -> Result<()> {
        match std::env::consts::OS {
            "macos" => self.stop_client_launchctrl()?,
            "linux" => self.stop_client_systemd()?,
            _ => {}
        }
        Ok(())
    }
    // service_manager crate doesn't support launchctrl properly
    // so we're using the command line tool directly
    fn stop_client_launchctrl(&self) -> Result<()> {
        let plist_path = format!("/Library/LaunchDaemons/{}.plist", SERVICE_LABEL);
        Command::new("launchctl")
            .arg("unload")
            .arg(plist_path)
            .stdout(Stdio::piped())
            .spawn()?
            .wait()?;
        Ok(())
    }
    fn stop_client_systemd(&self) -> Result<()> {
        let label: ServiceLabel = SERVICE_LABEL.parse()?;
        let manager = <dyn ServiceManager>::native().expect("Failed to detect management platform");
        manager
            .stop(ServiceStopCtx {
                label: label.clone(),
            })
            .expect("Failed to stop");
        Ok(())
    }
    pub fn restart_client(&self) -> Result<()> {
        match std::env::consts::OS {
            "macos" => {
                self.stop_client_launchctrl()?;
                self.start_client_launchctrl()?;
            }
            "linux" => {
                self.stop_client_systemd()?;
                self.start_client_systemd()?;
            }
            _ => {}
        }
        Ok(())
    }
}
