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
        match &mut self.sources {
            Some(sources) => sources.push(Source::sqlite_physical_replication {
                display_name,
                journal_path,
            }),
            None => {
                self.sources = Some(vec![Source::sqlite_physical_replication {
                    display_name,
                    journal_path,
                }])
            }
        }
    }
    pub fn add_sqlite_connector_source(&mut self, display_name: String, path: String) {
        match &mut self.sources {
            Some(sources) => sources.push(Source::sqlite_connector { display_name, path }),
            None => self.sources = Some(vec![Source::sqlite_connector { display_name, path }]),
        }
    }
    pub fn add_sqlite_physical_replication_destination(
        &mut self,
        display_name: String,
        journal_path: String,
        database_path: String,
    ) {
        match &mut self.destinations {
            Some(destinations) => destinations.push(Destination::sqlite_physical_replication {
                display_name,
                journal_path,
                database_path,
            }),
            None => {
                self.destinations = Some(vec![Destination::sqlite_physical_replication {
                    display_name,
                    journal_path,
                    database_path,
                }])
            }
        }
    }
    pub fn add_sqlite_connector_destination(&mut self, display_name: String, path: String) {
        match &mut self.destinations {
            Some(destinations) => {
                destinations.push(Destination::sqlite_connector { display_name, path })
            }
            None => {
                self.destinations = Some(vec![Destination::sqlite_connector { display_name, path }])
            }
        }
    }
    pub fn add_postgres_connector_destination(&mut self, display_name: String, url: String) {
        match &mut self.destinations {
            Some(destinations) => {
                destinations.push(Destination::postgres_connector { display_name, url })
            }
            None => {
                self.destinations =
                    Some(vec![Destination::postgres_connector { display_name, url }])
            }
        }
    }
    pub fn add_mysql_connector_destination(&mut self, display_name: String, url: String) {
        match &mut self.destinations {
            Some(destinations) => {
                destinations.push(Destination::mysql_connector { display_name, url })
            }
            None => {
                self.destinations = Some(vec![Destination::mysql_connector { display_name, url }])
            }
        }
    }
    pub fn add_kafka_destination(&mut self, display_name: String, brokers: String, topic: String) {
        match &mut self.destinations {
            Some(destinations) => destinations.push(Destination::kafka {
                display_name,
                brokers,
                topic,
            }),
            None => {
                self.destinations = Some(vec![Destination::kafka {
                    display_name,
                    brokers,
                    topic,
                }])
            }
        }
    }
    pub fn add_excel_connector_source(
        &mut self,
        display_name: String,
        path: String,
        sheets: String,
        strict: bool,
    ) {
        match &mut self.sources {
            Some(sources) => sources.push(Source::excel_connector {
                display_name,
                path,
                sheets,
                strict,
            }),
            None => {
                self.sources = Some(vec![Source::excel_connector {
                    display_name,
                    path,
                    sheets,
                    strict,
                }])
            }
        }
    }
    pub fn add_file_source(&mut self, display_name: String, path: String) {
        match &mut self.sources {
            Some(sources) => sources.push(Source::file { display_name, path }),
            None => self.sources = Some(vec![Source::file { display_name, path }]),
        }
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
        postgres_url: String,
        schema: String,
        tables: String,
        poll_interval: i32,
    ) {
        match &mut self.sources {
            Some(sources) => sources.push(Source::postgres_connector {
                display_name,
                postgres_url,
                schema,
                tables,
                poll_interval,
            }),
            None => {
                self.sources = Some(vec![Source::postgres_connector {
                    display_name,
                    postgres_url,
                    schema,
                    tables,
                    poll_interval,
                }])
            }
        }
    }

    pub(crate) fn add_mysql_connector_source(
        &mut self,
        display_name: String,
        mysql_url: String,
        schema: String,
        tables: String,
        poll_interval: i32,
    ) {
        match &mut self.sources {
            Some(sources) => sources.push(Source::mysql_connector {
                display_name,
                mysql_url,
                schema,
                tables,
                poll_interval,
            }),
            None => {
                self.sources = Some(vec![Source::mysql_connector {
                    display_name,
                    mysql_url,
                    schema,
                    tables,
                    poll_interval,
                }])
            }
        }
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
        path: String,
    },
    postgres_connector {
        display_name: String,
        postgres_url: String,
        schema: String,
        tables: String,
        poll_interval: i32,
    },
    mysql_connector {
        display_name: String,
        mysql_url: String,
        schema: String,
        tables: String,
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
