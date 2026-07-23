//! `winetop status` — one-shot / watch output for status bars (Waybar, etc.).

use serde::Serialize;
use std::io::{self, Write};
use std::process::Command;
use std::thread;
use std::time::Duration;
use winetop_core::util::format_bytes;
use winetop_core::{
    build_report, sample_snapshot, source_icon, FocusHint, PickPolicy, Source, StatusFilter,
    StatusReport,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusFormat {
    Text,
    Json,
    Waybar,
}

pub struct StatusArgs {
    pub format: StatusFormat,
    pub pick: PickPolicy,
    pub sample_ms: u64,
    pub interval_ms: u64,
    pub include_opaque: bool,
    pub min_rss_mib: u64,
    pub appid: Option<u32>,
    pub session: Option<String>,
}

#[derive(Debug, Serialize)]
struct WaybarOut {
    text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    tooltip: String,
    class: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    percentage: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alt: Option<String>,
}

pub fn run(args: StatusArgs) -> Result<(), Box<dyn std::error::Error>> {
    let filter = StatusFilter {
        include_opaque: args.include_opaque,
        min_rss_bytes: args.min_rss_mib.saturating_mul(1024 * 1024),
        steam_app_id: args.appid,
        session_id: args.session.clone(),
    };

    loop {
        let focus = if args.pick == PickPolicy::Focused {
            detect_focus()
        } else {
            None
        };
        let snap = sample_snapshot(args.sample_ms)?;
        let report = build_report(&snap, args.pick, &filter, focus.as_ref());
        emit(&report, args.format)?;
        if args.interval_ms == 0 {
            break;
        }
        thread::sleep(Duration::from_millis(args.interval_ms));
    }
    Ok(())
}

fn emit(report: &StatusReport, format: StatusFormat) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        StatusFormat::Text => {
            if !report.present {
                println!("(idle)");
            } else {
                println!(
                    "{} · {:.0}% · {} · {} procs",
                    report.name.as_deref().unwrap_or("?"),
                    report.cpu_percent,
                    format_bytes(report.rss_bytes),
                    report.process_count
                );
            }
        }
        StatusFormat::Json => {
            println!("{}", serde_json::to_string(report)?);
        }
        StatusFormat::Waybar => {
            let out = waybar_payload(report);
            println!("{}", serde_json::to_string(&out)?);
        }
    }
    let _ = io::stdout().flush();
    Ok(())
}

fn waybar_payload(report: &StatusReport) -> WaybarOut {
    if !report.present {
        return WaybarOut {
            text: String::new(),
            tooltip: "No active Wine/Proton session".into(),
            class: "idle".into(),
            percentage: None,
            alt: None,
        };
    }

    let name = report
        .short_name
        .as_deref()
        .or(report.name.as_deref())
        .unwrap_or("game");
    let source = report
        .source
        .as_deref()
        .map(|s| match s {
            "Steam" => Source::Steam,
            "Lutris" => Source::Lutris,
            "Heroic" => Source::Heroic,
            "Bottles" => Source::Bottles,
            "Wine" => Source::Wine,
            _ => Source::Unknown,
        })
        .unwrap_or(Source::Unknown);
    let icon = source_icon(source);
    let cpu = report.cpu_percent;
    let rss = format_bytes(report.rss_bytes);
    let text = format!("{icon} {name} · {cpu:.0}% · {rss}");

    let mut tip = Vec::new();
    tip.push(format!(
        "{} ({})",
        report.name.as_deref().unwrap_or("?"),
        report.source.as_deref().unwrap_or("?")
    ));
    if let Some(id) = report.steam_app_id {
        tip.push(format!("Steam AppId {id}"));
    }
    tip.push(format!("CPU {cpu:.1}%"));
    tip.push(format!("RSS {rss}"));
    tip.push(format!(
        "{} procs · wineserver {}",
        report.process_count,
        if report.wineserver_alive {
            "alive"
        } else {
            "down"
        }
    ));
    if report.session_count > 1 {
        tip.push(format!("{} sessions total", report.session_count));
    }

    let class = if cpu >= 400.0 {
        "critical"
    } else if cpu >= 200.0 {
        "warning"
    } else {
        "gaming"
    };

    WaybarOut {
        text,
        tooltip: tip.join("\n"),
        class: class.into(),
        percentage: Some(cpu.clamp(0.0, 100.0).round() as u32),
        alt: report.short_name.clone(),
    }
}

fn detect_focus() -> Option<FocusHint> {
    detect_focus_sway().or_else(detect_focus_hypr)
}

fn detect_focus_sway() -> Option<FocusHint> {
    let out = Command::new("swaymsg")
        .args(["-t", "get_tree"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let tree: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let focused = find_focused(&tree)?;
    hint_from_node(focused)
}

fn detect_focus_hypr() -> Option<FocusHint> {
    let out = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let class = v.get("class").and_then(|x| x.as_str()).map(str::to_string);
    let title = v.get("title").and_then(|x| x.as_str()).map(str::to_string);
    let steam_app_id = class
        .as_deref()
        .and_then(winetop_core::parse_steam_app_class);
    Some(FocusHint {
        app_id: class.clone(),
        class,
        title,
        steam_app_id,
    })
}

fn find_focused(node: &serde_json::Value) -> Option<&serde_json::Value> {
    if node.get("focused").and_then(|v| v.as_bool()) == Some(true) {
        return Some(node);
    }
    if let Some(arr) = node.get("nodes").and_then(|v| v.as_array()) {
        for child in arr {
            if let Some(found) = find_focused(child) {
                return Some(found);
            }
        }
    }
    if let Some(arr) = node.get("floating_nodes").and_then(|v| v.as_array()) {
        for child in arr {
            if let Some(found) = find_focused(child) {
                return Some(found);
            }
        }
    }
    None
}

fn hint_from_node(node: &serde_json::Value) -> Option<FocusHint> {
    let app_id = node
        .get("app_id")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let class = node
        .pointer("/window_properties/class")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| app_id.clone());
    let title = node
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| {
            node.pointer("/window_properties/title")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        });
    let steam_app_id = class
        .as_deref()
        .and_then(winetop_core::parse_steam_app_class)
        .or_else(|| {
            app_id
                .as_deref()
                .and_then(winetop_core::parse_steam_app_class)
        });
    Some(FocusHint {
        app_id,
        class,
        title,
        steam_app_id,
    })
}
