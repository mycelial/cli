use mycelial::{create_config, download_binaries};
use service_manager::*;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub struct Service {}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
const CLIENT_DEST_PATH: &str = "/usr/local/bin/myceliald";
const CLIENT_CONFIG_PATH: &str = "/etc/mycelial/config.toml";
const CLIENT_DB_PATH: &str = "/var/lib/mycelial";
const SERVICE_LABEL: &str = "com.mycelial.myceliald";
impl Service {
    pub fn new() -> Service {
        Service {}
    }
    pub async fn add_client(&self, config: Option<String>) -> Result<()> {
        self.download_client().await?;
        self.configure_client(config).await?;
        self.install_and_start()?;
        Ok(())
    }
    pub async fn remove_client(&self) -> Result<()> {
        self.uninstall_client()?;
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
    async fn configure_client(&self, config: Option<String>) -> Result<()> {
        fs::create_dir_all(CLIENT_DB_PATH)?;
        fs::create_dir_all("/etc/mycelial")?;
        let database_storage_path = Some(format!("{}/client.db", CLIENT_DB_PATH));
        match config {
            Some(config) => {
                fs::copy(config, CLIENT_CONFIG_PATH)?;
            }
            None => {
                create_config(CLIENT_CONFIG_PATH.to_string(), database_storage_path).await?;
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
        println!("Mycelial client installed and started");
        Ok(())
    }
    fn uninstall_client(&self) -> Result<()> {
        let label: ServiceLabel = SERVICE_LABEL.parse()?;
        let manager = <dyn ServiceManager>::native().expect("Failed to detect management platform");
        match manager.stop(ServiceStopCtx {
            label: label.clone(),
        }) {
            Ok(_) => println!("Myceliald client stopped"),
            Err(_) => println!("Myceliald client not running"),
        }
        manager
            .uninstall(ServiceUninstallCtx {
                label: label.clone(),
            })
            .expect("Failed to uninstall");
        println!("Myceliald client removed");
        Ok(())
    }
}
