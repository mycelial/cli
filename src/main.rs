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
        #[arg(short, long)]
        local: bool,
    },
    /// starts the server and myceliald (client)
    Start,
    /// stops the server and myceliald (client)
    Destroy,
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
        Commands::Init { local } => {
            if local {
                init().await?;
            } else {
                return Err(
                    "init command must be run with --local option ex: `mycelial init --local`"
                        .into(),
                );
            }
        }
        Commands::Start => {
            start().await?;
        }
        Commands::Destroy => {
            destroy().await?;
        }
    }
    Ok(())
}
