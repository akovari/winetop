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
    label: String,
}

/// Flat navigable row in the sessions table (session header or child process).
#[derive(Debug, Clone, PartialEq, Eq)]
enum NavRow {
    Session { session_idx: usize },
    Process { session_idx: usize, proc_idx: usize },
}

pub struct App {
    snap: SessionSnapshot,
    tracker: CpuTracker,
    /// Index into `nav_rows()`.
    selected: usize,
    table_state: TableState,
    expanded: HashSet<String>,
    filter: String,
    filtering: bool,
    view: ViewMode,
    sort: SortKey,
    theme: Theme,
    detail: bool,
    kill_modal: Option<KillModal>,
    status: String,
    refresh_ms: u64,
    cpu_history: HashMap<String, Vec<f32>>,
    compact: bool,
    /// Prefer re-selecting this PID after refresh.
    sticky_pid: Option<u32>,
    sticky_session: Option<String>,
}

impl App {
    pub fn new(refresh_ms: u64) -> Self {
        let mut tracker = CpuTracker::new();
        let snap = scan_with(&mut tracker).unwrap_or_else(|_| SessionSnapshot {
            sessions: vec![],
            orphans: vec![],
            scanned_at: chrono::Utc::now(),
        });
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
            kill_modal: None,
            status: String::new(),
            refresh_ms,
            cpu_history: HashMap::new(),
            compact: false,
            sticky_pid: None,
            sticky_session: None,
        };
        // Auto-expand sole session so processes are immediately navigable.
        if app.snap.sessions.len() == 1 {
            app.expanded.insert(app.snap.sessions[0].id.clone());
        }
        app.record_history();
        app.clamp_selection();
        app
    }

    pub fn refresh(&mut self) {
        match scan_with(&mut self.tracker) {
            Ok(snap) => {
                self.snap = snap;
                self.record_history();
                self.restore_selection();
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

    fn visible_session_indices(&self) -> Vec<usize> {
        let mut idxs: Vec<usize> = (0..self.snap.sessions.len()).collect();
        let f = self.filter.to_ascii_lowercase();
        if !f.is_empty() {
            idxs.retain(|&i| {
                let s = &self.snap.sessions[i];
                s.name.to_ascii_lowercase().contains(&f)
                    || s.source.as_str().to_ascii_lowercase().contains(&f)
                    || s.short_prefix().to_ascii_lowercase().contains(&f)
                    || s.processes.iter().any(|p| {
                        p.display_name().to_ascii_lowercase().contains(&f)
                            || p.detail().to_ascii_lowercase().contains(&f)
                            || p.pid.to_string().contains(&f)
                    })
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

    fn nav_rows(&self) -> Vec<NavRow> {
        let mut rows = Vec::new();
        for session_idx in self.visible_session_indices() {
            let id = self.snap.sessions[session_idx].id.clone();
            rows.push(NavRow::Session { session_idx });
            if self.expanded.contains(&id) {
                let n = self.snap.sessions[session_idx].processes.len();
                for proc_idx in 0..n {
                    rows.push(NavRow::Process {
                        session_idx,
                        proc_idx,
                    });
                }
            }
        }
        rows
    }

    fn current_row(&self) -> Option<NavRow> {
        self.nav_rows().get(self.selected).cloned()
    }

    fn clamp_selection(&mut self) {
        let n = self.nav_rows().len();
        if n == 0 {
            self.selected = 0;
            self.table_state.select(None);
        } else {
            if self.selected >= n {
                self.selected = n - 1;
            }
            self.table_state.select(Some(self.selected));
        }
        self.remember_sticky();
    }

    fn restore_selection(&mut self) {
        let rows = self.nav_rows();
        if rows.is_empty() {
            self.selected = 0;
            self.table_state.select(None);
            return;
        }
        if let Some(pid) = self.sticky_pid {
            if let Some(i) = rows.iter().position(|r| match r {
                NavRow::Process {
                    session_idx,
                    proc_idx,
                } => self.snap.sessions[*session_idx].processes[*proc_idx].pid == pid,
                _ => false,
            }) {
                self.selected = i;
                self.table_state.select(Some(self.selected));
                return;
            }
        }
        if let Some(ref sid) = self.sticky_session {
            if let Some(i) = rows.iter().position(|r| match r {
                NavRow::Session { session_idx } => self.snap.sessions[*session_idx].id == *sid,
                _ => false,
            }) {
                self.selected = i;
                self.table_state.select(Some(self.selected));
                return;
            }
        }
        self.clamp_selection();
    }

    fn remember_sticky(&mut self) {
        match self.current_row() {
            Some(NavRow::Process {
                session_idx,
                proc_idx,
            }) => {
                let s = &self.snap.sessions[session_idx];
                self.sticky_pid = Some(s.processes[proc_idx].pid);
                self.sticky_session = Some(s.id.clone());
            }
            Some(NavRow::Session { session_idx }) => {
                self.sticky_pid = None;
                self.sticky_session = Some(self.snap.sessions[session_idx].id.clone());
            }
            None => {}
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
                KeyCode::Esc | KeyCode::Char('d') => self.detail = false,
                KeyCode::Char('c') => self.open_process_kill(),
                KeyCode::Up | KeyCode::Char('k') => {
                    self.move_sel(-1);
                    // Stay in detail on newly selected row
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.move_sel(1);
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
                self.clamp_selection();
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
            KeyCode::Up | KeyCode::Char('k') => self.move_sel(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_sel(1),
            KeyCode::Home => {
                self.selected = 0;
                self.table_state.select(Some(0));
                self.remember_sticky();
            }
            KeyCode::End => {
                let n = self.nav_rows().len();
                if n > 0 {
                    self.selected = n - 1;
                    self.table_state.select(Some(self.selected));
                    self.remember_sticky();
                }
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => {
                self.toggle_expand_selected();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                // Collapse parent session when on a process, else collapse selected session.
                if let Some(NavRow::Process { session_idx, .. }) = self.current_row() {
                    let id = self.snap.sessions[session_idx].id.clone();
                    self.expanded.remove(&id);
                    // Jump selection to session header
                    if let Some(i) = self.nav_rows().iter().position(
                        |r| matches!(r, NavRow::Session { session_idx: si } if *si == session_idx),
                    ) {
                        self.selected = i;
                        self.table_state.select(Some(i));
                    }
                    self.remember_sticky();
                } else {
                    self.toggle_expand_selected();
                }
            }
            KeyCode::Enter | KeyCode::Char('d') => {
                self.detail = true;
            }
            KeyCode::Char('c') | KeyCode::Delete => self.open_process_kill(),
            KeyCode::Char('K') => {
                if let Some(id) = self.selected_session_id() {
                    let name = self
                        .selected_session()
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| id.clone());
                    self.kill_modal = Some(KillModal {
                        kind: KillModalKind::Session,
                        method_idx: 0,
                        session_id: id,
                        pid: None,
                        label: name,
                    });
                }
            }
            KeyCode::Char('P') => {
                if let Some(id) = self.selected_session_id() {
                    let name = self
                        .selected_session()
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| id.clone());
                    self.kill_modal = Some(KillModal {
                        kind: KillModalKind::Prefix,
                        method_idx: 0,
                        session_id: id,
                        pid: None,
                        label: name,
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

    fn move_sel(&mut self, delta: isize) {
        let n = self.nav_rows().len() as isize;
        if n == 0 {
            return;
        }
        let next = (self.selected as isize + delta).clamp(0, n - 1) as usize;
        self.selected = next;
        self.table_state.select(Some(self.selected));
        self.remember_sticky();
    }

    fn toggle_expand_selected(&mut self) {
        let Some(row) = self.current_row() else {
            return;
        };
        let session_idx = match row {
            NavRow::Session { session_idx } | NavRow::Process { session_idx, .. } => session_idx,
        };
        let id = self.snap.sessions[session_idx].id.clone();
        if !self.expanded.remove(&id) {
            self.expanded.insert(id);
        }
        self.clamp_selection();
    }

    fn open_process_kill(&mut self) {
        let Some(row) = self.current_row() else {
            return;
        };
        match row {
            NavRow::Process {
                session_idx,
                proc_idx,
            } => {
                let s = &self.snap.sessions[session_idx];
                let p = &s.processes[proc_idx];
                self.kill_modal = Some(KillModal {
                    kind: KillModalKind::Process,
                    method_idx: 0,
                    session_id: s.id.clone(),
                    pid: Some(p.pid),
                    label: format!("{} ({})", p.display_name(), p.pid),
                });
            }
            NavRow::Session { .. } => {
                self.status =
                    "select a process row (Tab to expand, ↑↓ to move), then c to kill".into();
            }
        }
    }

    fn selected_session_id(&self) -> Option<String> {
        match self.current_row()? {
            NavRow::Session { session_idx } | NavRow::Process { session_idx, .. } => {
                Some(self.snap.sessions[session_idx].id.clone())
            }
        }
    }

    fn selected_session(&self) -> Option<&winetop_core::Session> {
        match self.current_row()? {
            NavRow::Session { session_idx } | NavRow::Process { session_idx, .. } => {
                Some(&self.snap.sessions[session_idx])
            }
        }
    }

    fn selected_process(&self) -> Option<&winetop_core::WineProcess> {
        match self.current_row()? {
            NavRow::Process {
                session_idx,
                proc_idx,
            } => self.snap.sessions[session_idx].processes.get(proc_idx),
            NavRow::Session { .. } => None,
        }
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
                self.detail = false;
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
        let on_proc = matches!(self.current_row(), Some(NavRow::Process { .. }));
        let hints = if on_proc {
            key_hint(
                &[
                    ("↑↓", "process"),
                    ("c", "kill proc"),
                    ("d", "detail"),
                    ("←", "collapse"),
                    ("K", "kill sess"),
                    ("P", "wineserver"),
                    ("?", "help"),
                    ("q", "quit"),
                ],
                self.theme.footer(),
            )
        } else {
            key_hint(
                &[
                    ("↑↓", "select"),
                    ("Tab/→", "expand"),
                    ("/", "filter"),
                    ("d", "detail"),
                    ("c", "kill proc"),
                    ("K", "kill sess"),
                    ("?", "help"),
                    ("q", "quit"),
                ],
                self.theme.footer(),
            )
        };
        let status = if self.status.is_empty() {
            hints
        } else {
            Line::from(Span::styled(format!(" {}", self.status), self.theme.warn()))
        };
        frame.render_widget(Paragraph::new(status), area);
    }

    fn draw_sessions(&mut self, frame: &mut Frame, area: Rect) {
        let nav = self.nav_rows();
        if nav.is_empty() {
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
        for row in &nav {
            match *row {
                NavRow::Session { session_idx } => {
                    let s = &self.snap.sessions[session_idx];
                    let ws = if s.wineserver_alive { "●" } else { "○" };
                    let spark = self
                        .cpu_history
                        .get(&s.id)
                        .map(|h| sparkline(h, 10))
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
                            Cell::from("—"),
                            Cell::from("—"),
                            Cell::from(format!("{:.0}%", s.cpu_percent)),
                            Cell::from(format_bytes(s.rss_bytes)),
                            Cell::from(format!("{ws} {}", s.process_count())),
                            Cell::from(format!("{} {}", s.short_prefix(), spark))
                                .style(self.theme.spark()),
                        ])
                        .style(Style::default().add_modifier(Modifier::BOLD)),
                    );
                }
                NavRow::Process {
                    session_idx,
                    proc_idx,
                } => {
                    let s = &self.snap.sessions[session_idx];
                    let p = &s.processes[proc_idx];
                    let last = proc_idx + 1 == s.processes.len();
                    let branch = if last { "└─" } else { "├─" };
                    let root = if p.is_session_root { "*" } else { " " };
                    let detail = truncate(&p.detail(), 56);
                    rows.push(Row::new(vec![
                        Cell::from(format!("  {branch}{}", p.kind.as_str())),
                        Cell::from(format!("{root}{}", p.display_name())),
                        Cell::from(p.pid.to_string()),
                        Cell::from(p.ppid.to_string()),
                        Cell::from(format!("{:.0}%", p.cpu_percent)),
                        Cell::from(format_bytes(p.rss_bytes)),
                        Cell::from(p.state.clone()),
                        Cell::from(detail),
                    ]));
                }
            }
        }

        let header = Row::new(["KIND", "NAME", "PID", "PPID", "CPU", "RSS", "ST", "DETAIL"])
            .style(Style::default().add_modifier(Modifier::BOLD));
        let table = Table::new(
            rows,
            [
                Constraint::Length(12),
                Constraint::Length(22),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(5),
                Constraint::Length(7),
                Constraint::Length(6),
                Constraint::Min(20),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("sessions  (Tab expand · ↑↓ processes · c kill)"),
        )
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
            Line::from(" ↑↓ / j k   move selection (sessions and processes)"),
            Line::from(" Tab / →    expand session (show processes)"),
            Line::from(" ← / h      collapse session"),
            Line::from(" c / Del    kill selected process (confirm)"),
            Line::from(" K          kill whole session (confirm)"),
            Line::from(" P          wineserver -k for prefix (confirm)"),
            Line::from(" d / Enter  detail drawer for selection"),
            Line::from(" /          filter (name, pid, cmdline)"),
            Line::from(" F          cycle sort"),
            Line::from(" t / o / ?  tree / orphans / help"),
            Line::from(" T          theme · z compact · r refresh · q quit"),
            Line::from(""),
            Line::from(" DETAIL column shows Windows path + args to tell"),
            Line::from(" duplicate names (e.g. many Battle.net.exe) apart."),
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
        if let Some(p) = self.selected_process() {
            lines.push(Line::from(Span::styled(
                format!(" Process    {} [{}]", p.display_name(), p.kind.as_str()),
                self.theme.header(),
            )));
            lines.push(Line::from(format!(
                " PID/PPID   {} / {}   state {}   root={}",
                p.pid, p.ppid, p.state, p.is_session_root
            )));
            lines.push(Line::from(format!(
                " CPU/RSS    {:.1}% · {} · {} threads",
                p.cpu_percent,
                format_bytes(p.rss_bytes),
                p.threads
            )));
            if let Some(ref exe) = p.exe_path {
                lines.push(Line::from(format!(" Exe        {}", exe.display())));
            }
            if let Some(ref cwd) = p.cwd {
                lines.push(Line::from(format!(" Cwd        {}", cwd.display())));
            }
            lines.push(Line::from(format!(" Detail     {}", p.detail())));
            lines.push(Line::from(format!(
                " Cmdline    {}",
                truncate(&p.cmdline, 120)
            )));
            lines.push(Line::from(format!(
                " Environ    {}",
                p.environ_keys.join(" ")
            )));
        } else {
            lines.push(Line::from(
                " (session selected — expand and pick a process for PID details)",
            ));
            lines.push(Line::from(" Top processes:"));
            for p in s.processes.iter().take(8) {
                lines.push(Line::from(format!(
                    "   {:>7}  {:<18}  {:>5.0}%  {:>6}  {}",
                    p.pid,
                    truncate(p.display_name(), 18),
                    p.cpu_percent,
                    format_bytes(p.rss_bytes),
                    truncate(&p.detail(), 40)
                )));
            }
        }
        for n in &s.notes {
            lines.push(Line::from(Span::styled(
                format!(" Note       {n}"),
                self.theme.warn(),
            )));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(
            " Esc/d close · c kill process · ↑↓ move selection",
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
        let mut lines = vec![
            Line::from(format!(" Target: {}", modal.label)),
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
