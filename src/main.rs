use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use env_manager::{
    DEFAULT_PROFILES, ensure_profile, load_profile, profile_names, set_variable, unset_variable,
};
use std::path::PathBuf;
use std::process::{Command, ExitCode};

#[derive(Debug, Parser)]
#[command(
    name = "env-manager",
    version,
    about = "Manage and run with .env profiles"
)]
struct Cli {
    /// Directory containing the .env profiles.
    #[arg(short, long, default_value = ".")]
    directory: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create the standard development, staging, and production profiles.
    Init,
    /// List available profiles and their variable counts.
    List,
    /// Print all variables in a profile (values are visible).
    Show { profile: String },
    /// Print one variable from a profile.
    Get { profile: String, key: String },
    /// Set or replace a variable in a profile.
    Set {
        profile: String,
        key: String,
        value: String,
    },
    /// Remove a variable from a profile.
    Unset { profile: String, key: String },
    /// Run a command with the profile injected into its environment.
    Run {
        profile: String,
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<u8> {
    let cli = Cli::parse();
    let root = cli.directory.canonicalize().unwrap_or(cli.directory);
    match cli.command {
        Commands::Init => {
            for name in DEFAULT_PROFILES {
                let profile = ensure_profile(&root, name)?;
                println!("created {}", profile.path.display());
            }
            Ok(0)
        }
        Commands::List => {
            let names = profile_names(&root)?;
            if names.is_empty() {
                println!("No profiles found. Run `env-manager init` to create the defaults.");
            } else {
                for name in names {
                    let profile = load_profile(&root, &name)?;
                    println!(
                        "{:<16} {} variables  {}",
                        name,
                        profile.variables.len(),
                        profile.path.display()
                    );
                }
            }
            Ok(0)
        }
        Commands::Show { profile } => {
            let profile = load_profile(&root, &profile)?;
            for (key, value) in profile.variables {
                println!("{key}={value}");
            }
            Ok(0)
        }
        Commands::Get { profile, key } => {
            let loaded = load_profile(&root, &profile)?;
            match loaded.variables.get(&key) {
                Some(value) => println!("{value}"),
                None => bail!("variable '{key}' is not defined in profile '{profile}'"),
            }
            Ok(0)
        }
        Commands::Set {
            profile,
            key,
            value,
        } => {
            set_variable(&root, &profile, &key, &value)?;
            println!("updated {key} in {profile}");
            Ok(0)
        }
        Commands::Unset { profile, key } => {
            if unset_variable(&root, &profile, &key)? {
                println!("removed {key} from {profile}");
            } else {
                println!("{key} was not defined in {profile}");
            }
            Ok(0)
        }
        Commands::Run { profile, command } => run_command(&root, &profile, command),
    }
}

fn run_command(root: &std::path::Path, name: &str, command: Vec<String>) -> Result<u8> {
    let profile = load_profile(root, name)?;
    let (program, args) = command
        .split_first()
        .context("run requires a command after the profile")?;
    let status = Command::new(program)
        .args(args)
        .envs(&profile.variables)
        .env("ENV_PROFILE", name)
        .status()
        .with_context(|| format!("failed to start '{program}'"))?;
    Ok(status.code().unwrap_or(1) as u8)
}
