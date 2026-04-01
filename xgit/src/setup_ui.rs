use crate::config::{save_config, AppConfig, FileRuleConfig};
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
                handle_toggle_or_edit(catalog, working, state);
            }
        }
        KeyCode::Left | KeyCode::Right => {
            if state.focus == Focus::Fields {
                handle_toggle_or_edit(catalog, working, state);
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

fn handle_toggle_or_edit(catalog: &Catalog, working: &mut AppConfig, state: &mut SetupState) {
    if toggle_field(working, state.section, state.field) {
        state.dirty = true;
        state.status = catalog.t("setup.status.field_updated");
    } else {
        begin_edit_if_text(catalog, working, state);
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
        Line::from(if state.focus == Focus::Menu {
            catalog.t("setup.help.menu_nav")
        } else {
            catalog.t("setup.help.field_nav")
        }),
        Line::from(if state.focus == Focus::Menu {
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

fn field_count(section: usize, _config: &AppConfig) -> usize {
    match section {
        0 => 1,
        1 => 2,
        2 => 1,
        3 => 3,
        4 => 7,
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
                catalog.t("setup.field.annotate.file_rules"),
                file_rules_to_text(&config.annotate.file_rules)
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.policy_add"),
                config.annotate.policies.add
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.policy_modify"),
                config.annotate.policies.modify
            ),
            format!(
                "{}: {}",
                catalog.t("setup.field.annotate.policy_del"),
                config.annotate.policies.del
            ),
        ],
        _ => vec![],
    }
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
        (4, 3) => Some(file_rules_to_text(&config.annotate.file_rules)),
        (4, 4) => Some(config.annotate.policies.add.clone()),
        (4, 5) => Some(config.annotate.policies.modify.clone()),
        (4, 6) => Some(config.annotate.policies.del.clone()),
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
        (4, 3) => {
            config.annotate.file_rules = parse_file_rules(&value);
        }
        (4, 4) => config.annotate.policies.add = value,
        (4, 5) => config.annotate.policies.modify = value,
        (4, 6) => config.annotate.policies.del = value,
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

fn file_rules_to_text(rules: &[FileRuleConfig]) -> String {
    rules
        .iter()
        .map(|rule| format!("{}={}", rule.pattern, rule.renderer))
        .collect::<Vec<String>>()
        .join(";")
}

fn parse_file_rules(value: &str) -> Vec<FileRuleConfig> {
    let mut out = Vec::new();
    for item in value.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        if let Some((pattern, renderer)) = item.split_once('=') {
            out.push(FileRuleConfig {
                pattern: pattern.trim().to_string(),
                renderer: renderer.trim().to_string(),
            });
        }
    }
    out
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
    use crate::config::AppConfig;
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
}
