use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
        MouseButton, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
};
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::projects::ProjectStat;
use crate::summarizer::{self, ActivityData};

// ── Config field indices ─────────────────────────────────────────────────────
const CF_PROVIDER: usize = 0;
const CF_MODEL: usize = 1;
const CF_URL_OR_KEY: usize = 2;
const CF_LANG: usize = 3;
const CF_PROMPT: usize = 4;
const CF_MACHINE: usize = 5;
const CF_NOTION_TOKEN: usize = 6;
const CF_NOTION_PAGE: usize = 7;
const CF_SLACK: usize = 8;
const CF_ADD_BLOCK: usize = 9;
// CF_ADD_BLOCK + 1 + i  →  blocked_patterns[i]

const PROVIDERS: &[&str] = &[
    "ollama",
    "openai",
    "anthropic",
    "groq",
    "gemini",
    "openrouter",
];
const LANGS: &[&str] = &["pt-br", "en", "es", "fr", "de", "ja", "zh"];

#[derive(PartialEq, Clone, Copy)]
enum ActiveTab {
    Activities,
    Summary,
    Projects,
    Config,
}

#[derive(PartialEq)]
enum ConfigEditMode {
    Browse,
    Editing(String, usize), // (buffer, cursor char index)
}

enum SummaryState {
    Empty,
    Loading,
    Cached(String),
    Done(String),
    Error(String),
}

struct App {
    date: chrono::NaiveDate,
    data: Option<ActivityData>,
    summary: SummaryState,
    active_tab: ActiveTab,
    scroll: u16,
    tick: u8,
    model: String,
    provider: String,
    ollama_url: String,
    api_key: Option<String>,
    lang: String,
    projects: Option<Vec<ProjectStat>>,
    projects_days: u32,
    config: Config,
    config_cursor: usize,
    config_edit: ConfigEditMode,
    // linha lógica (com scroll) → índice do campo Config; usize::MAX = não-campo
    config_row_map: Vec<usize>,
    last_size: Rect,
}

impl App {
    fn new(opts: TuiOptions) -> Self {
        let config = Config::load().unwrap_or_default();
        Self {
            date: chrono::Local::now().date_naive(),
            data: None,
            summary: SummaryState::Empty,
            active_tab: ActiveTab::Activities,
            scroll: 0,
            tick: 0,
            model: opts.model,
            provider: opts.provider,
            ollama_url: opts.ollama_url,
            api_key: opts.api_key,
            lang: opts.lang,
            projects: None,
            projects_days: 7,
            config,
            config_cursor: 0,
            config_edit: ConfigEditMode::Browse,
            config_row_map: Vec::new(),
            last_size: Rect::default(),
        }
    }

    fn reload_config(&mut self) {
        if let Ok(cfg) = Config::load() {
            self.config = cfg;
        }
        self.scroll = 0;
        self.config_cursor = 0;
        self.config_edit = ConfigEditMode::Browse;
    }

    fn config_item_count(&self) -> usize {
        CF_ADD_BLOCK + 1 + self.config.blocked_patterns.len()
    }

    fn load_data(&mut self) {
        let d = summarizer::load_for_date(self.date).unwrap_or(ActivityData {
            dates: vec![],
            commands: vec![],
            top_apps: vec![],
            tabs: vec![],
            repos: vec![],
            tags: vec![],
        });
        let has_data = !d.commands.is_empty()
            || !d.top_apps.is_empty()
            || !d.tabs.is_empty()
            || !d.repos.is_empty()
            || !d.tags.is_empty();
        self.data = if has_data { Some(d) } else { None };
        self.scroll = 0;
        self.summary = match summarizer::load_summary(self.date) {
            Some(text) => SummaryState::Cached(text),
            None => SummaryState::Empty,
        };
    }

    fn prev_day(&mut self) {
        self.date -= chrono::Duration::days(1);
        self.load_data();
    }

    fn next_day(&mut self) {
        if self.date < chrono::Local::now().date_naive() {
            self.date += chrono::Duration::days(1);
            self.load_data();
        }
    }

    fn ensure_projects_loaded(&mut self) {
        if self.projects.is_none() {
            self.projects =
                Some(crate::projects::load_stats(self.projects_days).unwrap_or_default());
        }
    }

    fn reload_projects(&mut self) {
        self.projects = Some(crate::projects::load_stats(self.projects_days).unwrap_or_default());
    }
}

pub struct TuiOptions {
    pub model: String,
    pub provider: String,
    pub ollama_url: String,
    pub api_key: Option<String>,
    pub lang: String,
}

pub async fn run(opts: TuiOptions) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(opts);
    app.load_data();

    let (tx, mut rx) = mpsc::channel::<Result<String, String>>(1);
    let result = event_loop(&mut terminal, &mut app, &tx, &mut rx).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    tx: &mpsc::Sender<Result<String, String>>,
    rx: &mut mpsc::Receiver<Result<String, String>>,
) -> Result<()> {
    loop {
        terminal.draw(|f| render(f, app))?;
        app.tick = app.tick.wrapping_add(1);

        if let Ok(result) = rx.try_recv() {
            match result {
                Ok(summary) => {
                    summarizer::save_summary(app.date, &summary);
                    app.summary = SummaryState::Done(summary);
                }
                Err(e) => app.summary = SummaryState::Error(e),
            }
        }

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
            Event::Mouse(mouse) => {
                handle_mouse(app, mouse);
                continue;
            }
            Event::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                // ── Config tab gets full key control ────────────────────
                if app.active_tab == ActiveTab::Config {
                    let editing = matches!(app.config_edit, ConfigEditMode::Editing(_, _));
                    if editing {
                        match key.code {
                            KeyCode::Esc => {
                                app.config_edit = ConfigEditMode::Browse;
                            }
                            KeyCode::Enter => {
                                let old =
                                    std::mem::replace(&mut app.config_edit, ConfigEditMode::Browse);
                                if let ConfigEditMode::Editing(buf, _) = old {
                                    cfg_apply(&mut app.config, app.config_cursor, buf);
                                    let _ = app.config.save();
                                    if app.config_cursor == CF_ADD_BLOCK {
                                        app.config_cursor =
                                            CF_ADD_BLOCK + app.config.blocked_patterns.len();
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                if let ConfigEditMode::Editing(b, cur) = &mut app.config_edit {
                                    edit_cursor_backspace(b, cur);
                                }
                            }
                            KeyCode::Delete => {
                                if let ConfigEditMode::Editing(b, cur) = &mut app.config_edit {
                                    edit_cursor_delete(b, *cur);
                                }
                            }
                            KeyCode::Up => {
                                if let ConfigEditMode::Editing(b, cur) = &mut app.config_edit {
                                    *cur = edit_cursor_up(b, *cur);
                                }
                            }
                            KeyCode::Down => {
                                if let ConfigEditMode::Editing(b, cur) = &mut app.config_edit {
                                    *cur = edit_cursor_down(b, *cur);
                                }
                            }
                            KeyCode::Left => {
                                if let ConfigEditMode::Editing(_, cur) = &mut app.config_edit {
                                    *cur = cur.saturating_sub(1);
                                }
                            }
                            KeyCode::Right => {
                                if let ConfigEditMode::Editing(b, cur) = &mut app.config_edit {
                                    let max = b.chars().count();
                                    if *cur < max { *cur += 1; }
                                }
                            }
                            KeyCode::Home => {
                                if let ConfigEditMode::Editing(b, cur) = &mut app.config_edit {
                                    // Ctrl+Home → posição 0; Home → início da linha atual
                                    if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                                        *cur = 0;
                                    } else {
                                        *cur = edit_line_start(b, *cur);
                                    }
                                }
                            }
                            KeyCode::End => {
                                if let ConfigEditMode::Editing(b, cur) = &mut app.config_edit {
                                    if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                                        *cur = b.chars().count();
                                    } else {
                                        *cur = edit_line_end(b, *cur);
                                    }
                                }
                            }
                            KeyCode::Char(c) => {
                                if let ConfigEditMode::Editing(b, cur) = &mut app.config_edit {
                                    edit_cursor_insert(b, cur, c);
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // Browse mode
                    let total = app.config_item_count();
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Tab => {
                            app.active_tab = ActiveTab::Activities;
                            app.scroll = 0;
                        }
                        KeyCode::Char('1') => {
                            app.active_tab = ActiveTab::Activities;
                            app.scroll = 0;
                        }
                        KeyCode::Char('2') => {
                            app.active_tab = ActiveTab::Summary;
                            app.scroll = 0;
                        }
                        KeyCode::Char('3') => {
                            app.ensure_projects_loaded();
                            app.active_tab = ActiveTab::Projects;
                            app.scroll = 0;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if app.config_cursor + 1 < total {
                                app.config_cursor += 1;
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.config_cursor = app.config_cursor.saturating_sub(1);
                        }
                        // Cycle provider / lang with ←/→
                        KeyCode::Right | KeyCode::Char('l') => {
                            if app.config_cursor == CF_PROVIDER {
                                cfg_cycle(&mut app.config.provider, PROVIDERS, 1);
                                let _ = app.config.save();
                            } else if app.config_cursor == CF_LANG {
                                cfg_cycle(&mut app.config.lang, LANGS, 1);
                                let _ = app.config.save();
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            if app.config_cursor == CF_PROVIDER {
                                cfg_cycle(&mut app.config.provider, PROVIDERS, -1);
                                let _ = app.config.save();
                            } else if app.config_cursor == CF_LANG {
                                cfg_cycle(&mut app.config.lang, LANGS, -1);
                                let _ = app.config.save();
                            }
                        }
                        // Enter / e → start editing
                        KeyCode::Enter | KeyCode::Char('e') => {
                            if let Some(val) = cfg_initial_value(&app.config, app.config_cursor) {
                                let end = val.chars().count();
                                app.config_edit = ConfigEditMode::Editing(val, end);
                            }
                        }
                        // d / Delete → remove blocked pattern
                        KeyCode::Char('d') | KeyCode::Delete => {
                            let idx = app.config_cursor.saturating_sub(CF_ADD_BLOCK + 1);
                            if app.config_cursor > CF_ADD_BLOCK
                                && idx < app.config.blocked_patterns.len()
                            {
                                app.config.blocked_patterns.remove(idx);
                                let _ = app.config.save();
                                let new_total = app.config_item_count();
                                if app.config_cursor >= new_total {
                                    app.config_cursor = new_total.saturating_sub(1);
                                }
                            }
                        }
                        KeyCode::Char('R') => app.reload_config(),
                        _ => {}
                    }
                    continue;
                }
                // ── Other tabs ───────────────────────────────────────────

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,

                    KeyCode::Left | KeyCode::Char('h') => app.prev_day(),
                    KeyCode::Right | KeyCode::Char('l') => app.next_day(),

                    KeyCode::Tab => {
                        app.active_tab = match app.active_tab {
                            ActiveTab::Activities => ActiveTab::Summary,
                            ActiveTab::Summary => {
                                app.ensure_projects_loaded();
                                ActiveTab::Projects
                            }
                            ActiveTab::Projects => {
                                app.reload_config();
                                ActiveTab::Config
                            }
                            ActiveTab::Config => ActiveTab::Activities,
                        };
                        app.scroll = 0;
                    }
                    KeyCode::Char('1') => {
                        app.active_tab = ActiveTab::Activities;
                        app.scroll = 0;
                    }
                    KeyCode::Char('2') => {
                        app.active_tab = ActiveTab::Summary;
                        app.scroll = 0;
                    }
                    KeyCode::Char('3') => {
                        app.ensure_projects_loaded();
                        app.active_tab = ActiveTab::Projects;
                        app.scroll = 0;
                    }
                    KeyCode::Char('4') => {
                        app.reload_config();
                        app.active_tab = ActiveTab::Config;
                        app.scroll = 0;
                    }

                    // Projects tab: toggle week/month window
                    KeyCode::Char('s') if app.active_tab == ActiveTab::Projects => {
                        app.projects_days = 7;
                        app.reload_projects();
                        app.scroll = 0;
                    }
                    KeyCode::Char('m') if app.active_tab == ActiveTab::Projects => {
                        app.projects_days = 30;
                        app.reload_projects();
                        app.scroll = 0;
                    }

                    KeyCode::Down | KeyCode::Char('j') => {
                        app.scroll = app.scroll.saturating_add(3);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.scroll = app.scroll.saturating_sub(3);
                    }
                    KeyCode::PageDown => {
                        app.scroll = app.scroll.saturating_add(20);
                    }
                    KeyCode::PageUp => {
                        app.scroll = app.scroll.saturating_sub(20);
                    }
                    KeyCode::Home => app.scroll = 0,

                    KeyCode::Char('r') => {
                        if matches!(app.summary, SummaryState::Loading) {
                            continue;
                        }

                        if let Some(data) = &app.data {
                            let context = summarizer::build_context(data);
                            let provider = app.provider.clone();
                            let ollama_url = app.ollama_url.clone();
                            let api_key = app.api_key.clone();
                            let model = app.model.clone();
                            let lang = app.lang.clone();
                            let custom_prompt = app.config.custom_prompt.clone();
                            let tx = tx.clone();
                            app.summary = SummaryState::Loading;
                            app.active_tab = ActiveTab::Summary;
                            app.scroll = 0;
                            tokio::spawn(async move {
                                let res = summarizer::call_llm(
                                    &provider,
                                    &ollama_url,
                                    api_key.as_deref(),
                                    &model,
                                    &context,
                                    &lang,
                                    custom_prompt.as_deref(),
                                )
                                .await
                                .map_err(|e| e.to_string());
                                let _ = tx.send(res).await;
                            });
                        }
                    }
                    _ => {}
                }
            } // Event::Key
            _ => {}
            } // match event::read()
        }
    }
    Ok(())
}

// ─── Rendering ──────────────────────────────────────────────────────────────

fn render(f: &mut Frame, app: &mut App) {
    let size = f.area();
    app.last_size = size;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(size);

    render_header(f, app, chunks[0]);
    render_content(f, app, chunks[1]);
    render_footer(f, app, chunks[2]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let today = chrono::Local::now().date_naive();
    let date_label = if app.date == today {
        format!("◄  {} (hoje)  ►", app.date.format("%Y-%m-%d"))
    } else {
        format!("◄  {}  ►", app.date.format("%Y-%m-%d  %A"))
    };

    let tab_names = vec!["  Atividades  ", "  Resumo  ", "  Projetos  ", "  Config  "];
    let selected = match app.active_tab {
        ActiveTab::Activities => 0,
        ActiveTab::Summary => 1,
        ActiveTab::Projects => 2,
        ActiveTab::Config => 3,
    };

    let tabs = Tabs::new(tab_names)
        .select(selected)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider("│")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    format!(" {date_label} "),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ))
                .title_alignment(Alignment::Center),
        );

    f.render_widget(tabs, area);
}

fn render_content(f: &mut Frame, app: &mut App, area: Rect) {
    match app.active_tab {
        ActiveTab::Activities => render_activities(f, app, area),
        ActiveTab::Summary => render_summary(f, app, area),
        ActiveTab::Projects => render_projects(f, app, area),
        ActiveTab::Config => render_config(f, app, area),
    }
}

fn render_activities(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Atividades Brutas ");

    let Some(data) = &app.data else {
        let p = Paragraph::new(
            "\n\n  Nenhum log encontrado para esta data.\n\n  Use ← → para navegar entre os dias.",
        )
        .block(block)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    if !data.commands.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("■ SHELL  ({} comandos)", data.commands.len()),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "─".repeat(60),
            Style::default().fg(Color::DarkGray),
        )));
        for cmd in &data.commands {
            lines.push(Line::from(vec![
                Span::styled("  $ ", Style::default().fg(Color::DarkGray)),
                Span::raw(cmd.as_str()),
            ]));
        }
        lines.push(Line::from(""));
    }

    if !data.top_apps.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("■ APPS  ({} janelas)", data.top_apps.len()),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "─".repeat(60),
            Style::default().fg(Color::DarkGray),
        )));
        for (name, count) in &data.top_apps {
            if *count > 1 {
                lines.push(Line::from(format!("  {name} [{count}x]")));
            } else {
                lines.push(Line::from(format!("  {name}")));
            }
        }
        lines.push(Line::from(""));
    }

    if !data.tabs.is_empty() {
        // Deduplicar: manter ordem de primeira ocorrência, contar repetições
        let mut counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut ordered: Vec<String> = Vec::new();
        for (title, url) in &data.tabs {
            let label = if title.is_empty() || title == url {
                url.clone()
            } else {
                title.clone()
            };
            let n = counts.entry(label.clone()).or_insert(0);
            if *n == 0 {
                ordered.push(label);
            }
            *n += 1;
        }
        lines.push(Line::from(Span::styled(
            format!("■ SITES  ({} abas)", data.tabs.len()),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "─".repeat(60),
            Style::default().fg(Color::DarkGray),
        )));
        for label in ordered {
            let count = counts[&label];
            let suffix = if count > 1 {
                format!(" [{count}x]")
            } else {
                String::new()
            };
            lines.push(Line::from(vec![
                Span::styled("  · ", Style::default().fg(Color::DarkGray)),
                Span::raw(label),
                Span::styled(suffix, Style::default().fg(Color::DarkGray)),
            ]));
        }
        lines.push(Line::from(""));
    }

    if !data.repos.is_empty() {
        let total_commits: usize = data.repos.iter().map(|(_, c)| c.len()).sum();
        lines.push(Line::from(Span::styled(
            format!(
                "■ GIT  ({} repos, {} commits)",
                data.repos.len(),
                total_commits
            ),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "─".repeat(60),
            Style::default().fg(Color::DarkGray),
        )));
        for (repo, commits) in &data.repos {
            let name = std::path::Path::new(repo)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(repo);
            lines.push(Line::from(Span::styled(
                format!("  {name}"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
            for commit in commits {
                lines.push(Line::from(vec![
                    Span::styled("    ↳ ", Style::default().fg(Color::DarkGray)),
                    Span::raw(commit.as_str()),
                ]));
            }
        }
    }

    if !data.tags.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("■ NOTAS  ({} evento(s))", data.tags.len()),
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "─".repeat(60),
            Style::default().fg(Color::DarkGray),
        )));
        for (hour, label) in &data.tags {
            lines.push(Line::from(vec![
                Span::styled(format!("  {hour}  "), Style::default().fg(Color::DarkGray)),
                Span::raw(label.as_str()),
            ]));
        }
        lines.push(Line::from(""));
    }

    let p = Paragraph::new(lines)
        .block(block)
        .scroll((app.scroll, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn parse_inline(text: &str) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut remaining = text.to_string();
    while !remaining.is_empty() {
        if let Some(start) = remaining.find("**") {
            if start > 0 {
                spans.push(Span::raw(remaining[..start].to_string()));
            }
            let rest = remaining[start + 2..].to_string();
            if let Some(end) = rest.find("**") {
                spans.push(Span::styled(
                    rest[..end].to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                remaining = rest[end + 2..].to_string();
            } else {
                spans.push(Span::raw(format!("**{rest}")));
                remaining = String::new();
            }
        } else {
            spans.push(Span::raw(remaining.clone()));
            remaining = String::new();
        }
    }
    spans
}

fn markdown_to_lines(text: &str) -> Vec<Line<'static>> {
    text.lines()
        .map(|line| {
            if line.starts_with("### ") {
                Line::from(Span::styled(
                    line[4..].to_string(),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ))
            } else if line.starts_with("## ") {
                Line::from(Span::styled(
                    line[3..].to_string(),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ))
            } else if line.starts_with("# ") {
                Line::from(Span::styled(
                    line[2..].to_string(),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ))
            } else if let Some(rest) = line.strip_prefix("    - ").or_else(|| line.strip_prefix("    * ")) {
                let mut spans = vec![Span::styled("      ◦ ", Style::default().fg(Color::DarkGray))];
                spans.extend(parse_inline(rest));
                Line::from(spans)
            } else if let Some(rest) = line.strip_prefix("  - ").or_else(|| line.strip_prefix("  * ")) {
                let mut spans = vec![Span::styled("    • ", Style::default().fg(Color::DarkGray))];
                spans.extend(parse_inline(rest));
                Line::from(spans)
            } else if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
                let mut spans = vec![Span::styled("  • ", Style::default().fg(Color::DarkGray))];
                spans.extend(parse_inline(rest));
                Line::from(spans)
            } else if line.trim().is_empty() {
                Line::from("")
            } else {
                Line::from(parse_inline(line))
            }
        })
        .collect()
}

fn render_summary(f: &mut Frame, app: &App, area: Rect) {
    match &app.summary {
        SummaryState::Empty => {
            let block = Block::default().borders(Borders::ALL).title(" Resumo LLM ");
            let hint = if app.data.is_some() {
                format!(
                    "\n\n  Pressione r para gerar o resumo.\n\n  Provider: {}/{}",
                    app.provider, app.model
                )
            } else {
                "\n\n  Nenhum dado para esta data.".to_string()
            };
            let p = Paragraph::new(hint)
                .block(block)
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(p, area);
        }
        SummaryState::Loading => {
            let block = Block::default().borders(Borders::ALL).title(" Resumo LLM ");
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let frame = (app.tick as usize / 2) % frames.len();
            let p = Paragraph::new(format!(
                "\n\n  {} Gerando resumo com {}/{}...",
                frames[frame], app.provider, app.model
            ))
            .block(block)
            .style(Style::default().fg(Color::Cyan));
            f.render_widget(p, area);
        }
        SummaryState::Cached(text) => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Resumo LLM ")
                .title_bottom(Span::styled(
                    " salvo · r para regenerar ",
                    Style::default().fg(Color::DarkGray),
                ));
            let p = Paragraph::new(markdown_to_lines(text))
                .block(block)
                .scroll((app.scroll, 0))
                .wrap(Wrap { trim: false });
            f.render_widget(p, area);
        }
        SummaryState::Done(text) => {
            let block = Block::default().borders(Borders::ALL).title(" Resumo LLM ");
            let p = Paragraph::new(markdown_to_lines(text))
                .block(block)
                .scroll((app.scroll, 0))
                .wrap(Wrap { trim: false });
            f.render_widget(p, area);
        }
        SummaryState::Error(e) => {
            let block = Block::default().borders(Borders::ALL).title(" Resumo LLM ");
            let p = Paragraph::new(format!("\n  Erro: {e}"))
                .block(block)
                .style(Style::default().fg(Color::Red));
            f.render_widget(p, area);
        }
    }
}

fn render_projects(f: &mut Frame, app: &App, area: Rect) {
    let window_label = if app.projects_days == 7 {
        "7 dias"
    } else {
        "30 dias"
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Projetos — {window_label} "));

    let stats = match &app.projects {
        None => {
            let p = Paragraph::new("\n\n  Carregando...")
                .block(block)
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(p, area);
            return;
        }
        Some(s) if s.is_empty() => {
            let p = Paragraph::new(
                "\n\n  Nenhum commit encontrado no período.\n\n  O daemon precisa estar rodando para coletar dados de git.",
            )
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
            f.render_widget(p, area);
            return;
        }
        Some(s) => s,
    };

    let max_name = stats
        .iter()
        .map(|s| s.name.len())
        .max()
        .unwrap_or(10)
        .min(24);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for stat in stats {
        let bar_filled = ((stat.pct / 100.0) * 24.0).round() as usize;
        let bar_empty = 24usize.saturating_sub(bar_filled);

        let name = if stat.name.len() > max_name {
            format!("{}…", &stat.name[..max_name.saturating_sub(1)])
        } else {
            format!("{:<width$}", stat.name, width = max_name)
        };

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                name,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("█".repeat(bar_filled), Style::default().fg(Color::Green)),
            Span::styled("░".repeat(bar_empty), Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("  {:>5.1}%", stat.pct),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}c {}d", stat.commits, stat.days_active),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    let p = Paragraph::new(lines).block(block).scroll((app.scroll, 0));
    f.render_widget(p, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let at_today = app.date >= chrono::Local::now().date_naive();
    let nav = if at_today { "← dia" } else { "←/→ dia" };
    let r_hint = match &app.summary {
        SummaryState::Cached(_) => "r regenerar",
        _ => "r resumo",
    };
    let hints = match app.active_tab {
        ActiveTab::Projects => {
            format!(" {nav}  Tab aba  ↑↓/jk scroll  s semana  m mês  q sair ")
        }
        ActiveTab::Config => {
            if matches!(app.config_edit, ConfigEditMode::Editing(_, _)) {
                " Enter salvar  Esc cancelar ".to_string()
            } else {
                " ↑↓/jk navegar  Enter/e editar  ← → ciclar  d deletar padrão  R reload  q sair "
                    .to_string()
            }
        }
        _ => format!(" {nav}  Tab aba  ↑↓/jk scroll  {r_hint}  q sair "),
    };
    let p = Paragraph::new(hints).style(Style::default().fg(Color::DarkGray));
    f.render_widget(p, area);
}

fn render_config(f: &mut Frame, app: &mut App, area: Rect) {
    let cfg = &app.config;
    let cur = app.config_cursor;
    // edit_buf: Some((buffer, cursor_char_index)) quando em modo edição
    let edit_buf: Option<(&str, usize)> = match &app.config_edit {
        ConfigEditMode::Editing(b, cur) => Some((b.as_str(), *cur)),
        ConfigEditMode::Browse => None,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Configuração ");

    let mut lines: Vec<Line> = Vec::new();
    // Parallel vec: lines[i] → config field index (usize::MAX = non-field)
    let mut row_map: Vec<usize> = Vec::new();

    macro_rules! push {
        ($line:expr, $field:expr) => {
            lines.push($line);
            row_map.push($field);
        };
    }

    push!(Line::from(""), usize::MAX);

    // ── helpers ──────────────────────────────────────────────────────────────

    let section_line = |label: &str| {
        Line::from(Span::styled(
            format!("■ {label}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))
    };
    let sep_line = || {
        Line::from(Span::styled(
            "─".repeat(52),
            Style::default().fg(Color::DarkGray),
        ))
    };

    // Render one editable field row.
    // idx: field index; label: display name; display_val: value when not editing;
    // cycle_hint: show "← →" hint when selected (for provider/lang)
    let field_row = |idx: usize, label: &str, display_val: &str, cycle_hint: bool| -> Line {
        let selected = cur == idx;
        let editing = selected && edit_buf.is_some();

        let prefix = if selected {
            Span::styled(
                " > ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw("   ")
        };
        let lbl = Span::styled(
            format!("{label:<18}"),
            if selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            },
        );
        let val = if editing {
            let (buf, cursor_pos) = edit_buf.unwrap_or(("", 0));
            Span::styled(
                edit_render_cursor(buf, cursor_pos),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else if cycle_hint && selected {
            Span::styled(
                format!("{display_val}  ← →"),
                Style::default().fg(Color::Green),
            )
        } else if selected {
            Span::styled(display_val.to_string(), Style::default().fg(Color::Green))
        } else {
            Span::styled(
                display_val.to_string(),
                Style::default().fg(Color::DarkGray),
            )
        };
        Line::from(vec![prefix, lbl, val])
    };

    // ── LLM ──────────────────────────────────────────────────────────────────
    push!(section_line("LLM"), usize::MAX);
    push!(sep_line(), usize::MAX);
    push!(field_row(CF_PROVIDER, "provider", &cfg.provider, true), CF_PROVIDER);
    push!(field_row(CF_MODEL, "modelo", &cfg.model, false), CF_MODEL);

    let url_label = if cfg.provider == "ollama" {
        "url"
    } else {
        "api_key"
    };
    let url_val = if cfg.provider == "ollama" {
        cfg.ollama_url.clone()
    } else {
        cfg.api_key
            .as_deref()
            .map(|k| {
                let n = k.len();
                if n <= 8 {
                    "*".repeat(n)
                } else {
                    format!("{}…{}", &k[..4], &k[n - 4..])
                }
            })
            .unwrap_or_else(|| "não configurado".into())
    };
    push!(field_row(CF_URL_OR_KEY, url_label, &url_val, false), CF_URL_OR_KEY);
    push!(field_row(CF_LANG, "idioma", &cfg.lang, true), CF_LANG);

    // CF_PROMPT: rendering multi-linha customizado
    {
        let prompt_selected = cur == CF_PROMPT;
        let prompt_editing = prompt_selected && edit_buf.is_some();
        let prefix = if prompt_selected {
            Span::styled(" > ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        } else {
            Span::raw("   ")
        };
        let lbl = Span::styled(
            format!("{:<18}", "prompt"),
            if prompt_selected {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            },
        );

        if prompt_editing {
            // Modo edição: divide por \n literal e mostra cada segmento como linha
            let (buf, cursor_pos) = edit_buf.unwrap_or(("", 0));
            // Insere █ na posição do cursor, depois divide por \n
            let rendered = edit_render_cursor(buf, cursor_pos);
            let parts: Vec<&str> = rendered.split(r"\n").collect();
            let edit_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);

            for (i, part) in parts.iter().enumerate() {
                let text = part.to_string();
                if i == 0 {
                    push!(Line::from(vec![
                        prefix.clone(),
                        lbl.clone(),
                        Span::styled(text, edit_style),
                    ]), CF_PROMPT);
                } else {
                    push!(Line::from(Span::styled(
                        format!("   {:<18}{text}", ""),
                        edit_style,
                    )), CF_PROMPT);
                }
            }
            push!(Line::from(Span::styled(
                "   [Enter = confirma  •  \\n = quebra de linha  •  Esc = cancela]",
                Style::default().fg(Color::DarkGray),
            )), usize::MAX);
        } else {
            let template = cfg.custom_prompt.as_deref()
                .unwrap_or(summarizer::DEFAULT_PROMPT_TEMPLATE);
            let parts: Vec<&str> = template.split(r"\n").collect();

            // Primeira linha: prefixo + label + conteúdo
            let first = parts.first().copied().unwrap_or("");
            let val_style = if prompt_selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            push!(Line::from(vec![
                prefix,
                lbl,
                Span::styled(first.to_string(), val_style),
            ]), CF_PROMPT);

            // Linhas seguintes indentadas (sempre visíveis, não só quando selecionado)
            for part in parts.iter().skip(1) {
                push!(Line::from(Span::styled(
                    format!("   {:<18}{}", "", part),
                    val_style,
                )), CF_PROMPT);
            }

            if prompt_selected {
                push!(Line::from(Span::styled(
                    "   [Enter para editar  •  \\n = quebra de linha]",
                    Style::default().fg(Color::DarkGray),
                )), usize::MAX);
            }
        }
    }
    push!(Line::from(""), usize::MAX);

    // ── MÁQUINA ───────────────────────────────────────────────────────────────
    push!(section_line("MÁQUINA"), usize::MAX);
    push!(sep_line(), usize::MAX);
    push!(field_row(
        CF_MACHINE,
        "nome",
        &cfg.machine_name
            .clone()
            .unwrap_or_else(|| cfg.get_machine_name()),
        false,
    ), CF_MACHINE);
    push!(Line::from(""), usize::MAX);

    // ── INTEGRAÇÕES ───────────────────────────────────────────────────────────
    push!(section_line("INTEGRAÇÕES"), usize::MAX);
    push!(sep_line(), usize::MAX);
    let notion_token_val = cfg
        .notion_token
        .as_deref()
        .map(|k| {
            let n = k.len();
            if n <= 8 {
                "*".repeat(n)
            } else {
                format!("{}…{}", &k[..4], &k[n - 4..])
            }
        })
        .unwrap_or_else(|| "não configurado".into());
    push!(field_row(
        CF_NOTION_TOKEN,
        "notion_token",
        &notion_token_val,
        false,
    ), CF_NOTION_TOKEN);
    let notion_page_val = cfg.notion_page_id.as_deref().unwrap_or("não configurado");
    push!(field_row(
        CF_NOTION_PAGE,
        "notion_page",
        notion_page_val,
        false,
    ), CF_NOTION_PAGE);
    let slack_val = cfg
        .slack_webhook
        .as_deref()
        .map(|u| {
            if u.len() > 30 {
                format!("{}…", &u[..30])
            } else {
                u.to_string()
            }
        })
        .unwrap_or_else(|| "não configurado".into());
    push!(field_row(CF_SLACK, "slack_webhook", &slack_val, false), CF_SLACK);
    push!(Line::from(""), usize::MAX);

    // ── PRIVACIDADE ───────────────────────────────────────────────────────────
    push!(section_line("PRIVACIDADE"), usize::MAX);
    push!(sep_line(), usize::MAX);

    // "+ adicionar padrão" button
    let add_selected = cur == CF_ADD_BLOCK;
    let add_editing = add_selected && edit_buf.is_some();
    let add_prefix = if add_selected {
        Span::styled(
            " > ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("   ")
    };
    let add_val = if add_editing {
        let (buf, cur) = edit_buf.unwrap_or(("", 0));
        Span::styled(
            edit_render_cursor(buf, cur),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            "+ adicionar padrão…",
            if add_selected {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        )
    };
    push!(Line::from(vec![add_prefix, add_val]), CF_ADD_BLOCK);

    for (i, pattern) in cfg.blocked_patterns.iter().enumerate() {
        let idx = CF_ADD_BLOCK + 1 + i;
        let sel = cur == idx;
        let ed = sel && edit_buf.is_some();
        let prefix = if sel {
            Span::styled(
                " > ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw("   ")
        };
        let val_span = if ed {
            let (buf, cur) = edit_buf.unwrap_or(("", 0));
            Span::styled(
                edit_render_cursor(buf, cur),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                format!("- {pattern}"),
                if sel {
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Red)
                },
            )
        };
        let del_hint = if sel && !ed {
            Span::styled("  d deletar", Style::default().fg(Color::DarkGray))
        } else {
            Span::raw("")
        };
        push!(Line::from(vec![prefix, val_span, del_hint]), idx);
    }
    push!(Line::from(""), usize::MAX);

    // ── PATHS (read-only) ─────────────────────────────────────────────────────
    push!(section_line("PATHS"), usize::MAX);
    push!(sep_line(), usize::MAX);
    push!(Line::from(vec![
        Span::raw("   "),
        Span::styled("logs              ", Style::default().fg(Color::White)),
        Span::styled(
            crate::config::log_dir().display().to_string(),
            Style::default().fg(Color::DarkGray),
        ),
    ]), usize::MAX);
    push!(Line::from(vec![
        Span::raw("   "),
        Span::styled("config            ", Style::default().fg(Color::White)),
        Span::styled(
            crate::config::config_path().display().to_string(),
            Style::default().fg(Color::DarkGray),
        ),
    ]), usize::MAX);

    app.config_row_map = row_map;

    let p = Paragraph::new(lines)
        .block(block)
        .scroll((app.scroll, 0));
    f.render_widget(p, area);
}

// ── Mouse handler ────────────────────────────────────────────────────────────

fn handle_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) {
    let x = mouse.column;
    let y = mouse.row;
    let width = app.last_size.width;
    let height = app.last_size.height;

    match mouse.kind {
        MouseEventKind::ScrollUp => {
            app.scroll = app.scroll.saturating_sub(3);
        }
        MouseEventKind::ScrollDown => {
            app.scroll = app.scroll.saturating_add(3);
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if y == 0 {
                // Top border: click left half → prev day, right half → next day
                if x < width / 2 {
                    app.prev_day();
                } else {
                    app.next_day();
                }
            } else if y <= 2 {
                // Header block (rows 0-2): detect tab click by computing x ranges dynamically
                let tab_names = ["  Atividades  ", "  Resumo  ", "  Projetos  ", "  Config  "];
                let mut pos = 1u16; // skip left block border
                let mut clicked = None;
                let mut last_started = None; // fallback: last tab whose start ≤ x
                for (i, name) in tab_names.iter().enumerate() {
                    let len = name.chars().count() as u16;
                    if x >= pos { last_started = Some(i); }
                    if x >= pos && x < pos + len {
                        clicked = Some(i);
                        break;
                    }
                    pos += len + 1; // +1 for divider "│"
                }
                // Clicking in empty space to the right of all tabs → nearest tab to the left
                if clicked.is_none() {
                    clicked = last_started;
                }
                if let Some(idx) = clicked {
                    let new_tab = match idx {
                        0 => ActiveTab::Activities,
                        1 => ActiveTab::Summary,
                        2 => ActiveTab::Projects,
                        _ => ActiveTab::Config,
                    };
                    if new_tab == ActiveTab::Projects {
                        app.ensure_projects_loaded();
                    }
                    if new_tab == ActiveTab::Config && app.active_tab != ActiveTab::Config {
                        app.reload_config();
                    }
                    if new_tab != app.active_tab {
                        app.active_tab = new_tab;
                        app.scroll = 0;
                    }
                }
            } else if y >= 4 && y < height.saturating_sub(1) {
                // Content area: y=3 is block top border, y=4 is first content row
                if app.active_tab == ActiveTab::Config
                    && matches!(app.config_edit, ConfigEditMode::Browse)
                {
                    let row = (y as usize - 4) + app.scroll as usize;
                    if let Some(&field) = app.config_row_map.get(row) {
                        if field != usize::MAX {
                            app.config_cursor = field;
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

// ── Config helpers ────────────────────────────────────────────────────────────

fn cfg_cycle(current: &mut String, options: &[&str], dir: i32) {
    let pos = options
        .iter()
        .position(|&o| o == current.as_str())
        .unwrap_or(0);
    let n = options.len();
    let new_pos = ((pos as i32 + dir).rem_euclid(n as i32)) as usize;
    *current = options[new_pos].to_string();
}

fn cfg_initial_value(cfg: &Config, cursor: usize) -> Option<String> {
    match cursor {
        CF_MODEL => Some(cfg.model.clone()),
        CF_URL_OR_KEY => Some(if cfg.provider == "ollama" {
            cfg.ollama_url.clone()
        } else {
            cfg.api_key.clone().unwrap_or_default()
        }),
        CF_PROMPT => Some(
            cfg.custom_prompt
                .clone()
                .unwrap_or_else(|| summarizer::DEFAULT_PROMPT_TEMPLATE.to_string()),
        ),
        CF_MACHINE => Some(cfg.machine_name.clone().unwrap_or_default()),
        CF_NOTION_TOKEN => Some(cfg.notion_token.clone().unwrap_or_default()),
        CF_NOTION_PAGE => Some(cfg.notion_page_id.clone().unwrap_or_default()),
        CF_SLACK => Some(cfg.slack_webhook.clone().unwrap_or_default()),
        CF_ADD_BLOCK => Some(String::new()),
        c if c > CF_ADD_BLOCK => cfg.blocked_patterns.get(c - CF_ADD_BLOCK - 1).cloned(),
        _ => None, // provider and lang cycle, no text edit
    }
}

fn cfg_apply(cfg: &mut Config, cursor: usize, value: String) {
    let v = value.trim().to_string();
    match cursor {
        CF_MODEL => cfg.model = v,
        CF_URL_OR_KEY => {
            if cfg.provider == "ollama" {
                cfg.ollama_url = v;
            } else {
                cfg.api_key = if v.is_empty() { None } else { Some(v) };
            }
        }
        CF_PROMPT => cfg.custom_prompt = if v.is_empty() { None } else { Some(v) },
        CF_MACHINE => cfg.machine_name = if v.is_empty() { None } else { Some(v) },
        CF_NOTION_TOKEN => cfg.notion_token = if v.is_empty() { None } else { Some(v) },
        CF_NOTION_PAGE => cfg.notion_page_id = if v.is_empty() { None } else { Some(v) },
        CF_SLACK => cfg.slack_webhook = if v.is_empty() { None } else { Some(v) },
        CF_ADD_BLOCK => {
            if !v.is_empty() {
                cfg.blocked_patterns.push(v);
            }
        }
        c if c > CF_ADD_BLOCK => {
            let idx = c - CF_ADD_BLOCK - 1;
            if idx < cfg.blocked_patterns.len() {
                if v.is_empty() {
                    cfg.blocked_patterns.remove(idx);
                } else {
                    cfg.blocked_patterns[idx] = v;
                }
            }
        }
        _ => {}
    }
}

// ── Edit cursor helpers ───────────────────────────────────────────────────────

/// Insere char na posição do cursor e avança o cursor.
fn edit_cursor_insert(buf: &mut String, cursor: &mut usize, c: char) {
    let byte = buf.char_indices().nth(*cursor).map(|(i, _)| i).unwrap_or(buf.len());
    buf.insert(byte, c);
    *cursor += 1;
}

/// Apaga o char antes do cursor (Backspace).
fn edit_cursor_backspace(buf: &mut String, cursor: &mut usize) {
    if *cursor > 0 {
        let byte = buf.char_indices().nth(*cursor - 1).map(|(i, _)| i).unwrap_or(0);
        buf.remove(byte);
        *cursor -= 1;
    }
}

/// Apaga o char na posição do cursor (Delete).
fn edit_cursor_delete(buf: &mut String, cursor: usize) {
    if let Some((byte, _)) = buf.char_indices().nth(cursor) {
        buf.remove(byte);
    }
}

/// Produz a string de exibição com █ na posição do cursor.
fn edit_render_cursor(buf: &str, cursor: usize) -> String {
    let chars: Vec<char> = buf.chars().collect();
    let cur = cursor.min(chars.len());
    let before: String = chars[..cur].iter().collect();
    let after: String = chars[cur..].iter().collect();
    format!("{before}█{after}")
}

// ── Navegação por linha no buffer de edição ───────────────────────────────────
// O separador de linha é o literal dois-chars "\n" (backslash + n).

/// Retorna os índices de char onde cada linha começa.
fn edit_line_starts(buf: &str) -> Vec<usize> {
    let chars: Vec<char> = buf.chars().collect();
    let mut starts = vec![0usize];
    let mut i = 0;
    while i + 1 < chars.len() {
        if chars[i] == '\\' && chars[i + 1] == 'n' {
            starts.push(i + 2);
            i += 2;
        } else {
            i += 1;
        }
    }
    starts
}

/// Retorna (índice_da_linha, coluna) para a posição de cursor dada.
fn edit_line_col(buf: &str, cursor: usize) -> (usize, usize) {
    let starts = edit_line_starts(buf);
    let total = buf.chars().count();
    let cursor = cursor.min(total);
    // Percorre de trás para frente: a linha i começa em starts[i]
    for i in (0..starts.len()).rev() {
        if cursor >= starts[i] {
            let line_end = if i + 1 < starts.len() {
                starts[i + 1].saturating_sub(2)
            } else {
                total
            };
            let col = cursor.min(line_end) - starts[i];
            return (i, col);
        }
    }
    (0, 0)
}

/// Converte (linha, coluna) de volta para posição de cursor.
fn edit_pos_from_line_col(buf: &str, line: usize, col: usize) -> usize {
    let starts = edit_line_starts(buf);
    let total = buf.chars().count();
    let line_start = starts.get(line).copied().unwrap_or(total);
    let line_end = if line + 1 < starts.len() {
        starts[line + 1].saturating_sub(2)
    } else {
        total
    };
    (line_start + col).min(line_end)
}

/// Move o cursor uma linha acima, mantendo a coluna.
fn edit_cursor_up(buf: &str, cursor: usize) -> usize {
    let (line, col) = edit_line_col(buf, cursor);
    if line == 0 { return 0; }
    edit_pos_from_line_col(buf, line - 1, col)
}

/// Move o cursor uma linha abaixo, mantendo a coluna.
fn edit_cursor_down(buf: &str, cursor: usize) -> usize {
    let starts = edit_line_starts(buf);
    let (line, col) = edit_line_col(buf, cursor);
    if line + 1 >= starts.len() { return buf.chars().count(); }
    edit_pos_from_line_col(buf, line + 1, col)
}

/// Início da linha atual.
fn edit_line_start(buf: &str, cursor: usize) -> usize {
    let (line, _) = edit_line_col(buf, cursor);
    edit_line_starts(buf).get(line).copied().unwrap_or(0)
}

/// Final da linha atual (antes do separador \n ou fim do buffer).
fn edit_line_end(buf: &str, cursor: usize) -> usize {
    let starts = edit_line_starts(buf);
    let (line, _) = edit_line_col(buf, cursor);
    let total = buf.chars().count();
    if line + 1 < starts.len() {
        starts[line + 1].saturating_sub(2)
    } else {
        total
    }
}
