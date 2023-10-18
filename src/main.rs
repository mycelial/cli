use clap::{Parser, Subcommand};
use cli::{destroy, init, start};

#[derive(Debug, Parser)]
#[command(name = "mycelial")]
#[command(about = "A command line interface (Cli) for Mycelial", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long, action = clap::ArgAction::Count)]
    local: u8,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init,
    Start,
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
        Commands::Init => {
            let is_local = args.local > 0;
            if is_local {
                init().await?;
            } else {
                return Err("init command must be run with --local option".into());
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
