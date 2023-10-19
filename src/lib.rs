use colored::*;
use flate2::read::GzDecoder;
use os_info::Type;
use std::env::current_dir;
use std::fs::{self, read_to_string, remove_file, File};
use std::io::{Cursor, Write};
use std::process::Stdio;
use tar::Archive;
use toml::{map::Map, Value};
extern crate dirs;

use inquire::{required, Password, Select, Text};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub async fn init() -> Result<()> {
    println!("{}", "Initializing Mycelial".green());
    println!("{}", "Downloading binaries...".green());
    download_binaries().await?;
    println!(
        "{}",
        "Create a config file by answering the following questions.".green()
    );
    create_config().await?;
    Ok(())
}

pub async fn start() -> Result<()> {
    destroy().await?;
    do_start().await?;
    println!("{}", "Mycelial started!".green());
    println!("{}", "Running on `http://localhost:8080`".green());
    Ok(())
}

pub async fn destroy() -> Result<()> {
    println!("{}", "Destroying myceliald and server...".green());
    let pids = get_pids();
    for pid in pids {
        let pid_int = pid.parse::<i32>().unwrap();
        let result = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(pid_int),
            nix::sys::signal::SIGKILL,
        );
        match result {
            Ok(_) => {
                println!("killed pid {}", pid);
            }
            Err(_err) => {
                eprintln!("error killing pid {}", pid);
            }
        }
    }
    delete_pids_file()
}

fn delete_pids_file() -> Result<()> {
    let file_name = get_pid_file();
    let result = fs::remove_file(file_name);
    match result {
        Ok(_) => {
            println!("deleted pid file");
        }
        Err(_error) => {
            // pids file (~/.mycelial) may not exist, so ignore errors
        }
    }
    Ok(())
}

fn get_pid_file() -> String {
    let home_dir = dirs::home_dir().unwrap();
    let file_name = format!("{}/.mycelial", home_dir.display());
    file_name
}

fn get_pids() -> Vec<String> {
    let file_name = get_pid_file();
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

enum OS {
    LinuxX8664,
    LinuxARM64,
    LinuxARM32,
    MacOSX8664,
    MacOSARM64,
    Unknown,
}

fn get_os() -> OS {
    let info = os_info::get();
    match info.os_type() {
        Type::Macos => {
            if Some("arm64") == info.architecture() {
                OS::MacOSARM64
            } else {
                OS::MacOSX8664
            }
        }
        Type::Windows => OS::Unknown,
        Type::NetBSD
        | Type::FreeBSD
        | Type::OpenBSD
        | Type::DragonFly
        | Type::HardenedBSD
        | Type::MidnightBSD => OS::Unknown,
        _ => {
            if Some("arm64") == info.architecture() {
                OS::LinuxARM64
            } else if Some("arm32") == info.architecture() {
                OS::LinuxARM32
            } else {
                OS::LinuxX8664
            }
        }
    }
}

async fn download_binaries() -> Result<()> {
    match get_os() {
        OS::LinuxX8664 => {
            download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/server-x86_64-unknown-linux-gnu.tgz" , "server-x86_64-unknown-linux-gnu.tgz").await?;
            download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/myceliald-x86_64-unknown-linux-gnu.tgz", "myceliald-x86_64-unknown-linux-gnu.tgz").await?;
        }
        OS::LinuxARM64 => {
            download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/server-aarch64-unknown-linux-gnu.tgz" , "server-aarch64-unknown-linux-gnu.tgz").await?;
            download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/myceliald-aarch64-unknown-linux-gnu.tgz", "myceliald-aarch64-unknown-linux-gnu.tgz").await?;
        }
        OS::LinuxARM32 => {
            download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/server-arm-unknown-linux-gnueabihf.tgz" , "server-arm-unknown-linux-gnueabihf.tgz").await?;
            download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/myceliald-arm-unknown-linux-gnueabihf.tgz", "myceliald-arm-unknown-linux-gnueabihf.tgz").await?;
        }
        OS::MacOSX8664 => {
            download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/server-x86_64-apple-darwin.tgz", "server-x86_64-apple-darwin.tgz").await?;
            download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/myceliald-x86_64-apple-darwin.tgz", "myceliald-x86_64-apple-darwin.tgz").await?;
        }
        OS::MacOSARM64 => {
            download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/server-aarch64-apple-darwin.tgz", "server-aarch64-apple-darwin.tgz").await?;
            download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/myceliald-aarch64-apple-darwin.tgz", "myceliald-aarch64-apple-darwin.tgz").await?;
        }
        OS::Unknown => {
            panic!("Unknown OS");
        }
    }
    Ok(())
}

async fn do_start() -> Result<()> {
    println!("Starting Mycelial...");
    let server_log_file = File::create("server.log")?;
    let myceliald_log_file = File::create("myceliald.log")?;
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
    // println!("myceliald started with pid {:?}", client_process.id());
    let file_name = get_pid_file();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(file_name)?;
    file.write_all(format!("{}\n", server_process.id()).as_bytes())?;
    file.write_all(format!("{}\n", client_process.id()).as_bytes())?;
    Ok(())
}

pub async fn download_and_unarchive(url: &str, file_name: &str) -> Result<()> {
    print!("Downloading {}...", file_name);
    std::io::stdout().flush()?;
    let response = reqwest::get(url).await?;
    let mut file = std::fs::File::create(file_name)?;
    let mut content = Cursor::new(response.bytes().await?);
    std::io::copy(&mut content, &mut file)?;
    println!("done!");
    print!("Unarchiving {}...", file_name);
    std::io::stdout().flush()?;
    let tar_gz = File::open(file_name)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(".")?;
    println!("done!");
    remove_file(file_name)?;
    Ok(())
}

async fn create_config() -> Result<()> {
    let cwd = current_dir()?.into_os_string().into_string().unwrap();
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

    node_table.insert("display_name".into(), Value::String(client_name));
    node_table.insert("unique_id".into(), Value::String(client_id));
    node_table.insert("storage_path".into(), Value::String("client.sqlite".into()));

    tables.insert("node".into(), Value::Table(node_table));

    let server = Text::new("Server:")
        .with_default("http://localhost:8080")
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
        let options = vec!["Add Source", "Add Destination", "Exit"];
        let answer = Select::new("What would you like to do?", options).prompt()?;
        if answer == "Exit" {
            tables.insert("sources".into(), Value::Array(sources));
            tables.insert("destinations".into(), Value::Array(destinations));
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
        } else if answer == "Add Source" {
            let options = vec!["Full SQLite replication source"];
            let source =
                Select::new("What type of source would you like to add?", options).prompt()?;
            match source {
                "Full SQLite replication source" => {
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
                }
                _ => {
                    panic!("Unknown source type");
                }
            }
        } else if answer == "Add Destination" {
            let options = vec!["Full SQLite replication destination"];
            let destination =
                Select::new("What type of destination would you like to add?", options).prompt()?;
            match destination {
                "Full SQLite replication destination" => {
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
                }
                _ => {
                    panic!("Unknown destination type");
                }
            }
        }
    }
    Ok(())
}
