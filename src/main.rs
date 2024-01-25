use clap::{Parser, Subcommand};
use mycelial::{add_destination, add_source, destroy, download_binaries, init, reset, start};
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
        /// Installs the daemon as a service
        #[clap(long)]
        daemon: bool,
    },
    /// Remove a service
    Remove {
        /// Remove the daemon as a service
        #[clap(long)]
        daemon: bool,
        /// Removes artifacts (config, database)
        #[clap(long)]
        purge: bool,
    },
    /// Check status of service
    Status {
        /// show status of the daemon service
        #[clap(long)]
        daemon: bool,
    },
    /// Start a service
    Start {
        /// start the daemon service
        #[clap(long)]
        daemon: bool,
    },
    /// Stop a service
    Stop {
        /// stop the daemon service
        #[clap(long)]
        daemon: bool,
    },
    /// Restart a service
    Restart {
        /// restart the daemon service
        #[clap(long)]
        daemon: bool,
    },
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// setup mycelial
    Init {
        /// download both the daemon and the control plane
        #[arg(short, long)]
        local: bool,
        /// download the daemon
        #[arg(short, long)]
        daemon: bool,
        /// download the control plane
        #[arg(short, long)]
        control_plane: bool,
        /// specify a config file name to use
        #[arg(long)]
        config: Option<String>,
    },
    /// starts the daemon and control plane
    Start {
        /// start the daemon
        #[arg(short, long)]
        daemon: bool,
        /// start the control plane
        #[arg(short, long)]
        control_plane: bool,
        /// specify a config file name to use
        #[arg(long)]
        config: Option<String>,
    },
    /// stops the daemon and control plane
    Destroy {
        /// destroy the daemon
        #[arg(short, long)]
        daemon: bool,
        /// destroy the control plane
        #[arg(short, long)]
        control_plane: bool,
    },
    /// deletes the daemon and/or control plane  databases
    Reset {
        /// delete the daemon database
        #[arg(short, long)]
        daemon: bool,
        /// delete the control plane database
        #[arg(short, long)]
        control_plane: bool,
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
    /// update mycelial binaries
    Update {
        /// update the daemon
        #[arg(short, long)]
        daemon: bool,
        /// update the control plane
        #[arg(short, long)]
        control_plane: bool,
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
            daemon,
            control_plane,
            config,
        } => {
            let config_file_name = match config {
                Some(config) => config,
                None => "config.toml".to_string(),
            };
            if local {
                init(true, true, config_file_name).await?;
            } else if daemon || control_plane {
                init(daemon, control_plane, config_file_name).await?;
            } else {
                init(false, false, config_file_name).await?;
            }
        }
        Commands::Start {
            daemon,
            control_plane,
            config,
        } => {
            let config_file_name = match config {
                Some(config) => config,
                None => "config.toml".to_string(),
            };
            // if neither daemon or control_plane are specified, start both
            if !daemon && !control_plane {
                start(true, true, config_file_name).await?;
            } else {
                start(daemon, control_plane, config_file_name).await?;
            }
        }
        Commands::Destroy {
            daemon,
            control_plane,
        } => {
            // if neither daemon or control_plane are specified, destroy both
            if !daemon && !control_plane {
                destroy(true, true).await?;
            } else {
                destroy(daemon, control_plane).await?;
            }
        }
        Commands::Reset {
            daemon,
            control_plane,
            config,
        } => {
            let config_file_name = match config {
                Some(config) => config,
                None => "config.toml".to_string(),
            };
            // if neither daemon or control_plane are specified, destroy both
            if !daemon && !control_plane {
                reset(true, true, &config_file_name).await?;
            } else {
                if daemon {
                    reset(true, false, &config_file_name).await?;
                }
                if control_plane {
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
        Commands::Update {
            daemon,
            control_plane,
        } => {
            if !daemon && !control_plane {
                return Err(
                    "update command must be run with the --daemon and/or --control-plane options"
                        .into(),
                );
            }
            download_binaries(daemon, control_plane).await?;
            println!("Update complete");
        }
        Commands::Service { action } => {
            if !Uid::effective().is_root() {
                return Err("You must run this command with root permissions(sudo)".into());
            }
            match action {
                ServiceCommands::Add { config, daemon } => {
                    if daemon {
                        let service = Service::new();
                        service.add_client(config).await?;
                    } else {
                        println!("--daemon not specified");
                    }
                }
                ServiceCommands::Remove { daemon, purge } => {
                    if daemon {
                        let service = Service::new();
                        service.remove_client(purge).await?;
                    } else {
                        println!("--daemon not specified");
                    }
                }
                ServiceCommands::Status { daemon } => {
                    let service = Service::new();
                    if daemon {
                        service.status_client()?;
                    }
                    if !daemon {
                        println!("--daemon not specified");
                    }
                }
                ServiceCommands::Start { daemon } => {
                    let service = Service::new();
                    if daemon {
                        service.start_client()?;
                    }
                    if !daemon {
                        println!("--daemon not specified");
                    }
                }
                ServiceCommands::Stop { daemon } => {
                    let service = Service::new();
                    if daemon {
                        service.stop_client()?;
                    }
                    if !daemon {
                        println!("--daemon not specified");
                    }
                }
                ServiceCommands::Restart { daemon } => {
                    let service = Service::new();
                    if daemon {
                        service.restart_client()?;
                    }
                    if !daemon {
                        println!("--daemon not specified");
                    }
                }
            }
        }
    }
    Ok(())
}
