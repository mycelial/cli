use serde::{Deserialize, Serialize};
use std::fs;

impl Config {
    pub fn new() -> Config {
        Config {
            node: None,
            server: None,
            sources: None,
            destinations: None,
        }
    }
    pub fn set_node(
        &mut self,
        display_name: String,
        unique_id: String,
        storage_path: String,
        auth_token: String,
    ) {
        self.node = Some(Node {
            display_name,
            unique_id,
            storage_path,
            auth_token,
        });
    }
    pub fn set_server(&mut self, endpoint: String) {
        self.server = Some(Server { endpoint });
    }
    pub fn add_sqlite_connector_source(
        &mut self,
        display_name: String,
        origin: String,
        path: String,
        query: String,
    ) {
        self.get_mut_sources().push(Source::sqlite_connector {
            display_name,
            origin,
            path,
            query,
        });
    }

    pub fn add_sqlite_connector_destination(
        &mut self,
        display_name: String,
        path: String,
        truncate: bool,
    ) {
        self.get_mut_destinations()
            .push(Destination::sqlite_connector {
                display_name,
                path,
                truncate,
            });
    }

    pub fn add_postgres_connector_destination(
        &mut self,
        display_name: String,
        url: String,
        truncate: bool,
    ) {
        self.get_mut_destinations()
            .push(Destination::postgres_connector {
                display_name,
                url,
                truncate,
            });
    }

    pub fn add_mysql_connector_destination(
        &mut self,
        display_name: String,
        url: String,
        truncate: bool,
    ) {
        self.get_mut_destinations()
            .push(Destination::mysql_connector {
                display_name,
                url,
                truncate,
            });
    }
    pub fn add_kafka_destination(&mut self, display_name: String, brokers: String, topic: String) {
        self.get_mut_destinations().push(Destination::kafka {
            display_name,
            brokers,
            topic,
        });
    }

    pub fn add_excel_connector_source(
        &mut self,
        display_name: String,
        path: String,
        sheets: String,
        strict: bool,
    ) {
        self.get_mut_sources().push(Source::excel_connector {
            display_name,
            path,
            sheets,
            strict,
        });
    }
    pub fn add_file_source(&mut self, display_name: String, path: String) {
        self.get_mut_sources()
            .push(Source::file { display_name, path });
    }
    pub fn add_file_destination(&mut self, display_name: String, path: String) {
        self.get_mut_destinations()
            .push(Destination::file { display_name, path });
    }
    pub fn add_snowflake_connector_destination(
        &mut self,
        display_name: String,
        username: String,
        password: String,
        role: String,
        account_identifier: String,
        warehouse: String,
        database: String,
        schema: String,
        truncate: bool,
    ) {
        self.get_mut_destinations().push(Destination::snowflake {
            display_name,
            username,
            password,
            role,
            account_identifier,
            warehouse,
            database,
            schema,
            truncate,
        });
    }
    pub fn save<T: AsRef<str>>(&self, path: T) -> Result<(), Box<dyn std::error::Error>> {
        let path = path.as_ref();
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

    pub(crate) fn add_postgres_connector_source(
        &mut self,
        display_name: String,
        url: String,
        origin: String,
        query: String,
        poll_interval: i32,
    ) {
        self.get_mut_sources().push(Source::postgres_connector {
            display_name,
            url,
            origin,
            query,
            poll_interval,
        });
    }

    pub(crate) fn add_mysql_connector_source(
        &mut self,
        display_name: String,
        url: String,
        origin: String,
        query: String,
        poll_interval: i32,
    ) {
        self.get_mut_sources().push(Source::mysql_connector {
            display_name,
            url,
            origin,
            query,
            poll_interval,
        })
    }

    fn get_mut_destinations(&mut self) -> &mut Vec<Destination> {
        self.destinations = Some(self.destinations.take().unwrap_or(vec![]));
        self.destinations.as_mut().unwrap()
    }

    fn get_mut_sources(&mut self) -> &mut Vec<Source> {
        self.sources = Some(self.sources.take().unwrap_or(vec![]));
        self.sources.as_mut().unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    node: Option<Node>,
    server: Option<Server>,
    sources: Option<Vec<Source>>,
    destinations: Option<Vec<Destination>>,
}

#[derive(Serialize, Deserialize)]
struct Node {
    display_name: String,
    unique_id: String,
    storage_path: String,
    auth_token: String,
}

#[derive(Serialize, Deserialize)]
struct Server {
    endpoint: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(non_camel_case_types)]
enum Source {
    excel_connector {
        display_name: String,
        path: String,
        sheets: String,
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
        origin: String,
        path: String,
        query: String,
    },
    postgres_connector {
        display_name: String,
        url: String,
        origin: String,
        query: String,
        poll_interval: i32,
    },
    mysql_connector {
        display_name: String,
        url: String,
        origin: String,
        query: String,
        poll_interval: i32,
    },
    file {
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
        truncate: bool,
    },
    postgres_connector {
        display_name: String,
        url: String,
        truncate: bool,
    },
    mysql_connector {
        display_name: String,
        url: String,
        truncate: bool,
    },
    kafka {
        display_name: String,
        brokers: String,
        topic: String,
    },
    snowflake {
        display_name: String,
        username: String,
        password: String,
        role: String,
        account_identifier: String,
        warehouse: String,
        database: String,
        schema: String,
        truncate: bool,
    },
    file {
        display_name: String,
        path: String,
    },
}
