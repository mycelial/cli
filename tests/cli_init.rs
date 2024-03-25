use assert_fs::prelude::*;
use rexpect::session::spawn_command;
use serde::Deserialize;
use std::process::Command;


// current test suite can be runned only sequentially
// prevent race condition between std::env::set_current_dir
fn lock<'a>() -> std::sync::MutexGuard<'a, ()>{
    static GLOBAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // we don't care about mutex poisoning
    match GLOBAL_LOCK.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

fn init_session() -> rexpect::session::PtySession {
    let bin_path = assert_cmd::cargo::cargo_bin("mycelial");
    let mut cmd = Command::new(bin_path);
    cmd.arg("init");
    let mut session = spawn_command(cmd, Some(1_000)).unwrap();
    session.exp_string("Daemon Name:").unwrap();
    session.send_line("My Daemon").unwrap();
    session.exp_string("Daemon ID:").unwrap();
    session.send_line("my-daemon").unwrap();
    session.exp_string("Control Plane:").unwrap();
    session.send_line("http://localhost:8080").unwrap();
    session.exp_string("Auth Token:").unwrap();
    session.send_line("token").unwrap();
    session.exp_string("What would you like to do?").unwrap();
    session
}

#[test]
fn cli_init_config_node_server() {
    let _guard = lock();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();
    let mut session = init_session();

    session.send_line("exit").unwrap();
    session.exp_eof().unwrap();

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path()).unwrap();
    let parsed_config: toml::Value = toml::from_str(&config_file_contents).unwrap();
    assert_eq!(
        parsed_config["node"]["display_name"].as_str().unwrap(),
        "My Daemon"
    );
    let unique_id = parsed_config["node"]["unique_id"].as_str().unwrap();
    assert!(unique_id.starts_with("my-daemon"));
    let storage_path = parsed_config["node"]["storage_path"].as_str().unwrap();
    assert_eq!(storage_path, "daemon.db");

    let endpoint = parsed_config["server"]["endpoint"].as_str().unwrap();
    assert_eq!(endpoint, "http://localhost:8080");

    let token = parsed_config["node"]["auth_token"].as_str().unwrap();
    assert_eq!(token, "token");
    temp_dir.close().unwrap();
}

#[test]
fn cli_init_postgres_src() {
    let _guard = lock();
    #[derive(Deserialize)]
    struct Config {
        sources: Vec<Source>,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct Source {
        r#type: String,
        display_name: String,
        url: String,
        origin: String,
        query: String,
        poll_interval: i32,
    }
    let temp_dir = assert_fs::TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();
    let mut session = init_session();

    session.send("Add Source").unwrap();
    session.exp_string("Add Source").unwrap();
    session.send_line("").unwrap();
    session.exp_string("What type of source would you like to add?").unwrap();
    session.send("Postgres source").unwrap();
    session.exp_string("Postgres source").unwrap();
    session.send_line("").unwrap();
    session.exp_string("Display name:").unwrap();
    session.send_line("Postgres Source").unwrap();
    session.exp_string("Postgres username:").unwrap();
    session.send_line("postgres_user").unwrap();
    session.exp_string("Postgres password:").unwrap();
    session.send_line("password").unwrap();
    session.exp_string("Server address:").unwrap();
    session.send_line("127.0.0.1").unwrap();
    session.exp_string("Postgres port:").unwrap();
    session.send_line("1000").unwrap();
    session.exp_string("Database name:").unwrap();
    session.send_line("mydb").unwrap();
    session.exp_string("Origin:").unwrap();
    session.send_line("origin").unwrap();
    session.exp_string("Query:").unwrap();
    session.send_line("select * from schema.test_table").unwrap();
    session.exp_string("Poll interval (seconds):").unwrap();
    session.send_line("10").unwrap();
    session.send("Exit").unwrap();
    session.exp_string("Exit").unwrap();
    session.send_line("").unwrap();
    session.exp_eof().unwrap();

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path()).unwrap();
    let parsed: Config = toml::from_str(&config_file_contents).unwrap();
    assert_eq!(
        parsed.sources,
        vec![
            Source {
                r#type: "postgres_connector".into(),
                display_name: "Postgres Source".into(),
                url: "postgres://postgres_user:password@127.0.0.1:1000/mydb".into(),
                origin: "origin".into(),
                query: "select * from schema.test_table".into(),
                poll_interval: 10,
            },
        ]
    );
    temp_dir.close().unwrap();
}

#[test]
fn cli_init_snowflake_dest() {
    let _guard = lock();
    #[derive(Deserialize)]
    struct Config {
        destinations: Vec<Destination>,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct Destination {
        r#type: String,
        display_name: String,
        username: String,
        password: String,
        role: String,
        account_identifier: String,
        warehouse: String,
        database: String,
        schema: String,
        truncate: bool,
    }
    let temp_dir = assert_fs::TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();
    let mut session = init_session();
    session.send("Add Destination").unwrap();
    session.exp_string("Add Destination").unwrap();
    session.send_line("").unwrap();
    session.exp_string("What type of destination would you like to add?").unwrap();
    session.send("Snowflake destination").unwrap();
    session.exp_string("Snowflake destination").unwrap();
    session.send_line("").unwrap();
    session.exp_string("Display name:").unwrap();
    session.send_line("Snowflake Destination").unwrap();
    session.exp_string("Snowflake username:").unwrap();
    session.send_line("username").unwrap();
    session.exp_string("Snowflake password:").unwrap();
    session.send_line("secret").unwrap();
    session.exp_string("Snowflake role:").unwrap();
    session.send_line("admin").unwrap();
    session.exp_string("Snowflake account name:").unwrap();
    session.send_line("myaccount").unwrap();
    session.exp_string("Snowflake organization name:").unwrap();
    session.send_line("myorg").unwrap();
    session.exp_string("Snowflake warehouse:").unwrap();
    session.send_line("whse").unwrap();
    session.exp_string("Database name:").unwrap();
    session.send_line("mydb").unwrap();
    session.exp_string("Schema:").unwrap();
    session.send_line("myschema").unwrap();
    session.exp_string("Truncate:").unwrap();
    session.send_line("false").unwrap();

    session.exp_string("What would you like to do?").unwrap();
    session.send("Exit").unwrap();
    session.exp_string("Exit").unwrap();
    session.send_line("").unwrap();
    session.exp_eof().unwrap();

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path()).unwrap();
    let parsed: Config = toml::from_str(&config_file_contents).unwrap();
    assert_eq!(
        parsed.destinations,
        vec![
            Destination{
                r#type: "snowflake".into(),
                display_name: "Snowflake Destination".into(),
                username: "username".into(),
                password: "secret".into(),
                role: "admin".into(),
                account_identifier: "myorg-myaccount".into(),
                warehouse: "whse".into(),
                database: "mydb".into(),
                schema: "myschema".into(),
                truncate: false,
            }
        ]
    );
    temp_dir.close().unwrap();
}

#[test]
fn cli_init_file_dest() {
    let _guard = lock();
    #[derive(Deserialize)]
    struct Config {
        destinations: Vec<Destinations>,
    }

    #[derive(Deserialize)]
    struct Destinations {
        #[serde(rename = "type")]
        destination_type: String,
        display_name: String,
        path: String,
    }
    let temp_dir = assert_fs::TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();
    let mut session = init_session();
    session.send("Add Destination").unwrap();
    session.exp_string("Add Destination").unwrap();
    session.send_line("").unwrap();
    session.exp_string("What type of destination would you like to add?").unwrap();
    session.send("File destination").unwrap();
    session.exp_string("File destination").unwrap();
    session.send_line("").unwrap();
    session.exp_string("Display name:").unwrap();
    session.send_line("my file dest").unwrap();
    session.exp_string("Path:").unwrap();
    session.send_line("my_file.txt").unwrap();
    session.send("Exit").unwrap();
    session.exp_string("Exit").unwrap();
    session.send_line("").unwrap();
    session.exp_eof().unwrap();

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path()).unwrap();
    let parsed: Config = toml::from_str(&config_file_contents).unwrap();
    assert_eq!(parsed.destinations.len(), 1);
    assert_eq!(parsed.destinations[0].destination_type, "file");
    assert_eq!(parsed.destinations[0].display_name, "my file dest");
    assert_eq!(parsed.destinations[0].path, "my_file.txt");

    temp_dir.close().unwrap();
}

#[test]
fn cli_init_file_src() {
    let _guard = lock();
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
    let temp_dir = assert_fs::TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();
    let mut session = init_session();
    session.send("Add Source").unwrap();
    session.exp_string("Add Source").unwrap();
    session.send_line("").unwrap();
    session.exp_string("What type of source would you like to add?").unwrap();
    session.send("File source").unwrap();
    session.exp_string("File source").unwrap();
    session.send_line("").unwrap();
    session.exp_string("Display name:").unwrap();
    session.send_line("my file source").unwrap();
    session.exp_string("Path:").unwrap();
    session.send_line("my_file.txt").unwrap();
    session.send("Exit").unwrap();
    session.exp_string("Exit").unwrap();
    session.send_line("").unwrap();
    session.exp_eof().unwrap();

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path()).unwrap();
    let parsed: Config = toml::from_str(&config_file_contents).unwrap();
    assert_eq!(parsed.sources.len(), 1);
    assert_eq!(parsed.sources[0].source_type, "file");
    assert_eq!(parsed.sources[0].display_name, "my file source");
    assert_eq!(parsed.sources[0].path, "my_file.txt");

    temp_dir.close().unwrap();
}

#[test]
fn cli_init_mysql_src() {
    let _guard = lock();
    #[derive(Deserialize)]
    struct Config {
        sources: Vec<Source>,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct Source {
        r#type: String,
        display_name: String,
        url: String,
        origin: String,
        query: String,
        poll_interval: i32,
    }
    let temp_dir = assert_fs::TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();
    let mut session = init_session();
    session.send("Add Source").unwrap();
    session.exp_string("Add Source").unwrap();
    session.send_line("").unwrap();
    session.exp_string("What type of source would you like to add?").unwrap();
    session.send("MySQL source").unwrap();
    session.exp_string("MySQL source").unwrap();
    session.send_line("").unwrap();
    session.exp_string("Display name:").unwrap();
    session.send_line("Mysql Source").unwrap();
    session.exp_string("MySQL username:").unwrap();
    session.send_line("mysql_user").unwrap();
    session.exp_string("MySQL password:").unwrap();
    session.send_line("password").unwrap();
    session.exp_string("Server address:").unwrap();
    session.send_line("127.0.0.1").unwrap();
    session.exp_string("MySQL port:").unwrap();
    session.send_line("1000").unwrap();
    session.exp_string("Database name:").unwrap();
    session.send_line("mydb").unwrap();
    session.exp_string("Origin:").unwrap();
    session.send_line("origin").unwrap();
    session.exp_string("Query:").unwrap();
    session.send_line("select * from some_table").unwrap();
    session.exp_string("Poll interval (seconds):").unwrap();
    session.send_line("10").unwrap();
    session.send("Exit").unwrap();
    session.exp_string("Exit").unwrap();
    session.send_line("").unwrap();
    session.exp_eof().unwrap();

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path()).unwrap();
    let parsed: Config = toml::from_str(&config_file_contents).unwrap();

    assert_eq!(
        parsed.sources,
        vec![
            Source {
                r#type: "mysql_connector".into(),
                display_name: "Mysql Source".into(),
                url: "mysql://mysql_user:password@127.0.0.1:1000/mydb".into(),
                origin: "origin".into(),
                query: "select * from some_table".into(),
                poll_interval: 10,
            }
        ]
    );
    temp_dir.close().unwrap();
}

#[test]
fn cli_init_excel_src() {
    let _guard = lock();
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
    let temp_dir = assert_fs::TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();
    let mut session = init_session();
    session.send("Add Source").unwrap();
    session.exp_string("Add Source").unwrap();
    session.send_line("").unwrap();
    session.exp_string("What type of source would you like to add?").unwrap();
    session.send("Excel source").unwrap();
    session.exp_string("Excel source").unwrap();
    session.send_line("").unwrap();
    session.exp_string("Display name:").unwrap();
    session.send_line("Excel").unwrap();
    session.exp_string("Excel Path:").unwrap();
    session.send_line("some_file.xlsx").unwrap();
    session.exp_string("Sheets:").unwrap();
    session.send_line("*").unwrap();
    session.exp_string("Strict:").unwrap();
    session.send_line("y").unwrap();
    session.send("Exit").unwrap();
    session.exp_string("Exit").unwrap();
    session.send_line("").unwrap();
    session.exp_eof().unwrap();

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path()).unwrap();
    let parsed: Config = toml::from_str(&config_file_contents).unwrap();
    assert_eq!(parsed.sources.len(), 1);
    assert_eq!(parsed.sources[0].source_type, "excel_connector");
    assert_eq!(parsed.sources[0].display_name, "Excel");
    assert!(parsed.sources[0].path.ends_with("some_file.xlsx"));
    assert_eq!(parsed.sources[0].sheets, "*");
    assert_eq!(parsed.sources[0].strict, true);

    temp_dir.close().unwrap();
}

#[test]
fn cli_init_sqlite_src() {
    let _guard = lock();
    #[derive(Deserialize)]
    struct Config {
        sources: Vec<Source>,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct Source {
        r#type: String,
        display_name: String,
        origin: String,
        path: String,
        query: String,
    }
    let temp_dir = assert_fs::TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();
    let mut session = init_session();
    session.send("Add Source").unwrap();
    session.exp_string("Add Source").unwrap();
    session.send_line("").unwrap();
    session.exp_string("What type of source would you like to add?").unwrap();
    session.send("SQLite source").unwrap();
    session.exp_string("SQLite source").unwrap();
    session.send_line("").unwrap();
    session.exp_string("Display name:").unwrap();
    session.send_line("sqlite").unwrap();
    session.exp_string("Origin").unwrap();
    session.send_line("origin").unwrap();
    session.exp_string("Database Path:").unwrap();
    session.send_line("data.db").unwrap();
    session.exp_string("Query").unwrap();
    session.send_line("select * from some_table").unwrap();
    session.send("Exit").unwrap();
    session.exp_string("Exit").unwrap();
    session.send_line("").unwrap();
    session.exp_eof().unwrap();

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path()).unwrap();
    let parsed: Config = toml::from_str(&config_file_contents).unwrap();
    assert_eq!(
        parsed.sources,
        vec![
            Source {
                r#type: "sqlite_connector".into(),
                display_name: "sqlite".into(),
                origin: "origin".into(),
                query: "select * from some_table".into(),
                path: "data.db".into(),
            },
        ]
    );
    temp_dir.close().unwrap();
}

#[test]
fn cli_init_sqlite_dest() {
    let _guard = lock();
    #[derive(Deserialize)]
    struct Config {
        destinations: Vec<Destination>,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct Destination {
        r#type: String,
        display_name: String,
        path: String,
        truncate: bool,
    }
    let temp_dir = assert_fs::TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();
    let mut session = init_session();
    session.send("Add Destination").unwrap();
    session.exp_string("Add Destination").unwrap();
    session.send_line("").unwrap();
    session.exp_string("What type of destination would you like to add?").unwrap();
    session.send("SQLite destination").unwrap();
    session.exp_string("SQLite destination").unwrap();
    session.send_line("").unwrap();
    session.exp_string("Display name:").unwrap();
    session.send_line("sqlite destination").unwrap();
    session.exp_string("Database Path:").unwrap();
    session.send_line("destination.db").unwrap();
    session.exp_string("Truncate:").unwrap();
    session.send_line("true").unwrap();
    session.send("Exit").unwrap();
    session.exp_string("Exit").unwrap();
    session.send_line("").unwrap();
    session.exp_eof().unwrap();

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path()).unwrap();
    let parsed: Config = toml::from_str(&config_file_contents).unwrap();
    assert_eq!(
        parsed.destinations, 
        vec![
            Destination{
                r#type: "sqlite_connector".into(),
                display_name: "sqlite destination".into(),
                path: "destination.db".into(),
                truncate: true,
            }
        ]
    );
    temp_dir.close().unwrap();
}

#[test]
fn cli_init_postgres_dest() {
    let _guard = lock();
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
    let temp_dir = assert_fs::TempDir::new().unwrap();

    std::env::set_current_dir(&temp_dir).unwrap();
    let mut session = init_session();
    session.send("Add Destination").unwrap();
    session.exp_string("Add Destination").unwrap();
    session.send_line("").unwrap();
    session.exp_string("What type of destination would you like to add?").unwrap();
    session.send("Postgres destination").unwrap();
    session.exp_string("Postgres destination").unwrap();
    session.send_line("").unwrap();
    session.exp_string("Display name:").unwrap();
    session.send_line("postgres").unwrap();
    session.exp_string("Postgres username:").unwrap();
    session.send_line("pguser").unwrap();
    session.exp_string("Postgres password:").unwrap();
    session.send_line("password").unwrap();
    session.exp_string("Server address:").unwrap();
    session.send_line("10.0.0.10").unwrap();
    session.exp_string("Postgres port:").unwrap();
    session.send_line("1234").unwrap();
    session.exp_string("Database name:").unwrap();
    session.send_line("mydb").unwrap();
    session.exp_string("Truncate:").unwrap();
    session.send_line("true").unwrap();
    session.send("Exit").unwrap();
    session.exp_string("Exit").unwrap();
    session.send_line("").unwrap();
    session.exp_eof().unwrap();

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path()).unwrap();
    let parsed: Config = toml::from_str(&config_file_contents).unwrap();
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

    temp_dir.close().unwrap();
}

#[test]
fn cli_init_mysql_dest() {
    let _guard = lock();
    #[derive(Deserialize)]
    struct Config {
        destinations: Vec<Destination>,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct Destination {
        r#type: String,
        display_name: String,
        url: String,
        truncate: bool,
    }
    let temp_dir = assert_fs::TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();
    let mut session = init_session();
    session.send("Add Destination").unwrap();
    session.exp_string("Add Destination").unwrap();
    session.send_line("").unwrap();
    session.exp_string("What type of destination would you like to add?").unwrap();
    session.send("MySQL destination").unwrap();
    session.exp_string("MySQL destination").unwrap();
    session.send_line("").unwrap();
    session.exp_string("Display name:").unwrap();
    session.send_line("mysql").unwrap();
    session.exp_string("MySQL username:").unwrap();
    session.send_line("mysqluser").unwrap();
    session.exp_string("MySQL password:").unwrap();
    session.send_line("password").unwrap();
    session.exp_string("Server address:").unwrap();
    session.send_line("10.0.0.10").unwrap();
    session.exp_string("MySQL port:").unwrap();
    session.send_line("1234").unwrap();
    session.exp_string("Database name:").unwrap();
    session.send_line("mydb").unwrap();
    session.exp_string("Truncate:").unwrap();
    session.send_line("true").unwrap();
    session.send("Exit").unwrap();
    session.exp_string("Exit").unwrap();
    session.send_line("").unwrap();
    session.exp_eof().unwrap();

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path()).unwrap();
    let parsed: Config = toml::from_str(&config_file_contents).unwrap();
    assert_eq!(
        parsed.destinations,
        vec![
            Destination{
                r#type: "mysql_connector".into(),
                url: "mysql://mysqluser:password@10.0.0.10:1234/mydb".into(),
                display_name: "mysql".into(),
                truncate: true,
            }
        ]
    );
    temp_dir.close().unwrap();
}

#[test]
fn cli_init_kafka_dest() {
    let _guard = lock();
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
    let temp_dir = assert_fs::TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();
    let mut session = init_session();
    session.send("Add Destination").unwrap();
    session.exp_string("Add Destination").unwrap();
    session.send_line("").unwrap();
    session.exp_string("What type of destination would you like to add?").unwrap();
    session.send("Kafka destination").unwrap();
    session.exp_string("Kafka destination").unwrap();
    session.send_line("").unwrap();
    session.exp_string("Display name:").unwrap();
    session.send_line("kafka").unwrap();
    session.exp_string("Broker:").unwrap();
    session.send_line("localhost:1000").unwrap();
    session.exp_string("Topic").unwrap();
    session.send_line("test-topic").unwrap();
    session.send("Exit").unwrap();
    session.exp_string("Exit").unwrap();
    session.send_line("").unwrap();
    session.exp_eof().unwrap();

    let config_file = temp_dir.child("config.toml");
    let config_file_contents = std::fs::read_to_string(config_file.path()).unwrap();
    let parsed: Config = toml::from_str(&config_file_contents).unwrap();
    assert_eq!(parsed.destinations.len(), 1);
    assert_eq!(parsed.destinations[0].destination_type, "kafka");
    assert_eq!(parsed.destinations[0].display_name, "kafka");
    assert_eq!(parsed.destinations[0].brokers, "localhost:1000");
    assert_eq!(parsed.destinations[0].topic, "test-topic");

    temp_dir.close().unwrap();
}
