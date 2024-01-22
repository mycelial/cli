use colored::*;
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use std::cmp::min;
use std::env::current_dir;
use std::fmt;
use std::fs::{self, read_to_string, remove_file, File};
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use tar::Archive;
use uuid::Uuid;
extern crate dirs;
mod config;
use config::Config as Configuration;
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input, Password};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

enum Executable {
    Server,
    Client,
}

pub async fn init(client: bool, server: bool, config_file_name: String) -> Result<()> {
    println!("{}", "Initializing Mycelial".green());
    download_binaries(client, server).await?;
    println!(
        "{}",
        "Create a config file by answering the following questions.".green()
    );
    create_config(config_file_name, None, None).await?;
    Ok(())
}

pub async fn start(client: bool, server: bool, config_file_name: String) -> Result<()> {
    destroy(client, server).await?;
    if server {
        if !can_start_server() {
            println!(
                "{}",
                "Missing server binary. You must run `mycelial init --local` before `mycelial start`".red()
            );
            return Ok(());
        }
        start_server().await?;
        println!("{}", "Server started on `http://localhost:7777`".green());
    }
    if client {
        if !can_start_client(&config_file_name) {
            println!(
                "{}",
                "Missing myceliald binary or config file. You must run `mycelial init --local` before `mycelial start`".red()
            );
            return Ok(());
        }
        start_client(config_file_name).await?;
        println!("{}", "Myceliald (client) started!".green());
    }
    Ok(())
}

pub async fn destroy(client: bool, server: bool) -> Result<()> {
    if client {
        let pids = get_pids(Executable::Client);
        for pid in pids {
            let pid_int = pid.parse::<i32>().unwrap();
            let result = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid_int),
                nix::sys::signal::SIGKILL,
            );
            match result {
                Ok(_) => {
                    println!("killed client pid {}", pid);
                }
                Err(_err) => {
                    eprintln!("error killing client pid {}", pid);
                }
            }
        }
        delete_pids_file(Executable::Client)?;
    }
    if server {
        let pids = get_pids(Executable::Server);
        for pid in pids {
            let pid_int = pid.parse::<i32>().unwrap();
            let result = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid_int),
                nix::sys::signal::SIGKILL,
            );
            match result {
                Ok(_) => {
                    println!("killed server pid {}", pid);
                }
                Err(_err) => {
                    eprintln!("error killing server pid {}", pid);
                }
            }
        }
        delete_pids_file(Executable::Server)?;
    }
    Ok(())
}

fn storage_path(config_file_name: &str) -> Option<String> {
    match Configuration::load(config_file_name) {
        Ok(config) => {
            return config.get_node_storage_path();
        }
        Err(_error) => return None,
    }
}

pub async fn reset(client: bool, server: bool, config_file_name: &str) -> Result<()> {
    let answer: bool = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Are you sure you want to reset Mycelial?")
        .interact()
        .unwrap();
    if answer {
        if client {
            let client_db_path =
                storage_path(config_file_name).expect("Could not find config.toml");
            let result = remove_file(&client_db_path);
            match result {
                Ok(_) => {
                    println!("{}", format!("{} deleted", client_db_path).green());
                }
                Err(_error) => {
                    println!("{}", format!("{} does not exists", client_db_path).yellow());
                }
            }
        }
        if server {
            let result = remove_file("mycelial.db");
            match result {
                Ok(_) => {
                    println!("{}", "mycelial.db deleted!".green());
                }
                Err(_error) => {
                    println!("{}", "mycelial.db does not exists".yellow());
                }
            }
        }
    } else {
        println!("{}", "Reset cancelled".yellow());
    }
    Ok(())
}

fn delete_pids_file(executable: Executable) -> Result<()> {
    let file_name = get_pid_file(&executable);
    let result = fs::remove_file(&file_name);
    match result {
        Ok(_) => {
            let which = match executable {
                Executable::Server => "server",
                Executable::Client => "client",
            };
            println!("deleted {} pid file ({})", which, file_name);
        }
        Err(_error) => {
            // pids file (~/.mycelial) may not exist, so ignore errors
        }
    }
    Ok(())
}

fn get_pid_file(executable: &Executable) -> String {
    let home_dir = dirs::home_dir().unwrap();
    match executable {
        Executable::Server => format!("{}/.mycelial/server.pid", home_dir.display()),
        Executable::Client => format!("{}/.mycelial/myceliald.pid", home_dir.display()),
    }
}

fn get_pids(executable: Executable) -> Vec<String> {
    let file_name = get_pid_file(&executable);
    let mut pids = Vec::new();
    let result = read_to_string(file_name);
    match result {
        Ok(contents) => {
            for line in contents.lines() {
                pids.push(line.to_string())
            }
        }
        Err(_error) => {
            // pids file (~/.mycelial) may not exist, so ignore errors
        }
    }
    pids
}

fn create_pid_file_dir() -> Result<()> {
    let dir_name = format!("{}/.mycelial", dirs::home_dir().unwrap().display());
    let path = Path::new(&dir_name);
    fs::create_dir_all(path)?;
    Ok(())
}

fn save_pid(executable: Executable, pid: u32) -> Result<()> {
    create_pid_file_dir()?;
    let file_name = get_pid_file(&executable);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(file_name)?;
    file.write_all(format!("{}\n", pid).as_bytes())?;
    Ok(())
}

pub async fn download_binaries(client: bool, server: bool) -> Result<()> {
    if server && client {
        println!("Downloading and unarchiving server and myceliald (client)...");
    } else if server {
        println!("Downloading and unarchiving server...");
    } else if client {
        println!("Downloading and unarchiving myceliald (client)...");
    }
    match std::env::consts::OS {
        "linux" => match std::env::consts::ARCH {
            "x86_64" => {
                if server {
                    download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/server-x86_64-unknown-linux-gnu.tgz" , "server-x86_64-unknown-linux-gnu.tgz").await?;
                }
                if client {
                    download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/myceliald-x86_64-unknown-linux-gnu.tgz", "myceliald-x86_64-unknown-linux-gnu.tgz").await?;
                }
            }
            "aarch64" => {
                if server {
                    download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/server-aarch64-unknown-linux-gnu.tgz" , "server-aarch64-unknown-linux-gnu.tgz").await?;
                }
                if client {
                    download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/myceliald-aarch64-unknown-linux-gnu.tgz", "myceliald-aarch64-unknown-linux-gnu.tgz").await?;
                }
            }
            "arm" => {
                if server {
                    download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/server-arm-unknown-linux-gnueabihf.tgz" , "server-arm-unknown-linux-gnueabihf.tgz").await?;
                }
                if client {
                    download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/myceliald-arm-unknown-linux-gnueabihf.tgz", "myceliald-arm-unknown-linux-gnueabihf.tgz").await?;
                }
            }
            _ => {
                panic!("Unsupported architecture");
            }
        },
        "macos" => match std::env::consts::ARCH {
            "x86_64" => {
                if server {
                    download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/server-x86_64-apple-darwin.tgz", "server-x86_64-apple-darwin.tgz").await?;
                }
                if client {
                    download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/myceliald-x86_64-apple-darwin.tgz", "myceliald-x86_64-apple-darwin.tgz").await?;
                }
            }
            "aarch64" => {
                if server {
                    download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/server-aarch64-apple-darwin.tgz", "server-aarch64-apple-darwin.tgz").await?;
                }
                if client {
                    download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/myceliald-aarch64-apple-darwin.tgz", "myceliald-aarch64-apple-darwin.tgz").await?;
                }
            }
            _ => {
                panic!("Unsupported architecture");
            }
        },
        _ => {
            panic!("Unsupported OS");
        }
    }
    Ok(())
}

async fn start_server() -> Result<()> {
    println!("Starting Mycelial Server...");
    let server_log_file = File::create("server.log")?;
    let token = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Security Token:")
        .interact()
        .unwrap();

    let server_process = match std::process::Command::new("./server")
        .arg("--token")
        .arg(token)
        .stdin(Stdio::null())
        .stdout(Stdio::from(
            server_log_file.try_clone().expect("Could not clone file"),
        ))
        .stderr(Stdio::from(server_log_file))
        .spawn()
    {
        Ok(process) => process,
        Err(e) => panic!("failed to execute process: {}", e),
    };
    save_pid(Executable::Server, server_process.id())?;
    Ok(())
}

async fn start_client(config_file_name: String) -> Result<()> {
    println!(
        "Starting myceliald (client) with config file {}...",
        config_file_name
    );
    let myceliald_log_file = File::create("myceliald.log")?;
    let client_process = match std::process::Command::new("./myceliald")
        .arg("--config")
        .arg(config_file_name)
        .stdin(Stdio::null())
        .stdout(Stdio::from(
            myceliald_log_file
                .try_clone()
                .expect("Could not clone file"),
        ))
        .stderr(Stdio::from(myceliald_log_file))
        .spawn()
    {
        Ok(process) => process,
        Err(e) => panic!("failed to execute process: {}", e),
    };
    save_pid(Executable::Client, client_process.id())?;
    Ok(())
}
pub async fn download_and_unarchive(url: &str, file_name: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let mut response = client.get(url).send().await?;
    let mut file = File::create(file_name)?;
    let mut downloaded: u64 = 0;
    let length = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(length);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn fmt::Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk)?;
        let new = min(downloaded + chunk.len() as u64, length);
        downloaded = new;
        pb.set_position(new);
    }
    pb.finish_with_message("download complete");
    let tar_gz = File::open(file_name)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(".")?;
    remove_file(file_name)?;
    Ok(())
}

fn prompt_sqlite_source(config: &mut Configuration) -> Result<()> {
    let cwd = current_dir()?.into_os_string().into_string().unwrap();
    let display_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Display name:")
        .default("SQLite Append Only Source".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Database Path:")
        .default("data.db".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let database_path = format!("{}/{}", cwd, path);
    config.add_sqlite_connector_source(display_name, database_path);
    Ok(())
}

fn prompt_mycelite_source(config: &mut Configuration) -> Result<()> {
    let cwd = current_dir()?.into_os_string().into_string().unwrap();
    let display_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Display name:")
        .default("Example Source".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Journal Path:")
        .default("data.db-mycelial".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let journal_path = format!("{}/{}", cwd, path);
    config.add_sqlite_physical_replication_source(display_name, journal_path);
    Ok(())
}

fn prompt_mycelite_destination(config: &mut Configuration) -> Result<()> {
    let cwd = current_dir()?.into_os_string().into_string().unwrap();
    let display_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Display name:")
        .default("Example Destination".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Journal Path:")
        .default("destination-sqlite-mycelial".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let database_path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Database Path:")
        .default("destination-sqlite.data".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let journal_path = format!("{}/{}", cwd, path);
    config.add_sqlite_physical_replication_destination(display_name, journal_path, database_path);
    Ok(())
}

fn prompt_sqlite_destination(config: &mut Configuration) -> Result<()> {
    let cwd = current_dir()?.into_os_string().into_string().unwrap();
    let display_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Display name:")
        .default("SQLite Append Only Destination".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Database Path:")
        .default("destination.db".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let database_path = format!("{}/{}", cwd, path);
    config.add_sqlite_connector_destination(display_name, database_path);
    Ok(())
}

fn prompt_postgres_destination(config: &mut Configuration) -> Result<()> {
    let display_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Display name:")
        .default("Postgres Append Only Destination".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let user: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Postgres username:")
        .default("user".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let password = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Postgres password:")
        .interact()
        .unwrap();

    let address: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Server address:")
        .default("127.0.0.1".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let port: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Postgres port:")
        .default("5432".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();

    let database: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Database name:")
        .default("db".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let postgres_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        user, password, address, port, database
    );
    config.add_postgres_connector_destination(display_name, postgres_url);

    Ok(())
}

fn prompt_kafka_destination(config: &mut Configuration) -> Result<()> {
    let display_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Display name:")
        .default("Kafka Destination".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let brokers: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Broker:")
        .default("localhost:9092".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let topic: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Topic:")
        .default("test".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    config.add_kafka_destination(display_name, brokers, topic);
    Ok(())
}

fn prompt_postgres_source(config: &mut Configuration) -> Result<()> {
    let display_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Display name:")
        .default("Postgres Source".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let user: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Postgres username:")
        .default("user".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let password = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Postgres password:")
        .interact()
        .unwrap();
    let address: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Server address:")
        .default("localhost".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let port: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Postgres port:")
        .default("5432".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let database: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Database name:")
        .default("test".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let schema: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Schema:")
        .default("public".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let tables: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Tables:")
        .default("*".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let poll_interval: i32 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Poll interval (seconds):")
        .default(5)
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let postgres_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        user, password, address, port, database
    );
    config.add_postgres_connector_source(display_name, postgres_url, schema, tables, poll_interval);
    Ok(())
}

fn prompt_excel_source(config: &mut Configuration) -> Result<()> {
    let display_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Display name:")
        .default("Excel Source".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Excel Path:")
        .default("data.xlsx".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let sheets: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Sheets:")
        .default("*".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let strict: bool = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Strict:")
        .default(false)
        .interact()
        .unwrap();
    config.add_excel_connector_source(display_name, path, sheets, strict);
    Ok(())
}
fn prompt_mysql_source(config: &mut Configuration) -> Result<()> {
    let display_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Display name:")
        .default("MySQL Source".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let user: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("MySQL username:")
        .default("user".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let password = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("MySQL password:")
        .interact()
        .unwrap();
    let address: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Server address:")
        .default("localhost".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let port: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("MySQL port:")
        .default("3306".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let database: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Database name:")
        .default("test".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let schema: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Schema:")
        .default("public".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let tables: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Tables:")
        .default("*".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let poll_interval: i32 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Poll interval (seconds):")
        .default(5)
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let mysql_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        user, password, address, port, database
    );
    config.add_mysql_connector_source(display_name, mysql_url, schema, tables, poll_interval);
    Ok(())
}

fn prompt_mysql_destination(config: &mut Configuration) -> Result<()> {
    let display_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Display name:")
        .default("MySQL Append Only Destination".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let user: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("MySQL username:")
        .default("user".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let password = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("MySQL password:")
        .interact()
        .unwrap();
    let address: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Server address:")
        .default("127.0.0.1".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let port: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("MySQL port:")
        .default("3306".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let database: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Database name:")
        .default("db".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let postgres_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        user, password, address, port, database
    );
    config.add_mysql_connector_destination(display_name, postgres_url);
    Ok(())
}
fn prompt_file_source(config: &mut Configuration) -> Result<()> {
    let display_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Display name:")
        .default("file source".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Path:")
        .default("file.txt".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    config.add_file_source(display_name, path);
    Ok(())
}

fn prompt_file_destination(config: &mut Configuration) -> Result<()> {
    let display_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Display name:")
        .default("file destination".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Path:")
        .default("file.txt".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    config.add_file_destination(display_name, path);
    Ok(())
}

fn prompt_snowflake_destination(config: &mut Configuration) -> Result<()> {
    let display_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Display name:")
        .default("Snowflake Destination".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let username: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Snowflake username:")
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let password = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Snowflake password:")
        .interact()
        .unwrap();
    let role: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Snowflake role:")
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let account_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Snowflake account name:")
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let organization_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Snowflake organization name:")
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let warehouse: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Snowflake warehouse:")
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let database: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Database name:")
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let schema: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Schema:")
        .allow_empty(false)
        .interact_text()
        .unwrap();
    let account_identifier = format!("{}-{}", organization_name, account_name);
    config.add_snowflake_connector_destination(
        display_name,
        username,
        password,
        role,
        account_identifier,
        warehouse,
        database,
        schema,
    );
    Ok(())
}

pub enum ConfigAction {
    Create,
    Append,
    UseExisting,
}

fn config_file_action(config_file_name: String) -> Result<(ConfigAction, std::string::String)> {
    let config_path = Path::new(&config_file_name);
    const OVERWRITE: &str = "Overwrite file";
    const APPEND: &str = "Append to file";
    const RENAME: &str = "Rename file";
    let options = vec![OVERWRITE, APPEND, RENAME];
    if !config_path.exists() {
        return Ok((ConfigAction::Create, config_file_name));
    } else {
        let answer = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt(&format!(
                "The config file `{}` already exists, what would you like to do?",
                config_file_name
            ))
            .items(&options)
            .interact()
            .unwrap();
        match answer {
            // OVERWRITE
            0 => {
                return Ok((ConfigAction::Create, config_file_name));
            }
            // APPEND
            1 => {
                return Ok((ConfigAction::Append, config_file_name));
            }
            // RENAME
            2 => {
                let new_config_file_name: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("New config file name:")
                    .default("config.toml".to_string())
                    .allow_empty(false)
                    .interact_text()
                    .unwrap();
                let result = config_file_action(new_config_file_name)?;
                return Ok(result);
            }
            _ => {
                panic!("Unknown config file action");
            }
        }
    }
}

pub async fn create_config(
    config_file_name: String,
    database_storage_path: Option<String>,
    config_action: Option<ConfigAction>,
) -> Result<()> {
    let (action, config_file_name) = if config_action.is_none() {
        config_file_action(config_file_name)?
    } else {
        (config_action.unwrap(), config_file_name)
    };
    match action {
        ConfigAction::Create => {
            return do_create_config(config_file_name, database_storage_path).await;
        }
        ConfigAction::Append => {
            return do_append_config(config_file_name).await;
        }
        ConfigAction::UseExisting => {
            return Ok(());
        }
    }
}

async fn do_append_config(config_file_name: String) -> Result<()> {
    match Configuration::load(&config_file_name) {
        Ok(mut config) => {
            source_destination_loop(&mut config, config_file_name)?;
        }
        Err(_error) => {
            panic!("error loading config file");
        }
    }
    Ok(())
}

async fn do_create_config(
    config_file_name: String,
    database_storage_path: Option<String>,
) -> Result<()> {
    let database_storage_path = match database_storage_path {
        Some(database_storage_path) => database_storage_path,
        None => "client.db".to_string(),
    };
    let mut config = Configuration::new();
    let client_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Client Name:")
        .default("My Client".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();

    let client_id: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Client ID:")
        .default("client".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();

    let id = Uuid::new_v4().to_string();

    let unique_id = format!("{}-{}", client_id, id);

    config.set_node(client_name, unique_id, database_storage_path);

    let server: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Server:")
        .default("http://localhost:7777".to_string())
        .allow_empty(false)
        .interact_text()
        .unwrap();

    let token = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Security Token:")
        .interact()
        .unwrap();

    config.set_server(server, token);

    source_destination_loop(&mut config, config_file_name)?;
    Ok(())
}

fn source_prompts(config: &mut Configuration, config_file_name: Option<String>) -> Result<()> {
    const MYCELITE_SOURCE: &str = "Full SQLite replication source";
    const SQLITE_SOURCE: &str = "Append only SQLite source";
    const EXCEL_SOURCE: &str = "Excel source";
    const POSTGRES_SOURCE: &str = "Append only Postgres source";
    const MYSQL_SOURCE: &str = "Append only MySQL source";
    const FILE_SOURCE: &str = "File source";
    const EXIT: &str = "Exit";
    const PROMPT: &str = "What type of source would you like to add?";
    match config_file_name {
        Some(config_file_name) => {
            let options = vec![
                MYCELITE_SOURCE,
                SQLITE_SOURCE,
                EXCEL_SOURCE,
                POSTGRES_SOURCE,
                MYSQL_SOURCE,
                FILE_SOURCE,
                EXIT,
            ];
            let source = FuzzySelect::with_theme(&ColorfulTheme::default())
                .with_prompt(PROMPT)
                .items(&options)
                .interact()
                .unwrap();
            match source {
                // MYCELITE_SOURCE
                0 => {
                    prompt_mycelite_source(config)?;
                }
                // SQLITE_SOURCE
                1 => {
                    prompt_sqlite_source(config)?;
                }
                // EXCEL_SOURCE
                2 => {
                    prompt_excel_source(config)?;
                }
                // POSTGRES_SOURCE
                3 => {
                    prompt_postgres_source(config)?;
                }
                // MYSQL_SOURCE
                4 => {
                    prompt_mysql_source(config)?;
                }
                // FILE_SOURCE
                5 => {
                    prompt_file_source(config)?;
                }
                // EXIT
                6 => {
                    match config.save(&config_file_name) {
                        Ok(_) => {
                            println!("{}", format!("{} updated!", config_file_name).green());
                        }
                        Err(_error) => {
                            return Err(format!(
                                "error creating config file `{}`",
                                config_file_name
                            )
                            .into());
                        }
                    }
                    return Ok(());
                }
                _ => {
                    panic!("Unknown source type");
                }
            }
            source_prompts(config, Some(config_file_name))?;
        }
        None => {
            let options = vec![
                MYCELITE_SOURCE,
                SQLITE_SOURCE,
                EXCEL_SOURCE,
                POSTGRES_SOURCE,
                MYSQL_SOURCE,
                FILE_SOURCE,
            ];
            let source = FuzzySelect::with_theme(&ColorfulTheme::default())
                .with_prompt(PROMPT)
                .items(&options)
                .interact()
                .unwrap();
            match source {
                // MYCELITE_SOURCE
                0 => {
                    prompt_mycelite_source(config)?;
                }
                // SQLITE_SOURCE
                1 => {
                    prompt_sqlite_source(config)?;
                }
                // EXCEL_SOURCE
                2 => {
                    prompt_excel_source(config)?;
                }
                // POSTGRES_SOURCE
                3 => {
                    prompt_postgres_source(config)?;
                }
                // MYSQL_SOURCE
                4 => {
                    prompt_mysql_source(config)?;
                }
                5 => {
                    prompt_file_source(config)?;
                }
                _ => {
                    panic!("Unknown source type");
                }
            }
        }
    }
    Ok(())
}

fn destination_prompts(config: &mut Configuration, config_file_name: Option<String>) -> Result<()> {
    const MYCELITE_DESTINATION: &str = "Full SQLite replication destination";
    const SQLITE_DESTINATION: &str = "Append only SQLite destination";
    const POSTGRES_DESTINATION: &str = "Append only Postgres destination";
    const MYSQL_DESTINATION: &str = "Append only MySQL destination";
    const KAFKA_DESTINATION: &str = "Kafka destination";
    const SNOWFLAKE_DESTINATION: &str = "Snowflake destination";
    const FILE_DESTINATION: &str = "File destination";
    const EXIT: &str = "Exit";
    const PROMPT: &str = "What type of destination would you like to add?";
    match config_file_name {
        Some(config_file_name) => {
            let options = vec![
                MYCELITE_DESTINATION,
                SQLITE_DESTINATION,
                POSTGRES_DESTINATION,
                MYSQL_DESTINATION,
                KAFKA_DESTINATION,
                SNOWFLAKE_DESTINATION,
                FILE_DESTINATION,
                EXIT,
            ];
            let destination = FuzzySelect::with_theme(&ColorfulTheme::default())
                .with_prompt(PROMPT)
                .items(&options)
                .interact()
                .unwrap();
            match destination {
                // MYCELITE_DESTINATION
                0 => {
                    prompt_mycelite_destination(config)?;
                }
                // SQLITE_DESTINATION
                1 => {
                    prompt_sqlite_destination(config)?;
                }
                // POSTGRES_DESTINATION
                2 => {
                    prompt_postgres_destination(config)?;
                }
                // MYSQL_DESTINATION
                3 => {
                    prompt_mysql_destination(config)?;
                }
                // KAFKA_DESTINATION
                4 => {
                    prompt_kafka_destination(config)?;
                }
                // SNOWFLAKE_DESTINATION
                5 => {
                    prompt_snowflake_destination(config)?;
                }
                // FILE_DESTINATION
                6 => {
                    prompt_file_destination(config)?;
                }
                // EXIT
                7 => {
                    match config.save(&config_file_name) {
                        Ok(_) => {
                            println!("{}", "config file updated!".green());
                        }
                        Err(_error) => {
                            return Err(format!(
                                "error creating config file `{}`",
                                config_file_name
                            )
                            .into());
                        }
                    }
                    return Ok(());
                }
                _ => {
                    panic!("Unknown destination type");
                }
            }
            destination_prompts(config, Some(config_file_name))?;
        }
        None => {
            let options = vec![
                MYCELITE_DESTINATION,
                SQLITE_DESTINATION,
                POSTGRES_DESTINATION,
                MYSQL_DESTINATION,
                KAFKA_DESTINATION,
                SNOWFLAKE_DESTINATION,
                FILE_DESTINATION,
            ];
            let destination = FuzzySelect::with_theme(&ColorfulTheme::default())
                .with_prompt(PROMPT)
                .items(&options)
                .interact()
                .unwrap();
            match destination {
                // MYCELITE_DESTINATION
                0 => {
                    prompt_mycelite_destination(config)?;
                }
                // SQLITE_DESTINATION
                1 => {
                    prompt_sqlite_destination(config)?;
                }
                // POSTGRES_DESTINATION
                2 => {
                    prompt_postgres_destination(config)?;
                }
                // MYSQL_DESTINATION
                3 => {
                    prompt_mysql_destination(config)?;
                }
                // KAFKA_DESTINATION
                4 => {
                    prompt_kafka_destination(config)?;
                }
                // SNOWFLAKE_DESTINATION
                5 => {
                    prompt_snowflake_destination(config)?;
                }
                // FILE_DESTINATION
                6 => {
                    prompt_file_destination(config)?;
                }

                _ => {
                    panic!("Unknown destination type");
                }
            }
        }
    }
    Ok(())
}

pub async fn add_source(config_file_name: &str) -> Result<()> {
    let config_file_name_path = Path::new(config_file_name);
    if config_file_name_path.exists() {
        match Configuration::load(&config_file_name) {
            Ok(mut config) => {
                source_prompts(&mut config, Some(config_file_name.to_string()))?;
            }
            Err(_error) => {
                panic!("error loading config file");
            }
        }
    } else {
        create_config(config_file_name.to_string(), None, None).await?;
    }
    Ok(())
}

pub async fn add_destination(config_file_name: &str) -> Result<()> {
    let config_file_name_path = Path::new(config_file_name);
    if config_file_name_path.exists() {
        match Configuration::load(&config_file_name) {
            Ok(mut config) => {
                destination_prompts(&mut config, Some(config_file_name.to_string()))?;
            }
            Err(_error) => {
                panic!("error loading config file");
            }
        }
    } else {
        create_config(config_file_name.to_string(), None, None).await?;
    }
    Ok(())
}

fn source_destination_loop(config: &mut Configuration, config_file_name: String) -> Result<()> {
    loop {
        const ADD_SOURCE: &str = "Add Source";
        const ADD_DESTINATION: &str = "Add Destination";
        const EXIT: &str = "Exit";
        const PROMPT: &str = "What would you like to do?";
        let options = vec![ADD_SOURCE, ADD_DESTINATION, EXIT];
        let answer = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt(PROMPT)
            .items(&options)
            .interact()
            .unwrap();
        // EXIT
        if answer == 2 {
            match config.save(&config_file_name) {
                Ok(_) => {
                    println!("{}", format!("{} updated!", config_file_name).green());
                }
                Err(_error) => {
                    return Err("error creating config file".into());
                }
            }
            break;
        } else if answer == 0
        /* ADD_SOURCE */
        {
            source_prompts(config, None)?;
        } else if answer == 1
        /* ADD_DESTINATION */
        {
            destination_prompts(config, None)?;
        }
    }
    Ok(())
}

fn can_start_client(config_file_name: &str) -> bool {
    let myceliald_path = Path::new("myceliald");
    let config_path = Path::new(config_file_name);
    myceliald_path.exists() && config_path.exists()
}

fn can_start_server() -> bool {
    let server_path = Path::new("server");
    server_path.exists()
}
