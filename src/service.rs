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
        let database_storage_path = Some("/var/lib/mycelial".to_string());
        let mycelial_dir_path = Path::new("/etc/mycelial");
        if !mycelial_dir_path.exists() {
            fs::create_dir(mycelial_dir_path)?;
        }
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
        let label: ServiceLabel = "com.mycelial.myceliald".parse()?;
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
}
