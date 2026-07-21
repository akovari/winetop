mod cli;
mod ui;

use clap::Parser;
use cli::{Cli, Command};
use tracing_subscriber::EnvFilter;

fn main() {
    let cli = Cli::parse();
    init_tracing(cli.verbose);
    if let Err(e) = run(cli) {
        eprintln!("winetop: {e}");
        std::process::exit(1);
    }
}

fn init_tracing(verbose: bool) {
    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        None => ui::run(cli.refresh_ms)?,
        Some(Command::List { json }) => cli::cmd_list(json)?,
        Some(Command::Tree { json }) => cli::cmd_tree(json)?,
        Some(Command::Orphans { json }) => cli::cmd_orphans(json)?,
        Some(Command::Dump) => cli::cmd_dump()?,
        Some(Command::Kill {
            pid,
            appid,
            prefix,
            session,
            signal,
            method,
        }) => cli::cmd_kill(pid, appid, prefix, session, signal, method)?,
    }
    Ok(())
}
