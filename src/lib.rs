use colored::*;
use flate2::read::GzDecoder;
use std::env::current_dir;
use std::fs::{self, read_to_string, remove_file, File};
use std::io::{Cursor, Write};
use std::path::Path;
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
    if can_start() {
        println!(
            "{}",
            "You must run `mycelial --local init` before `mycelial start`".red()
        );
        return Ok(());
    }
    destroy().await?;
    do_start().await?;
    println!("{}", "Mycelial started!".green());
    println!("{}", "Running on `http://localhost:7777`".green());
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

async fn download_binaries() -> Result<()> {
    match std::env::consts::OS {
        "linux" => match std::env::consts::ARCH {
            "x86_64" => {
                download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/server-x86_64-unknown-linux-gnu.tgz" , "server-x86_64-unknown-linux-gnu.tgz").await?;
                download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/myceliald-x86_64-unknown-linux-gnu.tgz", "myceliald-x86_64-unknown-linux-gnu.tgz").await?;
            }
            "aarch64" => {
                download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/server-aarch64-unknown-linux-gnu.tgz" , "server-aarch64-unknown-linux-gnu.tgz").await?;
                download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/myceliald-aarch64-unknown-linux-gnu.tgz", "myceliald-aarch64-unknown-linux-gnu.tgz").await?;
            }
            "arm" => {
                download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/server-arm-unknown-linux-gnueabihf.tgz" , "server-arm-unknown-linux-gnueabihf.tgz").await?;
                download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/myceliald-arm-unknown-linux-gnueabihf.tgz", "myceliald-arm-unknown-linux-gnueabihf.tgz").await?;
            }
            _ => {
                panic!("Unsupported architecture");
            }
        },
        "macos" => match std::env::consts::ARCH {
            "x86_64" => {
                download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/server-x86_64-apple-darwin.tgz", "server-x86_64-apple-darwin.tgz").await?;
                download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/myceliald-x86_64-apple-darwin.tgz", "myceliald-x86_64-apple-darwin.tgz").await?;
            }
            "aarch64" => {
                download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/server-aarch64-apple-darwin.tgz", "server-aarch64-apple-darwin.tgz").await?;
                download_and_unarchive("https://github.com/mycelial/mycelial/releases/latest/download/myceliald-aarch64-apple-darwin.tgz", "myceliald-aarch64-apple-darwin.tgz").await?;
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

    node_table.insert("display_name".into(), Value::String(client_name));
    node_table.insert("unique_id".into(), Value::String(client_id));
    node_table.insert("storage_path".into(), Value::String("client.sqlite".into()));

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
            let options = vec![
                MYCELITE_DESTINATION,
                SQLITE_DESTINATION,
                POSTGRES_DESTINATION,
                MYSQL_DESTINATION,
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
                _ => {
                    panic!("Unknown destination type");
                }
            }
        }
    }
    Ok(())
}

fn can_start() -> bool {
    let server_path = Path::new("server");
    let myceliald_path = Path::new("myceliald");
    let config_path = Path::new("config.toml");
    !server_path.exists() || !myceliald_path.exists() || !config_path.exists()
}
