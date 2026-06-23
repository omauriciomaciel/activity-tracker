use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::edit::edit_render_cursor;
use super::i18n::Ui;
use super::{
    App, CF_ADD_BLOCK, CF_LANG, CF_MACHINE, CF_MODEL, CF_NOTION_PAGE, CF_NOTION_TOKEN, CF_PROMPT,
    CF_PROVIDER, CF_SLACK, CF_URL_OR_KEY, ConfigEditMode,
};
use crate::summarizer;

pub(super) fn render_config(f: &mut Frame, app: &mut App, area: Rect) {
    let ui = Ui::new(&app.config.lang);
    let cfg = &app.config;
    let cur = app.config_cursor;
    let cf_add_git = app.cf_add_git_path();
    let edit_buf: Option<(&str, usize)> = match &app.config_edit {
        ConfigEditMode::Editing(b, cur) => Some((b.as_str(), *cur)),
        ConfigEditMode::Browse => None,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(ui.t("block.config"));

    let mut lines: Vec<Line> = Vec::new();
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
    push!(
        field_row(CF_PROVIDER, "provider", &cfg.provider, true),
        CF_PROVIDER
    );
    push!(
        field_row(CF_MODEL, ui.t("field.model"), &cfg.model, false),
        CF_MODEL
    );

    let url_label = if cfg.provider == "ollama" {
        "url"
    } else {
        "api_key"
    };
    let not_cfg = ui.t("not_configured");
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
            .unwrap_or_else(|| not_cfg.to_string())
    };
    push!(
        field_row(CF_URL_OR_KEY, url_label, &url_val, false),
        CF_URL_OR_KEY
    );
    push!(
        field_row(CF_LANG, ui.t("field.lang"), &cfg.lang, true),
        CF_LANG
    );

    // CF_PROMPT: multi-line rendering
    {
        let prompt_selected = cur == CF_PROMPT;
        let prompt_editing = prompt_selected && edit_buf.is_some();
        let prefix = if prompt_selected {
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
            format!("{:<18}", "prompt"),
            if prompt_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            },
        );

        if prompt_editing {
            let (buf, cursor_pos) = edit_buf.unwrap_or(("", 0));
            let rendered = edit_render_cursor(buf, cursor_pos);
            let parts: Vec<&str> = rendered.split(r"\n").collect();
            let edit_style = Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD);

            for (i, part) in parts.iter().enumerate() {
                let text = part.to_string();
                if i == 0 {
                    push!(
                        Line::from(vec![
                            prefix.clone(),
                            lbl.clone(),
                            Span::styled(text, edit_style),
                        ]),
                        CF_PROMPT
                    );
                } else {
                    push!(
                        Line::from(Span::styled(format!("   {:<18}{text}", ""), edit_style,)),
                        CF_PROMPT
                    );
                }
            }
            push!(
                Line::from(Span::styled(
                    ui.t("prompt.editing_hint"),
                    Style::default().fg(Color::DarkGray),
                )),
                usize::MAX
            );
        } else {
            let template = cfg
                .custom_prompt
                .as_deref()
                .unwrap_or(summarizer::DEFAULT_PROMPT_TEMPLATE);
            let parts: Vec<&str> = template.split(r"\n").collect();

            let first = parts.first().copied().unwrap_or("");
            let val_style = if prompt_selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            push!(
                Line::from(vec![
                    prefix,
                    lbl,
                    Span::styled(first.to_string(), val_style),
                ]),
                CF_PROMPT
            );

            for part in parts.iter().skip(1) {
                push!(
                    Line::from(Span::styled(format!("   {:<18}{}", "", part), val_style,)),
                    CF_PROMPT
                );
            }

            if prompt_selected {
                push!(
                    Line::from(Span::styled(
                        ui.t("prompt.browse_hint"),
                        Style::default().fg(Color::DarkGray),
                    )),
                    usize::MAX
                );
            }
        }
    }
    push!(Line::from(""), usize::MAX);

    // ── MÁQUINA ───────────────────────────────────────────────────────────────
    push!(section_line(ui.t("section.machine")), usize::MAX);
    push!(sep_line(), usize::MAX);
    push!(
        field_row(
            CF_MACHINE,
            ui.t("field.name"),
            &cfg.machine_name
                .clone()
                .unwrap_or_else(|| cfg.get_machine_name()),
            false,
        ),
        CF_MACHINE
    );
    push!(Line::from(""), usize::MAX);

    // ── INTEGRAÇÕES ───────────────────────────────────────────────────────────
    push!(section_line(ui.t("section.integrations")), usize::MAX);
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
        .unwrap_or_else(|| not_cfg.to_string());
    push!(
        field_row(CF_NOTION_TOKEN, "notion_token", &notion_token_val, false),
        CF_NOTION_TOKEN
    );
    let notion_page_val = cfg.notion_page_id.as_deref().unwrap_or(not_cfg);
    push!(
        field_row(CF_NOTION_PAGE, "notion_page", notion_page_val, false),
        CF_NOTION_PAGE
    );
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
        .unwrap_or_else(|| not_cfg.to_string());
    push!(
        field_row(CF_SLACK, "slack_webhook", &slack_val, false),
        CF_SLACK
    );
    push!(Line::from(""), usize::MAX);

    // ── PRIVACIDADE ───────────────────────────────────────────────────────────
    push!(section_line(ui.t("section.privacy")), usize::MAX);
    push!(sep_line(), usize::MAX);

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
            ui.t("add_pattern"),
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
            Span::styled(ui.t("delete_hint"), Style::default().fg(Color::DarkGray))
        } else {
            Span::raw("")
        };
        push!(Line::from(vec![prefix, val_span, del_hint]), idx);
    }
    push!(Line::from(""), usize::MAX);

    // ── GIT IGNORAR ───────────────────────────────────────────────────────────
    push!(section_line(ui.t("section.git_ignore")), usize::MAX);
    push!(sep_line(), usize::MAX);

    let add_git_selected = cur == cf_add_git;
    let add_git_editing = add_git_selected && edit_buf.is_some();
    let add_git_prefix = if add_git_selected {
        Span::styled(
            " > ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("   ")
    };
    let add_git_val = if add_git_editing {
        let (buf, c) = edit_buf.unwrap_or(("", 0));
        Span::styled(
            edit_render_cursor(buf, c),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            ui.t("add_git_path"),
            if add_git_selected {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        )
    };
    push!(Line::from(vec![add_git_prefix, add_git_val]), cf_add_git);

    for (i, path) in cfg.ignored_git_paths.iter().enumerate() {
        let idx = cf_add_git + 1 + i;
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
            let (buf, c) = edit_buf.unwrap_or(("", 0));
            Span::styled(
                edit_render_cursor(buf, c),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                format!("- {path}"),
                if sel {
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Magenta)
                },
            )
        };
        let del_hint = if sel && !ed {
            Span::styled(ui.t("delete_hint"), Style::default().fg(Color::DarkGray))
        } else {
            Span::raw("")
        };
        push!(Line::from(vec![prefix, val_span, del_hint]), idx);
    }
    push!(Line::from(""), usize::MAX);

    // ── Status / ações ────────────────────────────────────────────────────────
    if let Some(status) = &app.config_status {
        push!(
            Line::from(Span::styled(
                format!("  ✓ {status}"),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            usize::MAX
        );
        push!(Line::from(""), usize::MAX);
    }
    push!(
        Line::from(Span::styled(
            "  P  purge git logs  |  R  reload",
            Style::default().fg(Color::DarkGray),
        )),
        usize::MAX
    );
    push!(Line::from(""), usize::MAX);

    // ── PATHS (read-only) ─────────────────────────────────────────────────────
    push!(section_line("PATHS"), usize::MAX);
    push!(sep_line(), usize::MAX);
    push!(
        Line::from(vec![
            Span::raw("   "),
            Span::styled("logs              ", Style::default().fg(Color::White)),
            Span::styled(
                crate::config::log_dir().display().to_string(),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        usize::MAX
    );
    push!(
        Line::from(vec![
            Span::raw("   "),
            Span::styled("config            ", Style::default().fg(Color::White)),
            Span::styled(
                crate::config::config_path().display().to_string(),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        usize::MAX
    );

    app.config_row_map = row_map;

    let p = Paragraph::new(lines).block(block).scroll((app.scroll, 0));
    f.render_widget(p, area);
}
