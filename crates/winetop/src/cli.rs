use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use winetop_core::classify::process_tree_lines;
use winetop_core::kill::{self, KillMethod, KillRequest};
use winetop_core::util::format_bytes;
use winetop_core::{scan, SessionSnapshot};

#[derive(Debug, Parser)]
#[command(
    name = "winetop",
    about = "htop for Wine prefixes — monitor and stop Wine/Proton sessions",
    version
)]
pub struct Cli {
    /// Increase logging to stderr
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// TUI refresh interval in milliseconds
    #[arg(long, default_value_t = 1000)]
    pub refresh_ms: u64,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List Wine/Proton sessions
    List {
        #[arg(long)]
        json: bool,
    },
    /// Show process tree for Wine-related PIDs
    Tree {
        #[arg(long)]
        json: bool,
    },
    /// List orphan wineservers / zombies
    Orphans {
        #[arg(long)]
        json: bool,
    },
    /// Dump full session snapshot as JSON
    Dump,
    /// Kill a process, session, or prefix
    Kill {
        #[arg(long)]
        pid: Option<u32>,
        #[arg(long)]
        appid: Option<u32>,
        #[arg(long)]
        prefix: Option<PathBuf>,
        #[arg(long)]
        session: Option<String>,
        /// term or kill (for --pid)
        #[arg(long, default_value = "term")]
        signal: SignalArg,
        /// Override method: reaper, wineserver, hard
        #[arg(long)]
        method: Option<MethodArg>,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum SignalArg {
    Term,
    Kill,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum MethodArg {
    Reaper,
    Wineserver,
    Hard,
}

pub fn cmd_list(json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let snap = scan()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&snap.sessions)?);
        return Ok(());
    }
    print_sessions_table(&snap);
    Ok(())
}

pub fn cmd_tree(json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let snap = scan()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&snap.sessions)?);
        return Ok(());
    }
    let lines = process_tree_lines(&snap.sessions);
    if lines.is_empty() {
        println!("No Wine/Proton processes.");
    } else {
        for line in lines {
            println!("{line}");
        }
    }
    Ok(())
}

pub fn cmd_orphans(json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let snap = scan()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&snap.orphans)?);
        return Ok(());
    }
    if snap.orphans.is_empty() {
        println!("No orphans detected.");
        return Ok(());
    }
    for o in &snap.orphans {
        println!("[{}] {}", o.kind, o.detail);
    }
    Ok(())
}

pub fn cmd_dump() -> Result<(), Box<dyn std::error::Error>> {
    let snap = scan()?;
    println!("{}", serde_json::to_string_pretty(&snap)?);
    Ok(())
}

pub fn cmd_kill(
    pid: Option<u32>,
    appid: Option<u32>,
    prefix: Option<PathBuf>,
    session: Option<String>,
    signal: SignalArg,
    method: Option<MethodArg>,
) -> Result<(), Box<dyn std::error::Error>> {
    let snap = scan()?;
    let kill_method = if let Some(m) = method {
        match m {
            MethodArg::Reaper => KillMethod::SessionReaper,
            MethodArg::Wineserver => KillMethod::WineServerK,
            MethodArg::Hard => KillMethod::SessionKill,
        }
    } else if pid.is_some() {
        match signal {
            SignalArg::Term => KillMethod::ProcessTerm,
            SignalArg::Kill => KillMethod::ProcessKill,
        }
    } else if appid.is_some() {
        KillMethod::SessionReaper
    } else if prefix.is_some() {
        KillMethod::WineServerK
    } else if session.is_some() {
        KillMethod::SessionReaper
    } else {
        return Err("specify --pid, --appid, --prefix, or --session".into());
    };

    let req = KillRequest {
        method: kill_method,
        pid,
        session_id: session,
        steam_app_id: appid,
        prefix,
    };
    let result = kill::execute(&req, &snap)?;
    println!("{}", result.message);
    if !result.ok {
        return Err("kill reported failure".into());
    }
    Ok(())
}

fn print_sessions_table(snap: &SessionSnapshot) {
    if snap.sessions.is_empty() {
        println!("No Wine/Proton sessions detected.");
        return;
    }
    println!(
        "{:<8} {:<28} {:<24} {:>6} {:>7} {:>3} WS",
        "SRC", "SESSION", "PREFIX", "CPU", "RSS", "N"
    );
    for s in &snap.sessions {
        let ws = if s.wineserver_alive { "●" } else { "○" };
        let app = s
            .steam_app_id
            .map(|id| format!("#{id}"))
            .unwrap_or_default();
        let name = if app.is_empty() {
            s.name.clone()
        } else {
            format!("{} {}", s.name, app)
        };
        println!(
            "{:<8} {:<28} {:<24} {:>5.1}% {:>7} {:>3} {}",
            s.source.as_str(),
            truncate(&name, 28),
            truncate(&s.short_prefix(), 24),
            s.cpu_percent,
            format_bytes(s.rss_bytes),
            s.process_count(),
            ws
        );
    }
    let (procs, cpu, rss) = snap.totals();
    println!(
        "\n{} sessions · {} procs · {:.1}% cpu · {}",
        snap.sessions.len(),
        procs,
        cpu,
        format_bytes(rss)
    );
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}
