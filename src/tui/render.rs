use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
};

use super::{ActiveTab, App, SummaryState};

// ─── Top-level render ─────────────────────────────────────────────────────────

pub(super) fn render(f: &mut Frame, app: &mut App) {
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
        ActiveTab::Config => super::render_config::render_config(f, app, area),
    }
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
            if matches!(app.config_edit, super::ConfigEditMode::Editing(_, _)) {
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

// ─── Atividades ───────────────────────────────────────────────────────────────

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

// ─── Projetos ─────────────────────────────────────────────────────────────────

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

// ─── Resumo ───────────────────────────────────────────────────────────────────

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

// ─── Markdown → Ratatui ───────────────────────────────────────────────────────

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

// ─── Mouse ────────────────────────────────────────────────────────────────────

pub(super) fn handle_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) {
    let x = mouse.column;
    let y = mouse.row;
    let width = app.last_size.width;
    let height = app.last_size.height;

    match mouse.kind {
        crossterm::event::MouseEventKind::ScrollUp => {
            app.scroll = app.scroll.saturating_sub(3);
        }
        crossterm::event::MouseEventKind::ScrollDown => {
            app.scroll = app.scroll.saturating_add(3);
        }
        crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
            if y == 0 {
                if x < width / 2 {
                    app.prev_day();
                } else {
                    app.next_day();
                }
            } else if y <= 2 {
                let tab_names = ["  Atividades  ", "  Resumo  ", "  Projetos  ", "  Config  "];
                let mut pos = 1u16;
                let mut clicked = None;
                let mut last_started = None;
                for (i, name) in tab_names.iter().enumerate() {
                    let len = name.chars().count() as u16;
                    if x >= pos { last_started = Some(i); }
                    if x >= pos && x < pos + len {
                        clicked = Some(i);
                        break;
                    }
                    pos += len + 1;
                }
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
                if app.active_tab == ActiveTab::Config
                    && matches!(app.config_edit, super::ConfigEditMode::Browse)
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
