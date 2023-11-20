use serde::{Deserialize, Serialize};
use std::fs;

impl Config {
    pub fn new() -> Config {
        Config {
            node: None,
            server: None,
            sources: Vec::new(),
            destinations: Vec::new(),
        }
    }
    pub fn set_node(&mut self, display_name: String, unique_id: String, storage_path: String) {
        self.node = Some(Node {
            display_name,
            unique_id,
            storage_path,
        });
    }
    pub fn set_server(&mut self, endpoint: String, token: String) {
        self.server = Some(Server { endpoint, token });
    }
    pub fn add_sqlite_physical_replication_source(
        &mut self,
        display_name: String,
        journal_path: String,
    ) {
        self.sources.push(Source::sqlite_physical_replication {
            display_name,
            journal_path,
        });
    }
    pub fn add_sqlite_connector_source(&mut self, display_name: String, path: String) {
        self.sources
            .push(Source::sqlite_connector { display_name, path });
    }
    pub fn add_sqlite_physical_replication_destination(
        &mut self,
        display_name: String,
        journal_path: String,
        database_path: String,
    ) {
        self.destinations
            .push(Destination::sqlite_physical_replication {
                display_name,
                journal_path,
                database_path,
            });
    }
    pub fn add_sqlite_connector_destination(&mut self, display_name: String, path: String) {
        self.destinations
            .push(Destination::sqlite_connector { display_name, path });
    }
    pub fn add_postgres_connector_destination(&mut self, display_name: String, url: String) {
        self.destinations
            .push(Destination::postgres_connector { display_name, url });
    }
    pub fn add_mysql_connector_destination(&mut self, display_name: String, url: String) {
        self.destinations
            .push(Destination::mysql_connector { display_name, url });
    }
    pub fn add_kafka_destination(&mut self, display_name: String, brokers: String, topic: String) {
        self.destinations.push(Destination::kafka {
            display_name,
            brokers,
            topic,
        });
    }
    pub fn save(&self, path: String) -> Result<(), Box<dyn std::error::Error>> {
        let toml = toml::to_string(&self)?;
        fs::write(path, toml)?;
        Ok(())
    }
    pub fn load(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }
    pub fn get_node_storage_path(&self) -> Option<String> {
        match &self.node {
            Some(node) => Some(node.storage_path.clone()),
            None => None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    node: Option<Node>,
    server: Option<Server>,
    sources: Vec<Source>,
    destinations: Vec<Destination>,
}

#[derive(Serialize, Deserialize)]
struct Node {
    display_name: String,
    unique_id: String,
    storage_path: String,
}

#[derive(Serialize, Deserialize)]
struct Server {
    endpoint: String,
    token: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(non_camel_case_types)]
enum Source {
    excel_connector {
        display_name: String,
        path: String,
        strict: bool,
    },
    sqlite_physical_replication {
        display_name: String,
        journal_path: String,
    },
    hello_world {
        interval_milis: u64,
        message: String,
        display_name: String,
    },
    sqlite_connector {
        display_name: String,
        path: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(non_camel_case_types)]
enum Destination {
    sqlite_physical_replication {
        display_name: String,
        journal_path: String,
        database_path: String,
    },
    hello_world {
        display_name: String,
    },
    sqlite_connector {
        display_name: String,
        path: String,
    },
    postgres_connector {
        display_name: String,
        url: String,
    },
    mysql_connector {
        display_name: String,
        url: String,
    },
    kafka {
        display_name: String,
        brokers: String,
        topic: String,
    },
}
