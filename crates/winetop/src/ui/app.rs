use super::theme::{Theme, ThemeId};
use super::widgets::{draw_modal, key_hint, sparkline};
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap};
use ratatui::Frame;
use std::collections::{HashMap, HashSet};
use winetop_core::classify::process_tree_lines;
use winetop_core::kill::{self, KillMethod, KillRequest};
use winetop_core::metrics::CpuTracker;
use winetop_core::util::format_bytes;
use winetop_core::{scan_with, SessionSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Sessions,
    Tree,
    Orphans,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortKey {
    Cpu,
    Rss,
    Name,
    Source,
}

impl SortKey {
    fn next(self) -> Self {
        match self {
            Self::Cpu => Self::Rss,
            Self::Rss => Self::Name,
            Self::Name => Self::Source,
            Self::Source => Self::Cpu,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KillModalKind {
    Session,
    Prefix,
    Process,
}

struct KillModal {
    kind: KillModalKind,
    method_idx: usize,
    session_id: String,
    pid: Option<u32>,
}

pub struct App {
    snap: SessionSnapshot,
    tracker: CpuTracker,
    selected: usize,
    table_state: TableState,
    expanded: HashSet<String>,
    filter: String,
    filtering: bool,
    view: ViewMode,
    sort: SortKey,
    theme: Theme,
    detail: bool,
    detail_proc_idx: usize,
    kill_modal: Option<KillModal>,
    status: String,
    refresh_ms: u64,
    cpu_history: HashMap<String, Vec<f32>>,
    compact: bool,
}

impl App {
    pub fn new(refresh_ms: u64) -> Self {
        let mut tracker = CpuTracker::new();
        let snap = scan_with(&mut tracker).unwrap_or_else(|_| SessionSnapshot {
            sessions: vec![],
            orphans: vec![],
            scanned_at: chrono::Utc::now(),
        });
        // Warm CPU sample
        let _ = scan_with(&mut tracker);
        let snap = scan_with(&mut tracker).unwrap_or(snap);
        let mut app = Self {
            snap,
            tracker,
            selected: 0,
            table_state: TableState::default().with_selected(Some(0)),
            expanded: HashSet::new(),
            filter: String::new(),
            filtering: false,
            view: ViewMode::Sessions,
            sort: SortKey::Cpu,
            theme: Theme::new(ThemeId::Default),
            detail: false,
            detail_proc_idx: 0,
            kill_modal: None,
            status: String::new(),
            refresh_ms,
            cpu_history: HashMap::new(),
            compact: false,
        };
        app.record_history();
        app
    }

    pub fn refresh(&mut self) {
        match scan_with(&mut self.tracker) {
            Ok(snap) => {
                self.snap = snap;
                self.record_history();
                self.clamp_selection();
            }
            Err(e) => self.status = format!("scan error: {e}"),
        }
    }

    fn record_history(&mut self) {
        for s in &self.snap.sessions {
            let h = self.cpu_history.entry(s.id.clone()).or_default();
            h.push(s.cpu_percent);
            if h.len() > 60 {
                h.remove(0);
            }
        }
    }

    fn visible_sessions(&self) -> Vec<usize> {
        let mut idxs: Vec<usize> = (0..self.snap.sessions.len()).collect();
        let f = self.filter.to_ascii_lowercase();
        if !f.is_empty() {
            idxs.retain(|&i| {
                let s = &self.snap.sessions[i];
                s.name.to_ascii_lowercase().contains(&f)
                    || s.source.as_str().to_ascii_lowercase().contains(&f)
                    || s.short_prefix().to_ascii_lowercase().contains(&f)
                    || s.steam_app_id
                        .map(|id| id.to_string().contains(&f))
                        .unwrap_or(false)
            });
        }
        idxs.sort_by(|&a, &b| {
            let sa = &self.snap.sessions[a];
            let sb = &self.snap.sessions[b];
            match self.sort {
                SortKey::Cpu => sb
                    .cpu_percent
                    .partial_cmp(&sa.cpu_percent)
                    .unwrap_or(std::cmp::Ordering::Equal),
                SortKey::Rss => sb.rss_bytes.cmp(&sa.rss_bytes),
                SortKey::Name => sa.name.cmp(&sb.name),
                SortKey::Source => sa.source.as_str().cmp(sb.source.as_str()),
            }
        });
        idxs
    }

    fn clamp_selection(&mut self) {
        let n = self.visible_sessions().len();
        if n == 0 {
            self.selected = 0;
            self.table_state.select(None);
        } else {
            if self.selected >= n {
                self.selected = n - 1;
            }
            self.table_state.select(Some(self.selected));
        }
    }

    /// Returns true if the app should quit.
    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        if self.filtering {
            match code {
                KeyCode::Esc => {
                    self.filtering = false;
                }
                KeyCode::Enter => {
                    self.filtering = false;
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                    self.clamp_selection();
                }
                KeyCode::Char(c) => {
                    self.filter.push(c);
                    self.clamp_selection();
                }
                _ => {}
            }
            return false;
        }

        if let Some(ref mut modal) = self.kill_modal {
            match code {
                KeyCode::Esc => self.kill_modal = None,
                KeyCode::Left | KeyCode::Char('h') => {
                    if modal.method_idx > 0 {
                        modal.method_idx -= 1;
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    let max = methods_for(modal.kind).len();
                    if modal.method_idx + 1 < max {
                        modal.method_idx += 1;
                    }
                }
                KeyCode::Enter => {
                    let modal = self.kill_modal.take().unwrap();
                    self.do_kill(modal);
                }
                KeyCode::Char('1') => {
                    modal.method_idx = 0;
                }
                KeyCode::Char('2') if methods_for(modal.kind).len() > 1 => {
                    modal.method_idx = 1;
                }
                KeyCode::Char('3') if methods_for(modal.kind).len() > 2 => {
                    modal.method_idx = 2;
                }
                _ => {}
            }
            return false;
        }

        if self.detail {
            match code {
                KeyCode::Esc | KeyCode::Char('d') | KeyCode::Enter => self.detail = false,
                KeyCode::Char('c') => {
                    if let Some(pid) = self.selected_proc_pid() {
                        self.kill_modal = Some(KillModal {
                            kind: KillModalKind::Process,
                            method_idx: 0,
                            session_id: self.selected_session_id().unwrap_or_default(),
                            pid: Some(pid),
                        });
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.detail_proc_idx = self.detail_proc_idx.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(s) = self.selected_session() {
                        if self.detail_proc_idx + 1 < s.processes.len() {
                            self.detail_proc_idx += 1;
                        }
                    }
                }
                _ => {}
            }
            return false;
        }

        match code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Char('/') => self.filtering = true,
            KeyCode::Char('F') => {
                self.sort = self.sort.next();
                self.status = format!("sort: {:?}", self.sort);
            }
            KeyCode::Char('t') => {
                self.view = if self.view == ViewMode::Tree {
                    ViewMode::Sessions
                } else {
                    ViewMode::Tree
                };
            }
            KeyCode::Char('o') => {
                self.view = if self.view == ViewMode::Orphans {
                    ViewMode::Sessions
                } else {
                    ViewMode::Orphans
                };
            }
            KeyCode::Char('?') => {
                self.view = if self.view == ViewMode::Help {
                    ViewMode::Sessions
                } else {
                    ViewMode::Help
                };
            }
            KeyCode::Char('T') => {
                self.theme = Theme::new(self.theme.id.next());
                self.status = format!("theme: {}", self.theme.id.name());
            }
            KeyCode::Char('z') => {
                self.compact = !self.compact;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                self.table_state.select(Some(self.selected));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let n = self.visible_sessions().len();
                if n > 0 && self.selected + 1 < n {
                    self.selected += 1;
                    self.table_state.select(Some(self.selected));
                }
            }
            KeyCode::Tab => {
                if let Some(id) = self.selected_session_id() {
                    if !self.expanded.remove(&id) {
                        self.expanded.insert(id);
                    }
                }
            }
            KeyCode::Enter | KeyCode::Char('d') => {
                self.detail = true;
                self.detail_proc_idx = 0;
            }
            KeyCode::Char('c') => {
                if let Some(pid) = self.selected_session().and_then(|s| {
                    s.processes
                        .iter()
                        .find(|p| p.windows_image.is_some())
                        .map(|p| p.pid)
                        .or_else(|| s.processes.first().map(|p| p.pid))
                }) {
                    self.kill_modal = Some(KillModal {
                        kind: KillModalKind::Process,
                        method_idx: 0,
                        session_id: self.selected_session_id().unwrap_or_default(),
                        pid: Some(pid),
                    });
                }
            }
            KeyCode::Char('K') => {
                if let Some(id) = self.selected_session_id() {
                    self.kill_modal = Some(KillModal {
                        kind: KillModalKind::Session,
                        method_idx: 0,
                        session_id: id,
                        pid: None,
                    });
                }
            }
            KeyCode::Char('P') => {
                if let Some(id) = self.selected_session_id() {
                    self.kill_modal = Some(KillModal {
                        kind: KillModalKind::Prefix,
                        method_idx: 0,
                        session_id: id,
                        pid: None,
                    });
                }
            }
            KeyCode::Esc => {
                self.view = ViewMode::Sessions;
                self.filter.clear();
            }
            _ => {}
        }
        false
    }

    fn selected_session_id(&self) -> Option<String> {
        let vis = self.visible_sessions();
        vis.get(self.selected)
            .map(|&i| self.snap.sessions[i].id.clone())
    }

    fn selected_session(&self) -> Option<&winetop_core::Session> {
        let vis = self.visible_sessions();
        vis.get(self.selected).map(|&i| &self.snap.sessions[i])
    }

    fn selected_proc_pid(&self) -> Option<u32> {
        self.selected_session()
            .and_then(|s| s.processes.get(self.detail_proc_idx).map(|p| p.pid))
    }

    fn do_kill(&mut self, modal: KillModal) {
        let methods = methods_for(modal.kind);
        let method = methods[modal.method_idx.min(methods.len() - 1)];
        let req = KillRequest {
            method,
            pid: modal.pid,
            session_id: Some(modal.session_id),
            steam_app_id: None,
            prefix: None,
        };
        match kill::execute(&req, &self.snap) {
            Ok(r) => {
                self.status = r.message;
                self.refresh();
            }
            Err(e) => self.status = format!("kill failed: {e}"),
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(if self.compact { 1 } else { 2 }),
        ])
        .split(area);

        self.draw_header(frame, chunks[0]);
        match self.view {
            ViewMode::Sessions => self.draw_sessions(frame, chunks[1]),
            ViewMode::Tree => self.draw_tree(frame, chunks[1]),
            ViewMode::Orphans => self.draw_orphans(frame, chunks[1]),
            ViewMode::Help => self.draw_help(frame, chunks[1]),
        }
        self.draw_footer(frame, chunks[2]);

        if self.detail {
            self.draw_detail(frame, area);
        }
        if self.kill_modal.is_some() {
            self.draw_kill_modal(frame, area);
        }
    }

    fn draw_header(&self, frame: &mut Frame, area: Rect) {
        let (procs, cpu, rss) = self.snap.totals();
        let filter = if self.filtering || !self.filter.is_empty() {
            format!("  filter:/{}/", self.filter)
        } else {
            String::new()
        };
        let title = Line::from(vec![
            Span::styled(
                format!(" winetop {} ", env!("CARGO_PKG_VERSION")),
                self.theme.header(),
            ),
            Span::styled(
                format!(
                    "│ {} sessions │ {} procs │ {:.0}% cpu │ {} │ refresh {}ms │ {}{}",
                    self.snap.sessions.len(),
                    procs,
                    cpu,
                    format_bytes(rss),
                    self.refresh_ms,
                    self.theme.id.name(),
                    filter
                ),
                self.theme.normal(),
            ),
        ]);
        frame.render_widget(Paragraph::new(title), area);
    }

    fn draw_footer(&self, frame: &mut Frame, area: Rect) {
        let hints = key_hint(
            &[
                ("↑↓", "select"),
                ("Tab", "expand"),
                ("/", "filter"),
                ("d", "detail"),
                ("K", "kill sess"),
                ("P", "wineserver"),
                ("t", "tree"),
                ("o", "orphans"),
                ("T", "theme"),
                ("?", "help"),
                ("q", "quit"),
            ],
            self.theme.footer(),
        );
        let status = if self.status.is_empty() {
            hints
        } else {
            Line::from(Span::styled(format!(" {}", self.status), self.theme.warn()))
        };
        frame.render_widget(Paragraph::new(status), area);
    }

    fn draw_sessions(&mut self, frame: &mut Frame, area: Rect) {
        let vis = self.visible_sessions();
        if vis.is_empty() {
            let msg = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    " No Wine/Proton sessions detected.",
                    self.theme.header(),
                )),
                Line::from(""),
                Line::from(" Tips: launch a game via Steam/Lutris, or: wine notepad"),
                Line::from(" Docs: https://github.com/akovari/winetop"),
            ])
            .block(Block::default().borders(Borders::ALL).title("sessions"));
            frame.render_widget(msg, area);
            return;
        }

        let mut rows = Vec::new();
        for &idx in &vis {
            let s = &self.snap.sessions[idx];
            let ws = if s.wineserver_alive { "●" } else { "○" };
            let spark = self
                .cpu_history
                .get(&s.id)
                .map(|h| sparkline(h, 12))
                .unwrap_or_default();
            let app = s
                .steam_app_id
                .map(|id| format!("#{id}"))
                .unwrap_or_default();
            let name = if app.is_empty() {
                s.name.clone()
            } else {
                format!("{} {app}", s.name)
            };
            let expanded = self.expanded.contains(&s.id);
            let marker = if expanded { "▼" } else { "▶" };
            rows.push(
                Row::new(vec![
                    Cell::from(format!("{marker} {}", s.source.as_str()))
                        .style(self.theme.source(s.source)),
                    Cell::from(name),
                    Cell::from(s.short_prefix()),
                    Cell::from(format!("{:.0}%", s.cpu_percent)),
                    Cell::from(format_bytes(s.rss_bytes)),
                    Cell::from(s.process_count().to_string()),
                    Cell::from(ws.to_string()),
                    Cell::from(spark).style(self.theme.spark()),
                ])
                .height(1),
            );
            if expanded {
                for p in &s.processes {
                    let label = p.windows_image.clone().unwrap_or_else(|| p.name.clone());
                    let root = if p.is_session_root { " *" } else { "" };
                    rows.push(Row::new(vec![
                        Cell::from(""),
                        Cell::from(format!("  ├─ {label}{root}")),
                        Cell::from(format!("pid {}", p.pid)),
                        Cell::from(format!("{:.0}%", p.cpu_percent)),
                        Cell::from(format_bytes(p.rss_bytes)),
                        Cell::from(p.kind.as_str()),
                        Cell::from(p.state.clone()),
                        Cell::from(""),
                    ]));
                }
            }
        }

        let header = Row::new(["SRC", "SESSION", "PREFIX", "CPU", "RSS", "N", "WS", "HIST"])
            .style(Style::default().add_modifier(Modifier::BOLD));
        let table = Table::new(
            rows,
            [
                Constraint::Length(10),
                Constraint::Min(20),
                Constraint::Min(16),
                Constraint::Length(5),
                Constraint::Length(7),
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Length(12),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("sessions"))
        .row_highlight_style(self.theme.selected())
        .highlight_symbol("> ");

        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn draw_tree(&self, frame: &mut Frame, area: Rect) {
        let lines = process_tree_lines(&self.snap.sessions);
        let text: Vec<Line> = if lines.is_empty() {
            vec![Line::from(" No Wine-related process tree.")]
        } else {
            lines.into_iter().map(Line::from).collect()
        };
        frame.render_widget(
            Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title("tree"))
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    fn draw_orphans(&self, frame: &mut Frame, area: Rect) {
        let text: Vec<Line> = if self.snap.orphans.is_empty() {
            vec![Line::from(" No orphans detected.")]
        } else {
            self.snap
                .orphans
                .iter()
                .map(|o| {
                    Line::from(Span::styled(
                        format!(" [{}] {}", o.kind, o.detail),
                        self.theme.warn(),
                    ))
                })
                .collect()
        };
        frame.render_widget(
            Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("orphans")),
            area,
        );
    }

    fn draw_help(&self, frame: &mut Frame, area: Rect) {
        let lines = vec![
            Line::from(Span::styled(" winetop keymap", self.theme.header())),
            Line::from(""),
            Line::from(" q          quit"),
            Line::from(" r          refresh"),
            Line::from(" /          filter sessions"),
            Line::from(" F          cycle sort (cpu/rss/name/source)"),
            Line::from(" Tab        expand/collapse session"),
            Line::from(" d/Enter    detail drawer"),
            Line::from(" c          kill process (confirm)"),
            Line::from(" K          kill session (confirm)"),
            Line::from(" P          wineserver -k for prefix (confirm)"),
            Line::from(" t          process tree"),
            Line::from(" o          orphans"),
            Line::from(" T          cycle theme"),
            Line::from(" z          compact footer"),
            Line::from(""),
            Line::from(" Steam games: prefer SIGTERM reaper, then wineserver -k."),
            Line::from(" Never kills the Steam client."),
        ];
        frame.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("help")),
            area,
        );
    }

    fn draw_detail(&self, frame: &mut Frame, area: Rect) {
        let Some(s) = self.selected_session() else {
            return;
        };
        let p = s.processes.get(self.detail_proc_idx);
        let mut lines = vec![
            Line::from(format!(
                " Source     {} {}",
                s.source,
                s.runner.as_deref().unwrap_or("")
            )),
            Line::from(format!(
                " Session    {}{}",
                s.name,
                s.steam_app_id
                    .map(|id| format!(" · AppId {id}"))
                    .unwrap_or_default()
            )),
            Line::from(format!(
                " Prefix     {}",
                s.prefix
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "-".into())
            )),
            Line::from(format!(
                " Aggregate  {:.1}% cpu · {} · {} procs · wineserver {}",
                s.cpu_percent,
                format_bytes(s.rss_bytes),
                s.process_count(),
                if s.wineserver_alive { "alive" } else { "no" }
            )),
            Line::from(""),
        ];
        if let Some(p) = p {
            lines.push(Line::from(format!(
                " Process    {}  pid {}  ppid {}",
                p.windows_image.as_deref().unwrap_or(&p.name),
                p.pid,
                p.ppid
            )));
            lines.push(Line::from(format!(
                " CPU/RSS    {:.1}% · {} · {} threads · {}",
                p.cpu_percent,
                format_bytes(p.rss_bytes),
                p.threads,
                p.state
            )));
            lines.push(Line::from(format!(
                " Cmdline    {}",
                truncate(&p.cmdline, 100)
            )));
            lines.push(Line::from(format!(
                " Environ    {}",
                p.environ_keys.join(" ")
            )));
        }
        for n in &s.notes {
            lines.push(Line::from(Span::styled(
                format!(" Note       {n}"),
                self.theme.warn(),
            )));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(
            " Esc/d close · c kill process · ↑↓ switch process",
        ));
        draw_modal(frame, area, " detail ", lines, self.theme.header());
    }

    fn draw_kill_modal(&self, frame: &mut Frame, area: Rect) {
        let Some(modal) = &self.kill_modal else {
            return;
        };
        let methods = methods_for(modal.kind);
        let labels: Vec<&str> = match modal.kind {
            KillModalKind::Session => vec![
                "(1) SIGTERM reaper / graceful",
                "(2) wineserver -k",
                "(3) SIGKILL all in session",
            ],
            KillModalKind::Prefix => vec!["(1) wineserver -k", "(2) SIGKILL all in session"],
            KillModalKind::Process => vec!["(1) SIGTERM", "(2) SIGKILL"],
        };
        let session_name = self
            .snap
            .find_session(&modal.session_id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| modal.session_id.clone());
        let mut lines = vec![
            Line::from(format!(" Target: {session_name}")),
            Line::from(match modal.pid {
                Some(pid) => format!(" PID: {pid}"),
                None => format!(" Kind: {:?}", modal.kind),
            }),
            Line::from(""),
        ];
        for (i, label) in labels.iter().enumerate() {
            let mark = if i == modal.method_idx { ">" } else { " " };
            lines.push(Line::from(format!(" {mark} {label}")));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(" ←→ choose · Enter confirm · Esc cancel"));
        let _ = methods;
        draw_modal(frame, area, " confirm kill ", lines, self.theme.warn());
    }
}

fn methods_for(kind: KillModalKind) -> &'static [KillMethod] {
    match kind {
        KillModalKind::Session => &[
            KillMethod::SessionReaper,
            KillMethod::WineServerK,
            KillMethod::SessionKill,
        ],
        KillModalKind::Prefix => &[KillMethod::WineServerK, KillMethod::SessionKill],
        KillModalKind::Process => &[KillMethod::ProcessTerm, KillMethod::ProcessKill],
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}
