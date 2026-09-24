use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about = "Package Niva applications with verified precompiled runtimes"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Build {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        resource_dir: PathBuf,
        #[arg(long)]
        output_dir: PathBuf,
        #[arg(long, required = true)]
        target: Vec<niva_packager::Target>,
    },
}
fn main() {
    let Command::Build {
        manifest,
        config,
        resource_dir,
        output_dir,
        target,
    } = Cli::parse().command;
    let report = niva_packager::build(&manifest, &config, &resource_dir, &output_dir, &target);
    match report {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string(&report).expect("serialize report")
            );
            if report.results.iter().any(|r| r.status == "failed") {
                std::process::exit(1);
            }
        }
        Err(error) => {
            println!(
                "{}",
                serde_json::json!({"results": [], "error": format!("{error:#}")})
            );
            std::process::exit(1);
        }
    }
}
