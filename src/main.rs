use clap::{Parser, Subcommand};
use mycelial::{destroy, init, start};

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
    },
    /// starts the server and myceliald (client)
    Start {
        /// start the client
        #[arg(short, long)]
        client: bool,
        /// start the server
        #[arg(short, long)]
        server: bool,
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
        } => {
            if local {
                init(true, true).await?;
            } else if client || server {
                init(client, server).await?;
            } else {
                return Err(
                    "init command must be run with the --local, --client and/or --server options"
                        .into(),
                );
            }
        }
        Commands::Start { client, server } => {
            // if neither client or server are specified, start both
            if !client && !server {
                start(true, true).await?;
            } else {
                start(client, server).await?;
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
    }
    Ok(())
}
