use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use traits::commands;

#[derive(Parser)]
#[command(
    name = "traits",
    version,
    about = "Reusable preferences for your AI agent"
)]
struct Cli {
    /// Path to a traits.kdl configuration file
    #[arg(long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the MCP server over stdio (default when stdin is piped)
    Serve,
    /// Write a starter traits.kdl and create the trait store
    Init {
        #[arg(long)]
        force: bool,
    },
    /// Assess project files against stored traits
    Check {
        /// File paths or glob patterns
        patterns: Vec<String>,
        /// Restrict the assessment to these trait names
        #[arg(long = "trait")]
        trait_names: Vec<String>,
        /// Circumstances triggering this assessment
        #[arg(long)]
        circumstances: Option<String>,
        /// Print findings as JSON
        #[arg(long)]
        json: bool,
        /// Ask every question instead of reusing cached answers
        #[arg(long)]
        no_cache: bool,
    },
    /// Inspect the trait store
    Trait {
        #[command(subcommand)]
        command: TraitCommand,
    },
    /// Validate and print resolved configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Install the bundled agent skill
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
    },
}

#[derive(Subcommand)]
enum TraitCommand {
    /// List stored traits
    Ls,
    /// Print a stored trait
    Show { name: String },
    /// Remove a stored trait
    Rm { name: String },
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Validate the resolved configuration
    Check,
}

#[derive(Subcommand)]
enum SkillCommand {
    /// Write the Traits skill into an agent's skill directory
    Install {
        #[arg(long, default_value = "project")]
        agent: String,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let config = cli.config.as_deref();
    let result = match cli.command {
        None if std::io::stdin().is_terminal() => {
            use clap::CommandFactory;
            let mut command = Cli::command();
            let _ = command.print_help();
            Ok(0)
        }
        None | Some(Command::Serve) => commands::serve(config).await.map(|_| 0),
        Some(Command::Init { force }) => commands::init(force).map(|_| 0),
        Some(Command::Check {
            patterns,
            trait_names,
            circumstances,
            json,
            no_cache,
        }) => commands::check(config, patterns, trait_names, circumstances, json, no_cache).await,
        Some(Command::Trait { command }) => match command {
            TraitCommand::Ls => commands::trait_ls(config).await.map(|_| 0),
            TraitCommand::Show { name } => commands::trait_show(config, &name).await.map(|_| 0),
            TraitCommand::Rm { name } => commands::trait_rm(config, &name).await.map(|_| 0),
        },
        Some(Command::Config { command }) => match command {
            ConfigCommand::Check => commands::config_check(config).map(|_| 0),
        },
        Some(Command::Skill { command }) => match command {
            SkillCommand::Install { agent } => commands::skill_install(&agent).map(|_| 0),
        },
    };

    match result {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
