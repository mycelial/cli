use colored::*;
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use serde::Deserialize;
use std::cmp::min;
use std::env::current_dir;
use std::fmt;
use std::fs::{self, read_to_string, remove_file, File};
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use tar::Archive;
use toml::{map::Map, Value};
use uuid::Uuid;
extern crate dirs;

use inquire::{required, Confirm, Password, Select, Text};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

enum Executable {
    Server,
    Client,
}

#[derive(Deserialize)]
struct Node {
    storage_path: String,
}

#[derive(Deserialize)]
struct Config {
    node: Node,
}

pub async fn init(client: bool, server: bool) -> Result<()> {
    println!("{}", "Initializing Mycelial".green());
    download_binaries(client, server).await?;
    println!(
        "{}",
        "Create a config file by answering the following questions.".green()
    );
    create_config().await?;
    Ok(())
}

pub async fn start(client: bool, server: bool) -> Result<()> {
    destroy(client, server).await?;
    if server {
        if !can_start_server() {
            println!(
                "{}",
                "Missing server binary. You must run `mycelial --local init` before `mycelial start`".red()
            );
            return Ok(());
        }
        start_server().await?;
        println!("{}", "Server started on `http://localhost:7777`".green());
    }
    if client {
        if !can_start_client() {
            println!(
                "{}",
                "Missing myceliald binary. You must run `mycelial --local init` before `mycelial start`".red()
            );
            return Ok(());
        }
        start_client().await?;
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

fn storage_path() -> Result<String> {
    let config_path = Path::new("config.toml");
    if !config_path.exists() {
        return Err("config.toml does not exist".into());
    }
    let config_string = read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_string)?;
    Ok(config.node.storage_path)
}

pub async fn reset(client: bool, server: bool) -> Result<()> {
    let answer = Confirm::new("Are you sure you want to reset Mycelial?")
        .with_default(false)
        .with_help_message("This deletes all local state (sqlite databases)")
        .prompt()?;
    if answer {
        let client_db_path = storage_path()?;
        if client {
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

async fn download_binaries(client: bool, server: bool) -> Result<()> {
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
    let token = Password::new("Enter Security Token:")
        .with_validator(required!("This field is required"))
        .with_help_message("Token")
        .prompt()?;

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

async fn start_client() -> Result<()> {
    println!("Starting myceliald (client)...");
    let myceliald_log_file = File::create("myceliald.log")?;
    let client_process = match std::process::Command::new("./myceliald")
        .arg("--config")
        .arg("config.toml")
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

fn prompt_sqlite_source(sources: &mut Vec<Value>) -> Result<()> {
    let cwd = current_dir()?.into_os_string().into_string().unwrap();
    let name = Text::new("Display name:")
        .with_default("SQLite Append Only Source")
        .with_validator(required!("This field is required"))
        .with_help_message("Display Name")
        .prompt()?;
    let path = Text::new("Database Path:")
        .with_default("data.db")
        .with_validator(required!("This field is required"))
        .with_help_message("Database path")
        .prompt()?;
    let mut source_table = Map::new();
    source_table.insert(
        "type".to_string(),
        Value::String("sqlite_connector".to_string()),
    );
    source_table.insert("display_name".to_string(), Value::String(name));
    source_table.insert(
        "path".to_string(),
        Value::String(format!("{}/{}", cwd, path)),
    );
    sources.push(Value::Table(source_table));
    Ok(())
}

fn prompt_mycelite_source(sources: &mut Vec<Value>) -> Result<()> {
    let cwd = current_dir()?.into_os_string().into_string().unwrap();
    let name = Text::new("Display name:")
        .with_default("Example Source")
        .with_validator(required!("This field is required"))
        .with_help_message("Display Name")
        .prompt()?;
    let path = Text::new("Journal Path:")
        .with_default("data.db-mycelial")
        .with_validator(required!("This field is required"))
        .with_help_message("Journal path")
        .prompt()?;
    let mut source_table = Map::new();
    source_table.insert(
        "type".to_string(),
        Value::String("sqlite_physical_replication".to_string()),
    );
    source_table.insert("display_name".to_string(), Value::String(name));
    source_table.insert(
        "journal_path".to_string(),
        Value::String(format!("{}/{}", cwd, path)),
    );
    sources.push(Value::Table(source_table));
    Ok(())
}

fn prompt_mycelite_destination(destinations: &mut Vec<Value>) -> Result<()> {
    let cwd = current_dir()?.into_os_string().into_string().unwrap();
    let name = Text::new("Display name:")
        .with_default("Example Destination")
        .with_validator(required!("This field is required"))
        .with_help_message("Display Name")
        .prompt()?;
    let journal_path = Text::new("Journal Path:")
        .with_default("destination-sqlite-mycelial")
        .with_validator(required!("This field is required"))
        .with_help_message("Journal path")
        .prompt()?;
    let database_path = Text::new("Database Path:")
        .with_default("destination-sqlite.data")
        .with_validator(required!("This field is required"))
        .with_help_message("Database path and filename")
        .prompt()?;
    let mut destination_table = Map::new();
    destination_table.insert(
        "type".to_string(),
        Value::String("sqlite_physical_replication".to_string()),
    );
    destination_table.insert("display_name".to_string(), Value::String(name));
    destination_table.insert(
        "journal_path".to_string(),
        Value::String(format!("{}/{}", cwd, journal_path)),
    );
    destination_table.insert(
        "database_path".to_string(),
        Value::String(format!("{}/{}", cwd, database_path)),
    );
    destinations.push(Value::Table(destination_table));
    Ok(())
}

fn prompt_sqlite_destination(destinations: &mut Vec<Value>) -> Result<()> {
    let cwd = current_dir()?.into_os_string().into_string().unwrap();
    let name = Text::new("Display name:")
        .with_default("SQLite Append Only Destination")
        .with_validator(required!("This field is required"))
        .with_help_message("Display Name")
        .prompt()?;
    let path = Text::new("Database Path:")
        .with_default("destination.db")
        .with_validator(required!("This field is required"))
        .with_help_message("Database path")
        .prompt()?;
    let mut source_table = Map::new();
    source_table.insert(
        "type".to_string(),
        Value::String("sqlite_connector".to_string()),
    );
    source_table.insert("display_name".to_string(), Value::String(name));
    source_table.insert(
        "path".to_string(),
        Value::String(format!("{}/{}", cwd, path)),
    );
    destinations.push(Value::Table(source_table));
    Ok(())
}

fn prompt_postgres_destination(destinations: &mut Vec<Value>) -> Result<()> {
    let name = Text::new("Display name:")
        .with_default("Postgres Append Only Destination")
        .with_validator(required!("This field is required"))
        .with_help_message("Display Name")
        .prompt()?;
    let user = Text::new("Postgres username:")
        .with_default("user")
        .with_validator(required!("This field is required"))
        .with_help_message("Postgres Username")
        .prompt()?;
    let password = Password::new("Postgres password:")
        .with_validator(required!("This field is required"))
        .with_help_message("Password")
        .prompt()?;
    let address = Text::new("Server address:")
        .with_default("127.0.0.1")
        .with_validator(required!("This field is required"))
        .with_help_message("Server address")
        .prompt()?;
    let port = Text::new("Postgres port:")
        .with_default("5432")
        .with_validator(required!("This field is required"))
        .with_help_message("Postgres port")
        .prompt()?;
    let database = Text::new("Database name:")
        .with_default("db")
        .with_validator(required!("This field is required"))
        .with_help_message("Database name")
        .prompt()?;
    let postgres_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        user, password, address, port, database
    );
    let mut destination_table = Map::new();
    destination_table.insert(
        "type".to_string(),
        Value::String("postgres_connector".to_string()),
    );
    destination_table.insert("display_name".to_string(), Value::String(name));
    destination_table.insert("url".to_string(), Value::String(postgres_url));
    destinations.push(Value::Table(destination_table));
    Ok(())
}

fn prompt_kafka_destination(destinations: &mut Vec<Value>) -> Result<()> {
    let name = Text::new("Display name:")
        .with_default("Kafka Destination")
        .with_validator(required!("This field is required"))
        .with_help_message("Display Name")
        .prompt()?;
    let broker = Text::new("Broker:")
        .with_default("localhost:9092")
        .with_validator(required!("This field is required"))
        .with_help_message("Broker")
        .prompt()?;
    let topic = Text::new("Topic:")
        .with_default("test")
        .with_validator(required!("This field is required"))
        .with_help_message("Topic")
        .prompt()?;
    let mut destination_table = Map::new();
    destination_table.insert("type".to_string(), Value::String("kafka".to_string()));
    destination_table.insert("display_name".to_string(), Value::String(name));
    destination_table.insert("broker".to_string(), Value::String(broker));
    destination_table.insert("topic".to_string(), Value::String(topic));
    destinations.push(Value::Table(destination_table));
    Ok(())
}

fn prompt_mysql_destination(destinations: &mut Vec<Value>) -> Result<()> {
    let name = Text::new("Display name:")
        .with_default("Mysql Append Only Destination")
        .with_validator(required!("This field is required"))
        .with_help_message("Display Name")
        .prompt()?;
    let user = Text::new("Mysql username:")
        .with_default("user")
        .with_validator(required!("This field is required"))
        .with_help_message("Postgres Username")
        .prompt()?;
    let password = Password::new("mysql password:")
        .with_validator(required!("This field is required"))
        .with_help_message("Password")
        .prompt()?;
    let address = Text::new("Server address:")
        .with_default("127.0.0.1")
        .with_validator(required!("This field is required"))
        .with_help_message("Server address")
        .prompt()?;
    let port = Text::new("Mysql port:")
        .with_default("3306")
        .with_validator(required!("This field is required"))
        .with_help_message("Mysql port")
        .prompt()?;
    let database = Text::new("Database name:")
        .with_default("db")
        .with_validator(required!("This field is required"))
        .with_help_message("Database name")
        .prompt()?;
    let postgres_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        user, password, address, port, database
    );
    let mut destination_table = Map::new();
    destination_table.insert(
        "type".to_string(),
        Value::String("mysql_connector".to_string()),
    );
    destination_table.insert("display_name".to_string(), Value::String(name));
    destination_table.insert("url".to_string(), Value::String(postgres_url));
    destinations.push(Value::Table(destination_table));
    Ok(())
}

async fn create_config() -> Result<()> {
    let mut tables = Map::new();
    let mut node_table = Map::new();
    let client_name = Text::new("Client Name:")
        .with_default("My Client")
        .with_validator(required!("This field is required"))
        .with_help_message("Client display name")
        .prompt()?;

    let client_id = Text::new("Client ID:")
        .with_default("client")
        .with_validator(required!("This field is required"))
        .with_help_message("Client ID")
        .prompt()?;

    let id = Uuid::new_v4().to_string();

    node_table.insert("display_name".into(), Value::String(client_name));
    node_table.insert(
        "unique_id".into(),
        Value::String(format!("{}-{}", client_id, id)),
    );
    node_table.insert("storage_path".into(), Value::String("client.db".into()));

    tables.insert("node".into(), Value::Table(node_table));

    let server = Text::new("Server:")
        .with_default("http://localhost:7777")
        .with_validator(required!("This field is required"))
        .with_help_message("Server address")
        .prompt()?;

    let token = Password::new("Security Token:")
        .with_validator(required!("This field is required"))
        .with_help_message("Token")
        .prompt()?;
    let mut server_table = Map::new();
    server_table.insert("endpoint".into(), Value::String(server));
    server_table.insert("token".into(), Value::String(token));
    tables.insert("server".into(), Value::Table(server_table));

    let mut sources: Vec<Value> = Vec::new();
    let mut destinations: Vec<Value> = Vec::new();
    loop {
        const ADD_SOURCE: &str = "Add Source";
        const ADD_DESTINATION: &str = "Add Destination";
        const EXIT: &str = "Exit";
        let options = vec![ADD_SOURCE, ADD_DESTINATION, EXIT];
        let answer = Select::new("What would you like to do?", options).prompt()?;
        if answer == EXIT {
            if sources.len() > 0 {
                tables.insert("sources".into(), Value::Array(sources));
            }
            if destinations.len() > 0 {
                tables.insert("destinations".into(), Value::Array(destinations));
            }
            let toml_string =
                toml::to_string(&Value::Table(tables)).expect("Could not encode TOML value");
            let result = fs::write("config.toml", toml_string);
            match result {
                Ok(_) => {
                    println!("{}", "config.toml created!".green());
                    println!("{}", "Run `mycelial start` to start Mycelial".green());
                }
                Err(_error) => {
                    return Err("error creating config.toml".into());
                }
            }
            break;
        } else if answer == ADD_SOURCE {
            const MYCELITE_SOURCE: &str = "Full SQLite replication source";
            const SQLITE_SOURCE: &str = "Append only SQLite source";
            let options = vec![MYCELITE_SOURCE, SQLITE_SOURCE];
            let source =
                Select::new("What type of source would you like to add?", options).prompt()?;
            match source {
                MYCELITE_SOURCE => {
                    prompt_mycelite_source(&mut sources)?;
                }
                SQLITE_SOURCE => {
                    prompt_sqlite_source(&mut sources)?;
                }
                _ => {
                    panic!("Unknown source type");
                }
            }
        } else if answer == ADD_DESTINATION {
            const MYCELITE_DESTINATION: &str = "Full SQLite replication destination";
            const SQLITE_DESTINATION: &str = "Append only SQLite destination";
            const POSTGRES_DESTINATION: &str = "Append only Postgres destination";
            const MYSQL_DESTINATION: &str = "Append only MySQL destination";
            const KAFKA_DESTINATION: &str = "Kafka destination";
            let options = vec![
                MYCELITE_DESTINATION,
                SQLITE_DESTINATION,
                POSTGRES_DESTINATION,
                MYSQL_DESTINATION,
                KAFKA_DESTINATION,
            ];
            let destination =
                Select::new("What type of destination would you like to add?", options).prompt()?;
            match destination {
                MYCELITE_DESTINATION => {
                    prompt_mycelite_destination(&mut destinations)?;
                }
                SQLITE_DESTINATION => {
                    prompt_sqlite_destination(&mut destinations)?;
                }
                POSTGRES_DESTINATION => {
                    prompt_postgres_destination(&mut destinations)?;
                }
                MYSQL_DESTINATION => {
                    prompt_mysql_destination(&mut destinations)?;
                }
                KAFKA_DESTINATION => {
                    prompt_kafka_destination(&mut destinations)?;
                }
                _ => {
                    panic!("Unknown destination type");
                }
            }
        }
    }
    Ok(())
}

fn can_start_client() -> bool {
    let myceliald_path = Path::new("myceliald");
    let config_path = Path::new("config.toml");
    myceliald_path.exists() && config_path.exists()
}

fn can_start_server() -> bool {
    let server_path = Path::new("server");
    server_path.exists()
}
