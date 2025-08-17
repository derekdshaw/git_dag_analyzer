#![warn(clippy::all, clippy::pedantic)]

use anyhow::Result;
use clap::{Parser, Subcommand};
use git_dag_analyzer::{
    git_processing::{process_all_commit_deps, process_initial_repo, process_tags},
    object_collection::ObjectContainer,
    report_all::report_all,
    report_blobs::report_blobs,
    report_commits::report_commits,
    report_trees::report_trees,
};
use std::path::PathBuf;
use tokio::main;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// REQUIRED: The git repo to work against
    #[arg(short, long, value_name = "REPO_PATH", required(true))]
    repo: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Outputs a report of repo size information
    Reports {
        #[arg(short, long)]
        all: bool,

        #[arg(short, long)]
        commits: bool,

        /// If set and the file is not present, it will be created for further use. If
        /// present then it will be loaded for processeing. Saving the time it normally
        /// takes to process commit deps.
        #[arg(short, long, value_name = "SAVE_LOCATION")]
        save_deps: Option<PathBuf>,

        #[arg(short, long)]
        trees: bool,

        #[arg(short, long)]
        blobs: bool,
    },
    /// Only process the data
    ProcessOnly {
        #[arg(short, long)]
        all: bool,

        #[arg(short, long)]
        commits: bool,

        /// If set and the file is not present, it will be created for further use. If
        /// present then it will be loaded for processeing. Saving the time it normally
        /// takes to process commit deps.
        #[arg(short, long, value_name = "SAVE_LOCATION")]
        save_deps: Option<PathBuf>,

        // This is tags, but it conflicts with the short command -t of trees.
        #[arg(short, long)]
        labels: bool,
    },
}

#[main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // since this is required by the cli, we can safely unwrap here.
    let repo_path = cli.repo.as_deref().unwrap();
    let mut container = ObjectContainer::new();

    match &cli.command {
        Some(Commands::Reports {
            all,
            commits,
            save_deps,
            trees,
            blobs,
        }) => {
            // first we have to process everything
            process_initial_repo(repo_path, &mut container);

            // required for all three reporting types.
            process_all_commit_deps(repo_path, &container, save_deps).await?;

            // Do reports
            if *all {
                process_tags(repo_path, &container);
                report_all(&container);
            } else if *commits {
                report_commits(&container);
            } else if *trees {
                report_trees(&container);
            } else if *blobs {
                report_blobs(&container);
            }
        }
        Some(Commands::ProcessOnly {
            all,
            commits,
            save_deps,
            labels,
        }) => {
            process_initial_repo(repo_path, &mut container);
            if *all {
                process_all_commit_deps(repo_path, &container, save_deps).await?;
                process_tags(repo_path, &container);
            } else if *commits {
                process_all_commit_deps(repo_path, &container, save_deps).await?;
            } else if *labels {
                process_tags(repo_path, &container);
            }
        }
        None => {}
    }

    Ok(())
}
