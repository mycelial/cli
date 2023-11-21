use clap::{Parser, Subcommand};
use mycelial::{destroy, init, reset, start};

#[derive(Debug, Parser)]
#[command(name = "mycelial")]
#[command(about = "A command line interface (Cli) for Mycelial", version, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
                return Err(
                    "init command must be run with the --local, --client and/or --server options"
                        .into(),
                );
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
    }
    Ok(())
}
