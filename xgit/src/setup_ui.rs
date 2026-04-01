use crate::code_file_types::{
    categories, category_selected_count, category_state, file_rules_from_selection,
    selection_from_file_rules, set_category_selected, total_builtin_entry_count, TriState,
};
use crate::config::{
    save_config, AnnotateOldCodeLineLayout, AnnotateOldCodeMode, AppConfig, FileRuleConfig,
};
use crate::i18n::Catalog;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use std::collections::BTreeSet;
use std::io::{stdout, IsTerminal};
use std::path::Path;
use std::time::Duration;

const SECTION_KEYS: [&str; 5] = [
    "setup.section.ui",
    "setup.section.features",
    "setup.section.push",
    "setup.section.identity",
    "setup.section.annotate",
];

const EXIT_OPTIONS: [&str; 3] = [
    "setup.confirm.save_exit",
    "setup.confirm.discard_exit",
    "setup.confirm.cancel",
];

pub fn run_setup_ui(catalog: &Catalog, initial: &AppConfig, target: &Path) -> Result<()> {
    if !stdout().is_terminal() {
        anyhow::bail!("{}", catalog.t("error.setup.terminal_required"));
    }
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    let mut state = SetupState::default();
    let mut working = initial.clone();

    loop {
        terminal.draw(|frame| draw(frame, catalog, target, &working, &state))?;

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if handle_key_code(key.code, catalog, target, &mut working, &mut state)? {
            break;
        }
    }

    terminal.show_cursor()?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Menu,
    Fields,
}

impl Default for Focus {
    fn default() -> Self {
        Self::Menu
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodeFileTypePickerLevel {
    Categories,
    Entries,
}

#[derive(Debug, Clone)]
struct CodeFileTypePickerState {
    level: CodeFileTypePickerLevel,
    category_index: usize,
    entry_index: usize,
    selected_keys: BTreeSet<String>,
    passthrough_rules: Vec<FileRuleConfig>,
}

#[derive(Debug, Default)]
struct SetupState {
    section: usize,
    field: usize,
    focus: Focus,
    editing: bool,
    input: String,
    status: String,
    dirty: bool,
    confirm_exit: bool,
    confirm_choice: usize,
    code_file_type_picker: Option<CodeFileTypePickerState>,
}

fn handle_key_code(
    code: KeyCode,
    catalog: &Catalog,
    target: &Path,
    working: &mut AppConfig,
    state: &mut SetupState,
) -> Result<bool> {
    if state.editing {
        match code {
            KeyCode::Enter => {
                apply_text(working, state.section, state.field, state.input.clone());
                state.editing = false;
                state.dirty = true;
                state.status = catalog.t("setup.status.field_updated");
            }
            KeyCode::Esc => {
                state.editing = false;
                state.status = catalog.t("setup.status.edit_canceled");
            }
            KeyCode::Backspace => {
                state.input.pop();
            }
            KeyCode::Char(ch) => {
                state.input.push(ch);
            }
            _ => {}
        }
        return Ok(false);
    }

    if state.code_file_type_picker.is_some() {
        return handle_code_file_type_picker_key(code, catalog, working, state);
    }

    if state.confirm_exit {
        return handle_exit_confirm(code, catalog, target, working, state);
    }

    match code {
        KeyCode::Char('q') => {
            return request_exit(catalog, state);
        }
        KeyCode::Esc => {
            if state.focus == Focus::Fields {
                state.focus = Focus::Menu;
                state.status = catalog.t("setup.status.back_to_menu");
                return Ok(false);
            }
            return request_exit(catalog, state);
        }
        KeyCode::Up => {
            if state.focus == Focus::Menu {
                state.section = if state.section == 0 {
                    SECTION_KEYS.len() - 1
                } else {
                    state.section - 1
                };
                state.field = 0;
            } else {
                let count = field_count(state.section, working);
                if count > 0 {
                    state.field = if state.field == 0 {
                        count.saturating_sub(1)
                    } else {
                        state.field - 1
                    };
                }
            }
        }
        KeyCode::Down => {
            if state.focus == Focus::Menu {
                state.section = (state.section + 1) % SECTION_KEYS.len();
                state.field = 0;
            } else {
                let count = field_count(state.section, working);
                if count > 0 {
                    state.field = (state.field + 1) % count;
                }
            }
        }
        KeyCode::Enter => {
            if state.focus == Focus::Menu {
                state.focus = Focus::Fields;
                let count = field_count(state.section, working);
                if count == 0 {
                    state.field = 0;
                } else {
                    state.field = state.field.min(count - 1);
                }
                state.status = catalog.t("setup.status.enter_section");
            } else {
                handle_enter_on_field(catalog, working, state);
            }
        }
        KeyCode::Left | KeyCode::Right => {
            if state.focus == Focus::Fields {
                if toggle_field(working, state.section, state.field) {
                    state.dirty = true;
                    state.status = catalog.t("setup.status.field_updated");
                }
            }
        }
        KeyCode::Char('e') => {
            if state.focus == Focus::Fields {
                begin_edit_if_text(catalog, working, state);
            }
        }
        KeyCode::Char('s') => {
            save_config(target, working)?;
            state.dirty = false;
            state.status = catalog.tf(
                "setup.status.saved",
                &[("path", target.display().to_string())],
            );
        }
        _ => {}
    }

    Ok(false)
}

fn request_exit(catalog: &Catalog, state: &mut SetupState) -> Result<bool> {
    if state.dirty {
        state.confirm_exit = true;
        state.confirm_choice = 0;
        state.status = catalog.t("setup.status.confirm_exit");
        Ok(false)
    } else {
        Ok(true)
    }
}

fn handle_exit_confirm(
    code: KeyCode,
    catalog: &Catalog,
    target: &Path,
    working: &mut AppConfig,
    state: &mut SetupState,
) -> Result<bool> {
    match code {
        KeyCode::Up => {
            state.confirm_choice = if state.confirm_choice == 0 {
                EXIT_OPTIONS.len() - 1
            } else {
                state.confirm_choice - 1
            };
        }
        KeyCode::Down => {
            state.confirm_choice = (state.confirm_choice + 1) % EXIT_OPTIONS.len();
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            state.confirm_exit = false;
            state.status = catalog.t("setup.status.exit_canceled");
        }
        KeyCode::Enter => match state.confirm_choice {
            0 => {
                save_config(target, working)?;
                state.dirty = false;
                state.status = catalog.tf(
                    "setup.status.saved",
                    &[("path", target.display().to_string())],
                );
                return Ok(true);
            }
            1 => {
                state.status = catalog.t("setup.status.exit_discarded");
                return Ok(true);
            }
            _ => {
                state.confirm_exit = false;
                state.status = catalog.t("setup.status.exit_canceled");
            }
        },
        _ => {}
    }

    Ok(false)
}

fn handle_enter_on_field(catalog: &Catalog, working: &mut AppConfig, state: &mut SetupState) {
    if (state.section, state.field) == (4, 3) {
        open_code_file_type_picker(working, state);
        state.status = catalog.t("setup.status.code_file_types_opened");
        return;
    }

    if toggle_field(working, state.section, state.field) {
        state.dirty = true;
        state.status = catalog.t("setup.status.field_updated");
    } else {
        begin_edit_if_text(catalog, working, state);
    }
}

fn open_code_file_type_picker(config: &AppConfig, state: &mut SetupState) {
    let selection = selection_from_file_rules(&config.annotate.file_rules);
    state.code_file_type_picker = Some(CodeFileTypePickerState {
        level: CodeFileTypePickerLevel::Categories,
        category_index: 0,
        entry_index: 0,
        selected_keys: selection.selected_keys,
        passthrough_rules: selection.passthrough_rules,
    });
}

fn handle_code_file_type_picker_key(
    code: KeyCode,
    catalog: &Catalog,
    working: &mut AppConfig,
    state: &mut SetupState,
) -> Result<bool> {
    let catalog_entries = categories();
    if catalog_entries.is_empty() {
        state.code_file_type_picker = None;
        state.status = catalog.t("setup.status.code_file_types_unavailable");
        return Ok(false);
    }

    let Some(picker) = state.code_file_type_picker.as_mut() else {
        return Ok(false);
    };

    match code {
        KeyCode::Up => match picker.level {
            CodeFileTypePickerLevel::Categories => {
                picker.category_index = if picker.category_index == 0 {
                    catalog_entries.len() - 1
                } else {
                    picker.category_index - 1
                };
            }
            CodeFileTypePickerLevel::Entries => {
                let entries = catalog_entries[picker.category_index].entries;
                if !entries.is_empty() {
                    picker.entry_index = if picker.entry_index == 0 {
                        entries.len() - 1
                    } else {
                        picker.entry_index - 1
                    };
                }
            }
        },
        KeyCode::Down => match picker.level {
            CodeFileTypePickerLevel::Categories => {
                picker.category_index = (picker.category_index + 1) % catalog_entries.len();
            }
            CodeFileTypePickerLevel::Entries => {
                let entries = catalog_entries[picker.category_index].entries;
                if !entries.is_empty() {
                    picker.entry_index = (picker.entry_index + 1) % entries.len();
                }
            }
        },
        KeyCode::Enter => match picker.level {
            CodeFileTypePickerLevel::Categories => {
                picker.level = CodeFileTypePickerLevel::Entries;
                picker.entry_index = 0;
            }
            CodeFileTypePickerLevel::Entries => {
                picker.level = CodeFileTypePickerLevel::Categories;
            }
        },
        KeyCode::Char(' ') => match picker.level {
            CodeFileTypePickerLevel::Categories => {
                let category = &catalog_entries[picker.category_index];
                let next_selected = !matches!(
                    category_state(category, &picker.selected_keys),
                    TriState::All
                );
                set_category_selected(category, &mut picker.selected_keys, next_selected);
            }
            CodeFileTypePickerLevel::Entries => {
                let category = &catalog_entries[picker.category_index];
                if let Some(entry) = category.entries.get(picker.entry_index) {
                    if picker.selected_keys.contains(entry.key) {
                        picker.selected_keys.remove(entry.key);
                    } else {
                        picker.selected_keys.insert(entry.key.to_string());
                    }
                }
            }
        },
        KeyCode::Esc => match picker.level {
            CodeFileTypePickerLevel::Entries => {
                picker.level = CodeFileTypePickerLevel::Categories;
            }
            CodeFileTypePickerLevel::Categories => {
                close_code_file_type_picker(catalog, working, state);
            }
        },
        _ => {}
    }

    Ok(false)
}

fn close_code_file_type_picker(catalog: &Catalog, working: &mut AppConfig, state: &mut SetupState) {
    let Some(picker) = state.code_file_type_picker.take() else {
        return;
    };
    let next_rules = file_rules_from_selection(&picker.selected_keys, &picker.passthrough_rules);
    if next_rules != working.annotate.file_rules {
        working.annotate.file_rules = next_rules;
        state.dirty = true;
        state.status = catalog.t("setup.status.code_file_types_updated");
    } else {
        state.status = catalog.t("setup.status.code_file_types_unchanged");
    }
}

fn begin_edit_if_text(catalog: &Catalog, working: &AppConfig, state: &mut SetupState) {
    if let Some(current) = get_text(working, state.section, state.field) {
        state.input = current;
        state.editing = true;
        state.status = catalog.t("setup.status.editing_field");
    }
}

fn draw(
    frame: &mut ratatui::Frame,
    catalog: &Catalog,
    target: &Path,
    config: &AppConfig,
    state: &SetupState,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(26), Constraint::Min(30)])
        .split(frame.area());

    let section_items: Vec<ListItem> = SECTION_KEYS
        .iter()
        .map(|key| ListItem::new(Line::from(catalog.t(key))))
        .collect();
    let mut section_state = ListState::default();
    section_state.select(Some(state.section));
    let menu_highlight = if state.focus == Focus::Menu {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let section_list = List::new(section_items)
        .block(
            Block::default()
                .title(catalog.t("setup.block.menu"))
                .borders(Borders::ALL),
        )
        .highlight_style(menu_highlight);
    frame.render_stateful_widget(section_list, chunks[0], &mut section_state);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(4),
            Constraint::Length(3),
        ])
        .split(chunks[1]);

    let help = Paragraph::new(vec![
        Line::from(catalog.tf(
            "setup.help.target",
            &[("path", target.display().to_string())],
        )),
        Line::from(if state.code_file_type_picker.is_some() {
            catalog.t("setup.help.code_file_types_nav")
        } else if state.focus == Focus::Menu {
            catalog.t("setup.help.menu_nav")
        } else {
            catalog.t("setup.help.field_nav")
        }),
        Line::from(if state.code_file_type_picker.is_some() {
            catalog.t("setup.help.code_file_types_actions")
        } else if state.focus == Focus::Menu {
            catalog.t("setup.help.menu_actions")
        } else {
            catalog.t("setup.help.field_actions")
        }),
    ])
    .block(
        Block::default()
            .title(catalog.t("setup.block.setup"))
            .borders(Borders::ALL),
    )
    .wrap(Wrap { trim: false });
    frame.render_widget(help, right[0]);

    let fields = field_lines(catalog, config, state.section);
    let field_items: Vec<ListItem> = fields
        .iter()
        .map(|line| ListItem::new(Line::from(line.to_string())))
        .collect();
    let mut field_state = ListState::default();
    field_state.select(Some(state.field));
    let field_highlight = if state.focus == Focus::Fields {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let field_list = List::new(field_items)
        .block(
            Block::default()
                .title(catalog.t(SECTION_KEYS[state.section]))
                .borders(Borders::ALL),
        )
        .highlight_style(field_highlight);
    frame.render_stateful_widget(field_list, right[1], &mut field_state);

    let edit_text = if state.editing {
        catalog.tf("setup.editor.on", &[("value", state.input.clone())])
    } else {
        catalog.t("setup.editor.off")
    };
    let edit = Paragraph::new(edit_text)
        .block(
            Block::default()
                .title(catalog.t("setup.block.editor"))
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(edit, right[2]);

    let status = Paragraph::new(Line::from(Span::raw(if state.status.is_empty() {
        catalog.t("setup.status.ready")
    } else {
        state.status.clone()
    })))
    .block(
        Block::default()
            .title(catalog.t("setup.block.status"))
            .borders(Borders::ALL),
    );
    frame.render_widget(status, right[3]);

    if state.confirm_exit {
        draw_exit_confirm(frame, catalog, state);
    } else if let Some(picker) = &state.code_file_type_picker {
        draw_code_file_type_picker(frame, catalog, picker);
    }
}

fn draw_exit_confirm(frame: &mut ratatui::Frame, catalog: &Catalog, state: &SetupState) {
    let area = centered_rect(60, 45, frame.area());
    frame.render_widget(Clear, area);

    let options: Vec<ListItem> = EXIT_OPTIONS
        .iter()
        .map(|key| ListItem::new(Line::from(catalog.t(key))))
        .collect();
    let mut confirm_state = ListState::default();
    confirm_state.select(Some(state.confirm_choice));

    let confirm_list = List::new(options)
        .block(
            Block::default()
                .title(catalog.t("setup.block.confirm_exit"))
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(confirm_list, area, &mut confirm_state);

    let hint_area = Rect {
        x: area.x,
        y: area.y.saturating_add(area.height.saturating_sub(1)),
        width: area.width,
        height: 1,
    };
    let hint = Paragraph::new(catalog.t("setup.confirm.hint"));
    frame.render_widget(hint, hint_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_code_file_type_picker(
    frame: &mut ratatui::Frame,
    catalog: &Catalog,
    picker: &CodeFileTypePickerState,
) {
    let area = centered_rect(70, 60, frame.area());
    frame.render_widget(Clear, area);

    let catalog_entries = categories();
    if catalog_entries.is_empty() {
        return;
    }

    let title = match picker.level {
        CodeFileTypePickerLevel::Categories => catalog.t("setup.block.code_file_types"),
        CodeFileTypePickerLevel::Entries => {
            let category = &catalog_entries[picker.category_index];
            let category_label =
                localized_label(catalog, category.label_key, category.default_label);
            catalog.tf(
                "setup.block.code_file_types_entries",
                &[("category", category_label)],
            )
        }
    };

    let items = match picker.level {
        CodeFileTypePickerLevel::Categories => catalog_entries
            .iter()
            .map(|category| {
                let label = localized_label(catalog, category.label_key, category.default_label);
                let selected = category_selected_count(category, &picker.selected_keys);
                let marker = tri_state_marker(category_state(category, &picker.selected_keys));
                ListItem::new(Line::from(format!(
                    "{marker} {label} ({selected}/{})",
                    category.entries.len()
                )))
            })
            .collect::<Vec<_>>(),
        CodeFileTypePickerLevel::Entries => {
            let category = &catalog_entries[picker.category_index];
            category
                .entries
                .iter()
                .map(|entry| {
                    let label = localized_label(catalog, entry.label_key, entry.default_label);
                    let marker = if picker.selected_keys.contains(entry.key) {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    ListItem::new(Line::from(format!("{marker} {label} ({})", entry.pattern)))
                })
                .collect::<Vec<_>>()
        }
    };

    let mut list_state = ListState::default();
    list_state.select(Some(match picker.level {
        CodeFileTypePickerLevel::Categories => picker.category_index,
        CodeFileTypePickerLevel::Entries => picker.entry_index,
    }));

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(1)])
        .split(area);

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, popup_layout[0], &mut list_state);

    let hint = match picker.level {
        CodeFileTypePickerLevel::Categories => catalog.t("setup.code_file_type.hint.categories"),
        CodeFileTypePickerLevel::Entries => catalog.t("setup.code_file_type.hint.entries"),
    };
    frame.render_widget(Paragraph::new(hint), popup_layout[1]);
}

fn tri_state_marker(state: TriState) -> &'static str {
    match state {
        TriState::All => "[x]",
        TriState::Partial => "[-]",
        TriState::None => "[ ]",
    }
}

fn localized_label(catalog: &Catalog, key: &str, fallback: &str) -> String {
    let value = catalog.t(key);
    if value == key {
        fallback.to_string()
    } else {
        value
    }
}

fn field_count(section: usize, _config: &AppConfig) -> usize {
    match section {
        0 => 1,
        1 => 2,
        2 => 1,
        3 => 3,
        4 => 20,
        _ => 0,
    }
}

fn field_lines(catalog: &Catalog, config: &AppConfig, section: usize) -> Vec<String> {
    match section {
        0 => vec![format!(
            "{}: {}",
            catalog.t("setup.field.ui.lang"),
            language_value(catalog, config.ui.lang.as_str())
        )],
        1 => vec![
            format!(
                "{}: {}",
                catalog.t("setup.field.features.push"),
                bool_text(catalog, config.features.push)
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.features.annotate"),
                bool_text(catalog, config.features.annotate)
            ),
        ],
        2 => vec![format!(
            "{}: {}",
            catalog.t("setup.field.push.placeholder"),
            config.push.placeholder.clone().unwrap_or_default()
        )],
        3 => vec![
            format!(
                "{}: {}",
                catalog.t("setup.field.identity.author_tag"),
                config.identity.author_tag.clone().unwrap_or_default()
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.identity.name"),
                config.identity.name.clone().unwrap_or_default()
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.identity.email"),
                config.identity.email.clone().unwrap_or_default()
            ),
        ],
        4 => vec![
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.include_untracked"),
                bool_text(catalog, config.annotate.staged.include_untracked)
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.form_fields"),
                config.annotate.form.fields.join(",")
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.reference_kinds"),
                config.annotate.reference_kinds.join(",")
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.code_file_types"),
                code_file_type_summary(catalog, &config.annotate.file_rules)
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.align_with_code_indent"),
                bool_text(catalog, config.annotate.render.align_with_code_indent)
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.wrap_blank_lines"),
                bool_text(catalog, config.annotate.render.wrap_blank_lines)
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.template_add_start"),
                config.annotate.block_templates.add.start
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.template_add_end"),
                config.annotate.block_templates.add.end
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.template_modify_start"),
                config.annotate.block_templates.modify.start
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.template_modify_end"),
                config.annotate.block_templates.modify.end
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.template_del_start"),
                config.annotate.block_templates.del.start
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.template_del_end"),
                config.annotate.block_templates.del.end
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.date_format"),
                config.annotate.date.format
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.old_code_mode"),
                old_code_mode_text(catalog, config.annotate.old_code.mode.as_ref())
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.old_code_line_layout"),
                old_code_line_layout_text(
                    catalog,
                    config.annotate.old_code.line_comment.layout.clone()
                )
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.old_code_line_header"),
                config.annotate.old_code.line_comment.header
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.old_code_line_body_prefix"),
                config.annotate.old_code.line_comment.body_prefix
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.old_code_line_body_suffix"),
                config.annotate.old_code.line_comment.body_suffix
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.old_code_block_title"),
                config.annotate.old_code.block_comment.title
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.old_code_block_body_prefix"),
                config.annotate.old_code.block_comment.body_prefix
            ),
        ],
        _ => vec![],
    }
}

fn code_file_type_summary(catalog: &Catalog, file_rules: &[FileRuleConfig]) -> String {
    let selection = selection_from_file_rules(file_rules);
    let selected = selection.selected_keys.len();
    let total = total_builtin_entry_count();
    let mut summary = catalog.tf(
        "setup.value.code_file_types_summary",
        &[
            ("selected", selected.to_string()),
            ("total", total.to_string()),
        ],
    );

    if !selection.passthrough_rules.is_empty() {
        summary = catalog.tf(
            "setup.value.code_file_types_summary_with_custom",
            &[
                ("summary", summary),
                ("count", selection.passthrough_rules.len().to_string()),
            ],
        );
    }

    summary
}

fn bool_text(catalog: &Catalog, value: bool) -> String {
    if value {
        catalog.t("setup.value.enabled")
    } else {
        catalog.t("setup.value.disabled")
    }
}

fn language_value(catalog: &Catalog, value: &str) -> String {
    match value {
        "zh-CN" => catalog.t("setup.value.lang.zh_cn"),
        "en-US" => catalog.t("setup.value.lang.en_us"),
        _ => value.to_string(),
    }
}

fn old_code_mode_text(catalog: &Catalog, mode: Option<&AnnotateOldCodeMode>) -> String {
    match mode {
        None => catalog.t("setup.value.annotate.old_code_mode.legacy"),
        Some(AnnotateOldCodeMode::None) => catalog.t("setup.value.annotate.old_code_mode.none"),
        Some(AnnotateOldCodeMode::LineComment) => {
            catalog.t("setup.value.annotate.old_code_mode.line_comment")
        }
        Some(AnnotateOldCodeMode::BlockComment) => {
            catalog.t("setup.value.annotate.old_code_mode.block_comment")
        }
    }
}

fn old_code_line_layout_text(catalog: &Catalog, layout: AnnotateOldCodeLineLayout) -> String {
    match layout {
        AnnotateOldCodeLineLayout::PerLine => {
            catalog.t("setup.value.annotate.old_code_line_layout.per_line")
        }
        AnnotateOldCodeLineLayout::HeaderBody => {
            catalog.t("setup.value.annotate.old_code_line_layout.header_body")
        }
    }
}

fn cycle_old_code_mode(mode: Option<AnnotateOldCodeMode>) -> Option<AnnotateOldCodeMode> {
    match mode {
        None => Some(AnnotateOldCodeMode::LineComment),
        Some(AnnotateOldCodeMode::LineComment) => Some(AnnotateOldCodeMode::BlockComment),
        Some(AnnotateOldCodeMode::BlockComment) => Some(AnnotateOldCodeMode::None),
        Some(AnnotateOldCodeMode::None) => None,
    }
}

fn cycle_old_code_line_layout(layout: AnnotateOldCodeLineLayout) -> AnnotateOldCodeLineLayout {
    match layout {
        AnnotateOldCodeLineLayout::PerLine => AnnotateOldCodeLineLayout::HeaderBody,
        AnnotateOldCodeLineLayout::HeaderBody => AnnotateOldCodeLineLayout::PerLine,
    }
}

fn toggle_field(config: &mut AppConfig, section: usize, field: usize) -> bool {
    match (section, field) {
        (0, 0) => {
            config.ui.lang = if config.ui.lang == "zh-CN" {
                "en-US".to_string()
            } else {
                "zh-CN".to_string()
            };
            true
        }
        (1, 0) => {
            config.features.push = !config.features.push;
            true
        }
        (1, 1) => {
            config.features.annotate = !config.features.annotate;
            true
        }
        (4, 0) => {
            config.annotate.staged.include_untracked = !config.annotate.staged.include_untracked;
            true
        }
        (4, 4) => {
            config.annotate.render.align_with_code_indent =
                !config.annotate.render.align_with_code_indent;
            true
        }
        (4, 5) => {
            config.annotate.render.wrap_blank_lines = !config.annotate.render.wrap_blank_lines;
            true
        }
        (4, 13) => {
            config.annotate.old_code.mode =
                cycle_old_code_mode(config.annotate.old_code.mode.clone());
            true
        }
        (4, 14) => {
            config.annotate.old_code.line_comment.layout =
                cycle_old_code_line_layout(config.annotate.old_code.line_comment.layout.clone());
            true
        }
        _ => false,
    }
}

fn get_text(config: &AppConfig, section: usize, field: usize) -> Option<String> {
    match (section, field) {
        (2, 0) => Some(config.push.placeholder.clone().unwrap_or_default()),
        (3, 0) => Some(config.identity.author_tag.clone().unwrap_or_default()),
        (3, 1) => Some(config.identity.name.clone().unwrap_or_default()),
        (3, 2) => Some(config.identity.email.clone().unwrap_or_default()),
        (4, 1) => Some(config.annotate.form.fields.join(",")),
        (4, 2) => Some(config.annotate.reference_kinds.join(",")),
        (4, 6) => Some(config.annotate.block_templates.add.start.clone()),
        (4, 7) => Some(config.annotate.block_templates.add.end.clone()),
        (4, 8) => Some(config.annotate.block_templates.modify.start.clone()),
        (4, 9) => Some(config.annotate.block_templates.modify.end.clone()),
        (4, 10) => Some(config.annotate.block_templates.del.start.clone()),
        (4, 11) => Some(config.annotate.block_templates.del.end.clone()),
        (4, 12) => Some(config.annotate.date.format.clone()),
        (4, 15) => Some(config.annotate.old_code.line_comment.header.clone()),
        (4, 16) => Some(config.annotate.old_code.line_comment.body_prefix.clone()),
        (4, 17) => Some(config.annotate.old_code.line_comment.body_suffix.clone()),
        (4, 18) => Some(config.annotate.old_code.block_comment.title.clone()),
        (4, 19) => Some(config.annotate.old_code.block_comment.body_prefix.clone()),
        _ => None,
    }
}

fn apply_text(config: &mut AppConfig, section: usize, field: usize, value: String) {
    match (section, field) {
        (2, 0) => config.push.placeholder = empty_to_none(&value),
        (3, 0) => config.identity.author_tag = empty_to_none(&value),
        (3, 1) => config.identity.name = empty_to_none(&value),
        (3, 2) => config.identity.email = empty_to_none(&value),
        (4, 1) => {
            config.annotate.form.fields = split_csv(value);
        }
        (4, 2) => {
            config.annotate.reference_kinds = split_csv(value);
        }
        (4, 6) => config.annotate.block_templates.add.start = value,
        (4, 7) => config.annotate.block_templates.add.end = value,
        (4, 8) => config.annotate.block_templates.modify.start = value,
        (4, 9) => config.annotate.block_templates.modify.end = value,
        (4, 10) => config.annotate.block_templates.del.start = value,
        (4, 11) => config.annotate.block_templates.del.end = value,
        (4, 12) => config.annotate.date.format = value,
        (4, 15) => config.annotate.old_code.line_comment.header = value,
        (4, 16) => config.annotate.old_code.line_comment.body_prefix = value,
        (4, 17) => config.annotate.old_code.line_comment.body_suffix = value,
        (4, 18) => config.annotate.old_code.block_comment.title = value,
        (4, 19) => config.annotate.old_code.block_comment.body_prefix = value,
        _ => {}
    }
}

fn empty_to_none(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn split_csv(value: String) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
    }
}

#[cfg(test)]
mod tests {
    use super::{handle_key_code, Focus, SetupState};
    use crate::code_file_types::{categories, category_state, TriState};
    use crate::config::{AnnotateOldCodeMode, AppConfig, FileRuleConfig};
    use crate::i18n;
    use crossterm::event::KeyCode;
    use std::path::Path;
    use tempfile::TempDir;

    fn test_catalog() -> crate::i18n::Catalog {
        i18n::load_catalog("en-US", Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    #[test]
    fn esc_returns_from_section_to_menu() {
        let catalog = test_catalog();
        let mut config = AppConfig::default();
        let mut state = SetupState::default();
        let target = Path::new("/tmp/xgit-setup-test.toml");

        let exit =
            handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        assert!(!exit);
        assert_eq!(state.focus, Focus::Fields);

        let exit =
            handle_key_code(KeyCode::Esc, &catalog, target, &mut config, &mut state).unwrap();
        assert!(!exit);
        assert_eq!(state.focus, Focus::Menu);
    }

    #[test]
    fn esc_on_menu_with_dirty_opens_confirm_and_can_cancel() {
        let catalog = test_catalog();
        let mut config = AppConfig::default();
        let mut state = SetupState::default();
        let target = Path::new("/tmp/xgit-setup-test.toml");

        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        assert!(state.dirty);

        handle_key_code(KeyCode::Esc, &catalog, target, &mut config, &mut state).unwrap();
        assert_eq!(state.focus, Focus::Menu);

        let exit =
            handle_key_code(KeyCode::Esc, &catalog, target, &mut config, &mut state).unwrap();
        assert!(!exit);
        assert!(state.confirm_exit);

        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        let exit =
            handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        assert!(!exit);
        assert!(!state.confirm_exit);
    }

    #[test]
    fn exit_confirm_save_writes_file() {
        let catalog = test_catalog();
        let tmp = TempDir::new().unwrap();
        let target_path = tmp.path().join("config.toml");
        let mut config = AppConfig::default();
        let mut state = SetupState::default();

        handle_key_code(
            KeyCode::Enter,
            &catalog,
            &target_path,
            &mut config,
            &mut state,
        )
        .unwrap();
        handle_key_code(
            KeyCode::Enter,
            &catalog,
            &target_path,
            &mut config,
            &mut state,
        )
        .unwrap();
        handle_key_code(
            KeyCode::Esc,
            &catalog,
            &target_path,
            &mut config,
            &mut state,
        )
        .unwrap();
        handle_key_code(
            KeyCode::Esc,
            &catalog,
            &target_path,
            &mut config,
            &mut state,
        )
        .unwrap();

        let exit = handle_key_code(
            KeyCode::Enter,
            &catalog,
            &target_path,
            &mut config,
            &mut state,
        )
        .unwrap();
        assert!(exit);

        let raw = std::fs::read_to_string(&target_path).unwrap();
        assert!(raw.contains("lang = \"en-US\""));
    }

    #[test]
    fn save_writes_block_template_model_for_annotate() {
        let catalog = test_catalog();
        let tmp = TempDir::new().unwrap();
        let target_path = tmp.path().join("config.toml");
        let mut config = AppConfig::default();
        config.annotate.block_templates.add.start = "// custom add {@".to_string();
        config.annotate.block_templates.add.end = "//@}".to_string();
        let mut state = SetupState::default();

        handle_key_code(
            KeyCode::Char('s'),
            &catalog,
            &target_path,
            &mut config,
            &mut state,
        )
        .unwrap();

        let raw = std::fs::read_to_string(&target_path).unwrap();
        assert!(raw.contains("[annotate.block_templates.add]"));
        assert!(raw.contains("start = \"// custom add {@\""));
        assert!(raw.contains("end = \"//@}\""));
        assert!(!raw.contains("[annotate.policies]"));
    }

    #[test]
    fn save_writes_date_and_old_code_model_for_annotate() {
        let catalog = test_catalog();
        let tmp = TempDir::new().unwrap();
        let target_path = tmp.path().join("config.toml");
        let mut config = AppConfig::default();
        config.annotate.date.format = "dd/mm/yyyy".to_string();
        config.annotate.old_code.mode = Some(AnnotateOldCodeMode::BlockComment);
        config.annotate.old_code.block_comment.title = "cover old codes".to_string();
        config.annotate.old_code.block_comment.body_prefix = "| ".to_string();
        let mut state = SetupState::default();

        handle_key_code(
            KeyCode::Char('s'),
            &catalog,
            &target_path,
            &mut config,
            &mut state,
        )
        .unwrap();

        let raw = std::fs::read_to_string(&target_path).unwrap();
        assert!(raw.contains("[annotate.date]"));
        assert!(raw.contains("format = \"dd/mm/yyyy\""));
        assert!(raw.contains("[annotate.old_code]"));
        assert!(raw.contains("mode = \"block_comment\""));
        assert!(raw.contains("[annotate.old_code.block_comment]"));
        assert!(raw.contains("title = \"cover old codes\""));
        assert!(raw.contains("body_prefix = \"| \""));
    }

    #[test]
    fn exit_confirm_discard_does_not_write_file() {
        let catalog = test_catalog();
        let tmp = TempDir::new().unwrap();
        let target_path = tmp.path().join("config.toml");
        let mut config = AppConfig::default();
        let mut state = SetupState::default();

        handle_key_code(
            KeyCode::Enter,
            &catalog,
            &target_path,
            &mut config,
            &mut state,
        )
        .unwrap();
        handle_key_code(
            KeyCode::Enter,
            &catalog,
            &target_path,
            &mut config,
            &mut state,
        )
        .unwrap();
        handle_key_code(
            KeyCode::Esc,
            &catalog,
            &target_path,
            &mut config,
            &mut state,
        )
        .unwrap();
        handle_key_code(
            KeyCode::Esc,
            &catalog,
            &target_path,
            &mut config,
            &mut state,
        )
        .unwrap();
        handle_key_code(
            KeyCode::Down,
            &catalog,
            &target_path,
            &mut config,
            &mut state,
        )
        .unwrap();

        let exit = handle_key_code(
            KeyCode::Enter,
            &catalog,
            &target_path,
            &mut config,
            &mut state,
        )
        .unwrap();
        assert!(exit);
        assert!(!target_path.exists());
    }

    #[test]
    fn code_file_type_picker_supports_hierarchy_partial_and_save_mapping() {
        let catalog = test_catalog();
        let target = Path::new("/tmp/xgit-setup-test.toml");
        let mut config = AppConfig::default();
        config.annotate.file_rules.push(FileRuleConfig {
            pattern: "*.proto".to_string(),
            renderer: "proto_line_block".to_string(),
        });
        let mut state = SetupState::default();

        for _ in 0..4 {
            handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        }
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        for _ in 0..3 {
            handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        }

        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        let picker = state.code_file_type_picker.as_ref().unwrap();
        assert!(picker.selected_keys.contains("c_cpp/c"));
        assert!(picker.selected_keys.contains("java/java"));

        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(
            KeyCode::Char(' '),
            &catalog,
            target,
            &mut config,
            &mut state,
        )
        .unwrap();
        handle_key_code(KeyCode::Esc, &catalog, target, &mut config, &mut state).unwrap();

        let picker = state.code_file_type_picker.as_ref().unwrap();
        assert_eq!(
            category_state(&categories()[0], &picker.selected_keys),
            TriState::Partial
        );

        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(
            KeyCode::Char(' '),
            &catalog,
            target,
            &mut config,
            &mut state,
        )
        .unwrap();
        handle_key_code(KeyCode::Esc, &catalog, target, &mut config, &mut state).unwrap();

        assert!(state.code_file_type_picker.is_none());
        assert!(state.dirty);
        assert!(config
            .annotate
            .file_rules
            .iter()
            .any(|rule| rule.pattern == "*.c" && rule.renderer == "c_line_block"));
        assert!(!config
            .annotate
            .file_rules
            .iter()
            .any(|rule| rule.pattern == "*.h" && rule.renderer == "c_line_block"));
        assert!(config
            .annotate
            .file_rules
            .iter()
            .any(|rule| rule.pattern == "*.cpp" && rule.renderer == "c_line_block"));
        assert!(config
            .annotate
            .file_rules
            .iter()
            .any(|rule| rule.pattern == "*.java" && rule.renderer == "c_line_block"));
        assert!(config
            .annotate
            .file_rules
            .iter()
            .any(|rule| rule.pattern == "*.js" && rule.renderer == "c_line_block"));
        assert!(config
            .annotate
            .file_rules
            .iter()
            .any(|rule| rule.pattern == "*.proto" && rule.renderer == "proto_line_block"));
    }
}
