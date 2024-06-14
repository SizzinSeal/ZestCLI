use cargo_metadata::camino::Utf8PathBuf;
use cargo_pros::{build, finish_binary, launch_simulator, CommandExt};
use clap::{Args, Parser, Subcommand};
use std::{path::PathBuf, process::Command};

cargo_subcommand_metadata::description!("Manage pros-rs projects");

#[derive(Parser, Debug)]
#[clap(bin_name = "cargo")]
enum Cli {
    /// Manage pros-rs projects
    #[clap(version)]
    Pros(Opt),
}

#[derive(Args, Debug)]
struct Opt {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value = ".")]
    path: PathBuf,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Build {
        #[clap(long, short)]
        simulator: bool,
        #[clap(last = true)]
        args: Vec<String>,
    },
    Upload {
        #[clap(long, short)]
        slot: u8,
        #[clap(long, short)]
        file: Option<Utf8PathBuf>,
        /// Convert the program to a stripped binary before uploading it.
        /// This is necessary for uploading an ELF that has not yet
        /// been processed.
        #[clap(long, short)]
        strip: bool,
        #[clap(long, short, default_value = "none")]
        action: UploadAction,

        #[clap(last = true)]
        args: Vec<String>,
    },
    Sim {
        #[clap(long)]
        ui: Option<String>,
        #[clap(last = true)]
        args: Vec<String>,
    },
}

#[derive(Clone, Debug, Default)]
enum UploadAction {
    Screen,
    Run,
    #[default]
    None,
}
impl std::str::FromStr for UploadAction {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "screen" => Ok(UploadAction::Screen),
            "run" => Ok(UploadAction::Run),
            "none" => Ok(UploadAction::None),
            _ => Err(format!(
                "Invalid upload action. Found: {}, expected one of: screen, run, or none",
                s
            )),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "legacy-pros-rs-support")]
    println!("cargo-pros is using legacy pros-rs support. Please consider upgrading to the new vexide crate.");

    let Cli::Pros(args) = Cli::parse();
    let path = args.path;

    match args.command {
        Commands::Build { simulator, args } => {
            build(path, args, simulator, |path| {
                if !simulator {
                    finish_binary(&path);
                }
            });
        }
        Commands::Upload {
            slot,
            file,
            action,
            strip,
            args,
        } => {
            let mut artifact = None;
            if let Some(path) = file {
                if strip {
                    artifact = Some(finish_binary(&path));
                } else {
                    artifact = Some(path);
                }
            } else {
                build(path.clone(), args, false, |new_artifact| {
                    let mut bin_path = new_artifact.clone();
                    bin_path.set_extension("bin");
                    artifact = Some(bin_path);
                    finish_binary(&new_artifact);
                });
            }
            let artifact =
                artifact.expect("Binary not found! Try explicitly providing one with --path (-p)");
            Command::new("pros")
                .args([
                    "upload",
                    "--target",
                    "v5",
                    "--slot",
                    &slot.to_string(),
                    "--after",
                    match action {
                        UploadAction::Screen => "screen",
                        UploadAction::Run => "run",
                        UploadAction::None => "none",
                    },
                    artifact.as_str(),
                ])
                .spawn_handling_not_found()?
                .wait()?;
        }
        Commands::Sim { ui, args } => {
            let mut artifact = None;
            build(path.clone(), args, true, |new_artifact| {
                artifact = Some(new_artifact);
            });
            launch_simulator(
                ui.clone(),
                path.as_ref(),
                artifact
                    .expect("Binary target not found (is this a library?)")
                    .as_ref(),
            );
        }
    }

    Ok(())
}
