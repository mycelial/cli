use clap::{Parser, Subcommand};
use mycelial::{add_destination, add_source, destroy, init, reset, start};
mod service;
use nix::unistd::Uid;
use service::Service;

#[derive(Debug, Parser)]
#[command(name = "mycelial")]
#[command(about = "A command line interface (Cli) for Mycelial", version, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum ServiceCommands {
    /// Add a new service
    Add {
        /// Sets a custom config file
        #[clap(short, long, value_parser)]
        config: Option<String>,
        /// Installs the client as a service
        #[clap(long)]
        client: bool,
    },
    /// Remove a service
    Remove {
        /// Remove the client as a service
        #[clap(long)]
        client: bool,
        /// Removes artifacts (config, database)
        #[clap(long)]
        purge: bool,
    },
    Status {
        /// show status of the client service (myceliald)
        #[clap(long)]
        client: bool,
    },
    Start {
        /// start the client service (myceliald)
        #[clap(long)]
        client: bool,
    },
    Stop {
        /// stop the client service (myceliald)
        #[clap(long)]
        client: bool,
    },
    Restart {
        /// restart the client service (myceliald)
        #[clap(long)]
        client: bool,
    },
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// setup mycelial
    Init {
        /// download both the server and the client
        #[arg(short, long)]
        local: bool,
        /// download the client
        #[arg(short, long)]
        client: bool,
        /// download the server
        #[arg(short, long)]
        server: bool,
        /// specify a config file name to use
        #[arg(long)]
        config: Option<String>,
    },
    /// starts the server and myceliald (client)
    Start {
        /// start the client
        #[arg(short, long)]
        client: bool,
        /// start the server
        #[arg(short, long)]
        server: bool,
        /// specify a config file name to use
        #[arg(long)]
        config: Option<String>,
    },
    /// stops the server and myceliald (client)
    Destroy {
        /// destroy the client
        #[arg(short, long)]
        client: bool,
        /// destroy the server
        #[arg(short, long)]
        server: bool,
    },
    /// deletes the server and/or client databases
    Reset {
        /// delete the client database
        #[arg(short, long)]
        client: bool,
        /// delete the server database
        #[arg(short, long)]
        server: bool,
        /// specify a config file name to use
        #[arg(long)]
        config: Option<String>,
    },
    /// add a source or destination to config
    Add {
        /// add a source to config
        #[arg(short, long)]
        source: bool,
        #[arg(short, long)]
        /// add a destination to config
        destination: bool,
        /// specify a config file name to use
        #[arg(long)]
        config: Option<String>,
    },
    /// install mycelial as a service
    Service {
        #[clap(subcommand)]
        action: ServiceCommands,
    },
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    match run(args).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run(args: Cli) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match args.command {
        Commands::Init {
            local,
            client,
            server,
            config,
        } => {
            let config_file_name = match config {
                Some(config) => config,
                None => "config.toml".to_string(),
            };
            if local {
                init(true, true, config_file_name).await?;
            } else if client || server {
                init(client, server, config_file_name).await?;
            } else {
                init(false, false, config_file_name).await?;
                // return Err(
                //     "init command must be run with the --local, --client and/or --server options"
                //         .into(),
                // );
            }
        }
        Commands::Start {
            client,
            server,
            config,
        } => {
            let config_file_name = match config {
                Some(config) => config,
                None => "config.toml".to_string(),
            };
            // if neither client or server are specified, start both
            if !client && !server {
                start(true, true, config_file_name).await?;
            } else {
                start(client, server, config_file_name).await?;
            }
        }
        Commands::Destroy { client, server } => {
            // if neither client or server are specified, destroy both
            if !client && !server {
                destroy(true, true).await?;
            } else {
                destroy(client, server).await?;
            }
        }
        Commands::Reset {
            client,
            server,
            config,
        } => {
            let config_file_name = match config {
                Some(config) => config,
                None => "config.toml".to_string(),
            };
            // if neither client or server are specified, destroy both
            if !client && !server {
                reset(true, true, &config_file_name).await?;
            } else {
                if client {
                    reset(true, false, &config_file_name).await?;
                }
                if server {
                    reset(false, true, &config_file_name).await?;
                }
            }
        }
        Commands::Add {
            source,
            destination,
            config,
        } => {
            let config_file_name = match config {
                Some(config) => config,
                None => "config.toml".to_string(),
            };
            if !source && !destination {
                return Err(
                    "add command must be run with the --source and/or --destination options".into(),
                );
            }
            if source {
                add_source(&config_file_name).await?;
            }
            if destination {
                add_destination(&config_file_name).await?;
            }
        }
        Commands::Service { action } => {
            if !Uid::effective().is_root() {
                return Err("You must run this command with root permissions(sudo)".into());
            }
            match action {
                ServiceCommands::Add { config, client } => {
                    if client {
                        let service = Service::new();
                        service.add_client(config).await?;
                    } else {
                        println!("client not specified");
                    }
                }
                ServiceCommands::Remove { client, purge } => {
                    if client {
                        let service = Service::new();
                        service.remove_client(purge).await?;
                    } else {
                        println!("client not specified");
                    }
                }
                ServiceCommands::Status { client } => {
                    let service = Service::new();
                    if client {
                        service.status_client()?;
                    }
                    if !client {
                        println!("client not specified");
                    }
                }
                ServiceCommands::Start { client } => {
                    let service = Service::new();
                    if client {
                        service.start_client()?;
                    }
                    if !client {
                        println!("client not specified");
                    }
                }
                ServiceCommands::Stop { client } => {
                    let service = Service::new();
                    if client {
                        service.stop_client()?;
                    }
                    if !client {
                        println!("client not specified");
                    }
                }
                ServiceCommands::Restart { client } => {
                    let service = Service::new();
                    if client {
                        service.restart_client()?;
                    }
                    if !client {
                        println!("client not specified");
                    }
                }
            }
        }
    }
    Ok(())
}
