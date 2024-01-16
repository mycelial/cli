use assert_fs::prelude::*;
use rexpect::session::spawn_command;
use serde::Deserialize;
use std::error::Error;
use std::process::Command;

fn init_session() -> Result<rexpect::session::PtySession, Box<dyn Error>> {
    let bin_path = assert_cmd::cargo::cargo_bin("mycelial");
    let mut cmd = Command::new(bin_path);
    cmd.arg("init");
    let mut session = spawn_command(cmd, Some(1_000))?;
    session.exp_string("Client Name:")?;
    session.send_line("My Client")?;
    session.exp_string("Client ID:")?;
    session.send_line("my-client")?;
    session.exp_string("Server:")?;
    session.send_line("http://localhost:8080")?;
    session.exp_string("Security Token:")?;
    session.send_line("token")?;
    session.exp_string("What would you like to do?")?;
    Ok(session)
}

#[test]
fn cli_init_config_node_server() -> Result<(), Box<dyn Error>> {
    let temp_dir = assert_fs::TempDir::new()?;
    std::env::set_current_dir(&temp_dir)?;
    let mut session = init_session()?;

    session.send_line("exit")?;
    session.exp_eof()?;

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path())?;
    let parsed_config: toml::Value = toml::from_str(&config_file_contents)?;
    assert_eq!(
        parsed_config["node"]["display_name"].as_str().unwrap(),
        "My Client"
    );
    let unique_id = parsed_config["node"]["unique_id"].as_str().unwrap();
    assert!(unique_id.starts_with("my-client"));
    let storage_path = parsed_config["node"]["storage_path"].as_str().unwrap();
    assert_eq!(storage_path, "client.db");

    let endpoint = parsed_config["server"]["endpoint"].as_str().unwrap();
    assert_eq!(endpoint, "http://localhost:8080");

    let token = parsed_config["server"]["token"].as_str().unwrap();
    assert_eq!(token, "token");
    temp_dir.close()?;
    Ok(())
}
#[test]
fn cli_init_postgres_src() -> Result<(), Box<dyn Error>> {
    #[derive(Deserialize)]
    struct Config {
        sources: Vec<Source>,
    }

    #[derive(Deserialize)]
    struct Source {
        #[serde(rename = "type")]
        source_type: String,
        display_name: String,
        postgres_url: String,
        schema: String,
        tables: String,
        poll_interval: i32,
    }
    let temp_dir = assert_fs::TempDir::new()?;
    std::env::set_current_dir(&temp_dir)?;
    let mut session = init_session()?;
    session.send("Add Source")?;
    session.exp_string("Add Source")?;
    session.send_line("")?;
    session.exp_string("What type of source would you like to add?")?;
    session.send("Append only Postgres source")?;
    session.exp_string("Append only Postgres source")?;
    session.send_line("")?;
    session.exp_string("Display name:")?;
    session.send_line("Postgres Source")?;
    session.exp_string("Postgres username:")?;
    session.send_line("postgres_user")?;
    session.exp_string("Postgres password:")?;
    session.send_line("password")?;
    session.exp_string("Server address:")?;
    session.send_line("127.0.0.1")?;
    session.exp_string("Postgres port:")?;
    session.send_line("1000")?;
    session.exp_string("Database name:")?;
    session.send_line("mydb")?;
    session.exp_string("Schema:")?;
    session.send_line("public")?;
    session.exp_string("Tables:")?;
    session.send_line("table1,table2")?;
    session.exp_string("Poll interval (seconds):")?;
    session.send_line("10")?;
    session.send("Exit")?;
    session.exp_string("Exit")?;
    session.send_line("")?;
    session.exp_eof()?;

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path())?;
    let parsed: Config = toml::from_str(&config_file_contents)?;
    assert_eq!(parsed.sources.len(), 1);
    assert_eq!(parsed.sources[0].source_type, "postgres_connector");
    assert_eq!(parsed.sources[0].display_name, "Postgres Source");
    assert_eq!(
        parsed.sources[0].postgres_url,
        "postgres://postgres_user:password@127.0.0.1:1000/mydb"
    );
    assert_eq!(parsed.sources[0].schema, "public");
    assert_eq!(parsed.sources[0].tables, "table1,table2");
    assert_eq!(parsed.sources[0].poll_interval, 10);

    temp_dir.close()?;
    Ok(())
}

#[test]
fn cli_init_excel_src() -> Result<(), Box<dyn Error>> {
    #[derive(Deserialize)]
    struct Config {
        sources: Vec<Source>,
    }

    #[derive(Deserialize)]
    struct Source {
        #[serde(rename = "type")]
        source_type: String,
        display_name: String,
        path: String,
        sheets: String,
        strict: bool,
    }
    let temp_dir = assert_fs::TempDir::new()?;
    std::env::set_current_dir(&temp_dir)?;
    let mut session = init_session()?;
    session.send("Add Source")?;
    session.exp_string("Add Source")?;
    session.send_line("")?;
    session.exp_string("What type of source would you like to add?")?;
    session.send("Excel source")?;
    session.exp_string("Excel source")?;
    session.send_line("")?;
    session.exp_string("Display name:")?;
    session.send_line("Excel")?;
    session.exp_string("Excel Path:")?;
    session.send_line("some_file.xlsx")?;
    session.exp_string("Sheets:")?;
    session.send_line("*")?;
    session.exp_string("Strict:")?;
    session.send_line("y")?;
    session.send("Exit")?;
    session.exp_string("Exit")?;
    session.send_line("")?;
    session.exp_eof()?;

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path())?;
    let parsed: Config = toml::from_str(&config_file_contents)?;
    assert_eq!(parsed.sources.len(), 1);
    assert_eq!(parsed.sources[0].source_type, "excel_connector");
    assert_eq!(parsed.sources[0].display_name, "Excel");
    assert!(parsed.sources[0].path.ends_with("some_file.xlsx"));
    assert_eq!(parsed.sources[0].sheets, "*");
    assert_eq!(parsed.sources[0].strict, true);

    temp_dir.close()?;
    Ok(())
}

#[test]
fn cli_init_mycelite_src() -> Result<(), Box<dyn Error>> {
    #[derive(Deserialize)]
    struct Config {
        sources: Vec<Source>,
    }

    #[derive(Deserialize)]
    struct Source {
        #[serde(rename = "type")]
        source_type: String,
        display_name: String,
        journal_path: String,
    }
    let temp_dir = assert_fs::TempDir::new()?;
    std::env::set_current_dir(&temp_dir)?;
    let mut session = init_session()?;
    session.send("Add Source")?;
    session.exp_string("Add Source")?;
    session.send_line("")?;
    session.exp_string("What type of source would you like to add?")?;
    session.send("Full SQLite replication source")?;
    session.exp_string("Full SQLite replication source")?;
    session.send_line("")?;
    session.exp_string("Display name:")?;
    session.send_line("mycelite")?;
    session.exp_string("Journal Path:")?;
    session.send_line("mydata.db-mycelial")?;
    session.send("Exit")?;
    session.exp_string("Exit")?;
    session.send_line("")?;
    session.exp_eof()?;

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path())?;
    let parsed: Config = toml::from_str(&config_file_contents)?;
    assert_eq!(parsed.sources.len(), 1);
    assert_eq!(parsed.sources[0].source_type, "sqlite_physical_replication");
    assert_eq!(parsed.sources[0].display_name, "mycelite");
    assert!(parsed.sources[0]
        .journal_path
        .ends_with("mydata.db-mycelial"));

    temp_dir.close()?;
    Ok(())
}

#[test]
fn cli_init_append_only_sqlite_src() -> Result<(), Box<dyn Error>> {
    #[derive(Deserialize)]
    struct Config {
        sources: Vec<Source>,
    }

    #[derive(Deserialize)]
    struct Source {
        #[serde(rename = "type")]
        source_type: String,
        display_name: String,
        path: String,
    }
    let temp_dir = assert_fs::TempDir::new()?;
    std::env::set_current_dir(&temp_dir)?;
    let mut session = init_session()?;
    session.send("Add Source")?;
    session.exp_string("Add Source")?;
    session.send_line("")?;
    session.send_line("")?;
    session.exp_string("What type of source would you like to add?")?;
    session.send("Append only SQLite source")?;
    session.exp_string("Append only SQLite source")?;
    session.send_line("")?;
    session.exp_string("Display name:")?;
    session.send_line("sqlite")?;
    session.exp_string("Database Path:")?;
    session.send_line("data.db")?;
    session.send("Exit")?;
    session.exp_string("Exit")?;
    session.send_line("")?;
    session.exp_eof()?;

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path())?;
    let parsed: Config = toml::from_str(&config_file_contents)?;
    assert_eq!(parsed.sources.len(), 1);
    assert_eq!(parsed.sources[0].source_type, "sqlite_connector");
    assert_eq!(parsed.sources[0].display_name, "sqlite");
    assert!(parsed.sources[0].path.ends_with("data.db"));

    temp_dir.close()?;
    Ok(())
}

#[test]
fn cli_init_append_only_mycelite_dest() -> Result<(), Box<dyn Error>> {
    #[derive(Deserialize)]
    struct Config {
        destinations: Vec<Destination>,
    }

    #[derive(Deserialize)]
    struct Destination {
        #[serde(rename = "type")]
        destination_type: String,
        display_name: String,
        journal_path: String,
        database_path: String,
    }
    let temp_dir = assert_fs::TempDir::new()?;
    std::env::set_current_dir(&temp_dir)?;
    let mut session = init_session()?;
    session.send("Add Destination")?;
    session.exp_string("Add Destination")?;
    session.send_line("")?;
    session.exp_string("What type of destination would you like to add?")?;
    session.send("Full SQLite replication destination")?;
    session.exp_string("Full SQLite replication destination")?;
    session.send_line("")?;
    session.exp_string("Display name:")?;
    session.send_line("mycelite")?;
    session.exp_string("Journal Path:")?;
    session.send_line("data-mycelial")?;
    session.exp_string("Database Path:")?;
    session.send_line("destination.db")?;
    session.send("Exit")?;
    session.exp_string("Exit")?;
    session.send_line("")?;
    session.exp_eof()?;

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path())?;
    let parsed: Config = toml::from_str(&config_file_contents)?;
    assert_eq!(parsed.destinations.len(), 1);
    assert_eq!(
        parsed.destinations[0].destination_type,
        "sqlite_physical_replication"
    );
    assert_eq!(parsed.destinations[0].display_name, "mycelite");
    assert!(parsed.destinations[0]
        .journal_path
        .ends_with("data-mycelial"));
    assert!(parsed.destinations[0]
        .database_path
        .ends_with("destination.db"));

    temp_dir.close()?;
    Ok(())
}

#[test]
fn cli_init_append_only_sqlite_dest() -> Result<(), Box<dyn Error>> {
    #[derive(Deserialize)]
    struct Config {
        destinations: Vec<Destination>,
    }

    #[derive(Deserialize)]
    struct Destination {
        #[serde(rename = "type")]
        destination_type: String,
        display_name: String,
        path: String,
    }
    let temp_dir = assert_fs::TempDir::new()?;
    std::env::set_current_dir(&temp_dir)?;
    let mut session = init_session()?;
    session.send("Add Destination")?;
    session.exp_string("Add Destination")?;
    session.send_line("")?;
    session.exp_string("What type of destination would you like to add?")?;
    session.send("Append only SQLite destination")?;
    session.exp_string("Append only SQLite destination")?;
    session.send_line("")?;
    session.exp_string("Display name:")?;
    session.send_line("mycelite")?;
    session.exp_string("Database Path:")?;
    session.send_line("destination.db")?;
    session.send("Exit")?;
    session.exp_string("Exit")?;
    session.send_line("")?;
    session.exp_eof()?;

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path())?;
    let parsed: Config = toml::from_str(&config_file_contents)?;
    assert_eq!(parsed.destinations.len(), 1);
    assert_eq!(parsed.destinations[0].destination_type, "sqlite_connector");
    assert_eq!(parsed.destinations[0].display_name, "mycelite");
    assert!(parsed.destinations[0].path.ends_with("destination.db"));

    temp_dir.close()?;
    Ok(())
}

#[test]
fn cli_init_append_only_postgres_dest() -> Result<(), Box<dyn Error>> {
    #[derive(Deserialize)]
    struct Config {
        destinations: Vec<Destination>,
    }

    #[derive(Deserialize)]
    struct Destination {
        #[serde(rename = "type")]
        destination_type: String,
        display_name: String,
        url: String,
    }
    let temp_dir = assert_fs::TempDir::new()?;
    std::env::set_current_dir(&temp_dir)?;
    let mut session = init_session()?;
    session.send("Add Destination")?;
    session.exp_string("Add Destination")?;
    session.send_line("")?;
    session.exp_string("What type of destination would you like to add?")?;
    session.send("Append only Postgres destination")?;
    session.exp_string("Append only Postgres destination")?;
    session.send_line("")?;
    session.exp_string("Display name:")?;
    session.send_line("postgres")?;
    session.exp_string("Postgres username:")?;
    session.send_line("pguser")?;
    session.exp_string("Postgres password:")?;
    session.send_line("password")?;
    session.exp_string("Server address:")?;
    session.send_line("10.0.0.10")?;
    session.exp_string("Postgres port:")?;
    session.send_line("1234")?;
    session.exp_string("Database name:")?;
    session.send_line("mydb")?;
    session.send("Exit")?;
    session.exp_string("Exit")?;
    session.send_line("")?;
    session.exp_eof()?;

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path())?;
    let parsed: Config = toml::from_str(&config_file_contents)?;
    assert_eq!(parsed.destinations.len(), 1);
    assert_eq!(
        parsed.destinations[0].destination_type,
        "postgres_connector"
    );
    assert_eq!(parsed.destinations[0].display_name, "postgres");
    assert_eq!(
        parsed.destinations[0].url,
        "postgres://pguser:password@10.0.0.10:1234/mydb"
    );

    temp_dir.close()?;
    Ok(())
}

#[test]
fn cli_init_append_only_mysql_dest() -> Result<(), Box<dyn Error>> {
    #[derive(Deserialize)]
    struct Config {
        destinations: Vec<Destination>,
    }

    #[derive(Deserialize)]
    struct Destination {
        #[serde(rename = "type")]
        destination_type: String,
        display_name: String,
        url: String,
    }
    let temp_dir = assert_fs::TempDir::new()?;
    std::env::set_current_dir(&temp_dir)?;
    let mut session = init_session()?;
    session.send("Add Destination")?;
    session.exp_string("Add Destination")?;
    session.send_line("")?;
    session.exp_string("What type of destination would you like to add?")?;
    session.send("Append only MySQL destination")?;
    session.exp_string("Append only MySQL destination")?;
    session.send_line("")?;
    session.exp_string("Display name:")?;
    session.send_line("mysql")?;
    session.exp_string("Mysql username:")?;
    session.send_line("mysqluser")?;
    session.exp_string("Mysql password:")?;
    session.send_line("password")?;
    session.exp_string("Server address:")?;
    session.send_line("10.0.0.10")?;
    session.exp_string("Mysql port:")?;
    session.send_line("1234")?;
    session.exp_string("Database name:")?;
    session.send_line("mydb")?;
    session.send("Exit")?;
    session.exp_string("Exit")?;
    session.send_line("")?;
    session.exp_eof()?;

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path())?;
    let parsed: Config = toml::from_str(&config_file_contents)?;
    assert_eq!(parsed.destinations.len(), 1);
    assert_eq!(parsed.destinations[0].destination_type, "mysql_connector");
    assert_eq!(parsed.destinations[0].display_name, "mysql");
    assert_eq!(
        parsed.destinations[0].url,
        "mysql://mysqluser:password@10.0.0.10:1234/mydb"
    );

    temp_dir.close()?;
    Ok(())
}

#[test]
fn cli_init_kafka_dest() -> Result<(), Box<dyn Error>> {
    #[derive(Deserialize)]
    struct Config {
        destinations: Vec<Destination>,
    }

    #[derive(Deserialize)]
    struct Destination {
        #[serde(rename = "type")]
        destination_type: String,
        display_name: String,
        brokers: String,
        topic: String,
    }
    let temp_dir = assert_fs::TempDir::new()?;
    std::env::set_current_dir(&temp_dir)?;
    let mut session = init_session()?;
    session.send("Add Destination")?;
    session.exp_string("Add Destination")?;
    session.send_line("")?;
    session.exp_string("What type of destination would you like to add?")?;
    session.send("Kafka destination")?;
    session.exp_string("Kafka destination")?;
    session.send_line("")?;
    session.exp_string("Display name:")?;
    session.send_line("kafka")?;
    session.exp_string("Broker:")?;
    session.send_line("localhost:1000")?;
    session.exp_string("Topic")?;
    session.send_line("test-topic")?;
    session.send("Exit")?;
    session.exp_string("Exit")?;
    session.send_line("")?;
    session.exp_eof()?;

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path())?;
    let parsed: Config = toml::from_str(&config_file_contents)?;
    assert_eq!(parsed.destinations.len(), 1);
    assert_eq!(parsed.destinations[0].destination_type, "kafka");
    assert_eq!(parsed.destinations[0].display_name, "kafka");
    assert_eq!(parsed.destinations[0].brokers, "localhost:1000");
    assert_eq!(parsed.destinations[0].topic, "test-topic");

    temp_dir.close()?;
    Ok(())
}
