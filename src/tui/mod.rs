use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::Rect,
};
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::projects::ProjectStat;
use crate::summarizer::{self, ActivityData};

mod config;
mod edit;
mod render;
mod render_config;

// ── Config field indices ──────────────────────────────────────────────────────
pub(super) const CF_PROVIDER: usize = 0;
pub(super) const CF_MODEL: usize = 1;
pub(super) const CF_URL_OR_KEY: usize = 2;
pub(super) const CF_LANG: usize = 3;
pub(super) const CF_PROMPT: usize = 4;
pub(super) const CF_MACHINE: usize = 5;
pub(super) const CF_NOTION_TOKEN: usize = 6;
pub(super) const CF_NOTION_PAGE: usize = 7;
pub(super) const CF_SLACK: usize = 8;
pub(super) const CF_ADD_BLOCK: usize = 9;
// CF_ADD_BLOCK + 1 + i  →  blocked_patterns[i]

pub(super) const PROVIDERS: &[&str] = &[
    "ollama",
    "openai",
    "anthropic",
    "groq",
    "gemini",
    "openrouter",
];
pub(super) const LANGS: &[&str] = &["pt-br", "en", "es", "fr", "de", "ja", "zh"];

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
pub(super) enum ActiveTab {
    Activities,
    Summary,
    Projects,
    Config,
}

#[derive(PartialEq)]
pub(super) enum ConfigEditMode {
    Browse,
    Editing(String, usize), // (buffer, cursor char index)
}

pub(super) enum SummaryState {
    Empty,
    Loading,
    Cached(String),
    Done(String),
    Error(String),
}

// ── App ───────────────────────────────────────────────────────────────────────

pub(super) struct App {
    pub date: chrono::NaiveDate,
    pub data: Option<ActivityData>,
    pub summary: SummaryState,
    pub active_tab: ActiveTab,
    pub scroll: u16,
    pub tick: u8,
    pub model: String,
    pub provider: String,
    pub ollama_url: String,
    pub api_key: Option<String>,
    pub lang: String,
    pub projects: Option<Vec<ProjectStat>>,
    pub projects_days: u32,
    pub config: Config,
    pub config_cursor: usize,
    pub config_edit: ConfigEditMode,
    pub config_row_map: Vec<usize>,
    pub last_size: Rect,
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

    pub fn reload_config(&mut self) {
        if let Ok(cfg) = Config::load() {
            self.config = cfg;
        }
        self.scroll = 0;
        self.config_cursor = 0;
        self.config_edit = ConfigEditMode::Browse;
    }

    pub fn config_item_count(&self) -> usize {
        CF_ADD_BLOCK + 1 + self.config.blocked_patterns.len()
    }

    pub fn load_data(&mut self) {
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

    pub fn prev_day(&mut self) {
        self.date -= chrono::Duration::days(1);
        self.load_data();
    }

    pub fn next_day(&mut self) {
        if self.date < chrono::Local::now().date_naive() {
            self.date += chrono::Duration::days(1);
            self.load_data();
        }
    }

    pub fn ensure_projects_loaded(&mut self) {
        if self.projects.is_none() {
            self.projects =
                Some(crate::projects::load_stats(self.projects_days).unwrap_or_default());
        }
    }

    pub fn reload_projects(&mut self) {
        self.projects = Some(crate::projects::load_stats(self.projects_days).unwrap_or_default());
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

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

// ── Event loop ────────────────────────────────────────────────────────────────

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    tx: &mpsc::Sender<Result<String, String>>,
    rx: &mut mpsc::Receiver<Result<String, String>>,
) -> Result<()> {
    loop {
        terminal.draw(|f| render::render(f, app))?;
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
                render::handle_mouse(app, mouse);
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
                                    config::cfg_apply(&mut app.config, app.config_cursor, buf);
                                    let _ = app.config.save();
                                    if app.config_cursor == CF_ADD_BLOCK {
                                        app.config_cursor =
                                            CF_ADD_BLOCK + app.config.blocked_patterns.len();
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                if let ConfigEditMode::Editing(b, cur) = &mut app.config_edit {
                                    edit::edit_cursor_backspace(b, cur);
                                }
                            }
                            KeyCode::Delete => {
                                if let ConfigEditMode::Editing(b, cur) = &mut app.config_edit {
                                    edit::edit_cursor_delete(b, *cur);
                                }
                            }
                            KeyCode::Up => {
                                if let ConfigEditMode::Editing(b, cur) = &mut app.config_edit {
                                    *cur = edit::edit_cursor_up(b, *cur);
                                }
                            }
                            KeyCode::Down => {
                                if let ConfigEditMode::Editing(b, cur) = &mut app.config_edit {
                                    *cur = edit::edit_cursor_down(b, *cur);
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
                                    if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                                        *cur = 0;
                                    } else {
                                        *cur = edit::edit_line_start(b, *cur);
                                    }
                                }
                            }
                            KeyCode::End => {
                                if let ConfigEditMode::Editing(b, cur) = &mut app.config_edit {
                                    if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                                        *cur = b.chars().count();
                                    } else {
                                        *cur = edit::edit_line_end(b, *cur);
                                    }
                                }
                            }
                            KeyCode::Char(c) => {
                                if let ConfigEditMode::Editing(b, cur) = &mut app.config_edit {
                                    edit::edit_cursor_insert(b, cur, c);
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
                        KeyCode::Right | KeyCode::Char('l') => {
                            if app.config_cursor == CF_PROVIDER {
                                config::cfg_cycle(&mut app.config.provider, PROVIDERS, 1);
                                let _ = app.config.save();
                            } else if app.config_cursor == CF_LANG {
                                config::cfg_cycle(&mut app.config.lang, LANGS, 1);
                                let _ = app.config.save();
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            if app.config_cursor == CF_PROVIDER {
                                config::cfg_cycle(&mut app.config.provider, PROVIDERS, -1);
                                let _ = app.config.save();
                            } else if app.config_cursor == CF_LANG {
                                config::cfg_cycle(&mut app.config.lang, LANGS, -1);
                                let _ = app.config.save();
                            }
                        }
                        KeyCode::Enter | KeyCode::Char('e') => {
                            if let Some(val) = config::cfg_initial_value(&app.config, app.config_cursor) {
                                let end = val.chars().count();
                                app.config_edit = ConfigEditMode::Editing(val, end);
                            }
                        }
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
