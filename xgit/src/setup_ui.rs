use crate::code_file_types::{
    categories, category_selected_count, category_state, file_rules_from_selection,
    selection_from_file_rules, set_category_selected, total_builtin_entry_count, TriState,
};
use crate::config::{
    default_field_definition, save_config, AnnotateConfig, AnnotateFormFieldConfig,
    AnnotateFormFieldKind, AnnotateOldCodeLineLayout, AnnotateOldCodeMode, AppConfig,
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
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use std::io::{stdout, IsTerminal};
use std::path::Path;
use std::time::Duration;

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
        ensure_valid_navigation(catalog, &working, &mut state);
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum MenuId {
    Root,
    General,
    Features,
    Push,
    Identity,
    Annotate,
    AnnotateForm,
    FormFields,
    FormField(String),
    OptionSets,
    OptionSet(String),
    CodeFileTypesCategories,
    CodeFileTypesEntries(String),
    Render,
    Template(TemplateKind),
    OldCode,
    OldCodeLineLayoutPerLine,
    OldCodeLineLayoutHeaderBody,
    ChoiceUiLang,
    ChoiceOldCodeMode,
    ChoiceOldCodeLineLayout,
    ChoiceFieldKind(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TemplateKind {
    Add,
    Modify,
    Delete,
}

impl TemplateKind {
    fn title_key(self) -> &'static str {
        match self {
            Self::Add => "setup.menu.template_add",
            Self::Modify => "setup.menu.template_modify",
            Self::Delete => "setup.menu.template_delete",
        }
    }
}

#[derive(Debug, Clone)]
struct MenuFrame {
    id: MenuId,
    selected: usize,
}

impl MenuFrame {
    fn new(id: MenuId) -> Self {
        Self { id, selected: 0 }
    }
}

#[derive(Debug, Clone)]
struct EditorState {
    title: String,
    value: String,
    target: EditorTarget,
}

#[derive(Debug, Clone)]
enum EditorTarget {
    PushPlaceholder,
    IdentityAuthorTag,
    IdentityName,
    IdentityEmail,
    DateFormat,
    TemplateStart(TemplateKind),
    TemplateEnd(TemplateKind),
    OldCodeLineHeader,
    OldCodeLineBodyPrefix,
    OldCodeLineBodySuffix,
    FieldId(String),
    FieldLabel(String),
    FieldOptionSet(String),
    AddField,
    AddOptionSet,
    AddOptionValue(String),
    EditOptionValue { set_name: String, index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FeatureFlag {
    Push,
    Annotate,
    Reset,
    CheckoutRemote,
    Completion,
}

#[derive(Debug, Clone)]
enum ToggleTarget {
    Feature(FeatureFlag),
    IncludeUntracked,
    AlignWithCodeIndent,
    WrapBlankLines,
    TemplateEnabled(TemplateKind),
    OldCodeEnabled,
    FieldRequired(String),
}

#[derive(Debug, Clone)]
enum Action {
    DeleteField(String),
}

#[derive(Debug, Clone)]
enum ChoiceOption {
    UiLang(&'static str),
    OldCodeMode(Option<AnnotateOldCodeMode>),
    OldCodeLineLayout(AnnotateOldCodeLineLayout),
    FieldKind {
        field_id: String,
        kind: AnnotateFormFieldKind,
    },
}

#[derive(Debug, Clone)]
enum MenuItemKind {
    Submenu(MenuId),
    ToggleSubmenu {
        menu: MenuId,
        toggle: ToggleTarget,
        enabled: bool,
    },
    Toggle(ToggleTarget),
    Text(EditorTarget),
    Add(EditorTarget),
    Action(Action),
    ChoiceOption(ChoiceOption),
    SelectSubmenu {
        option: ChoiceOption,
        menu: MenuId,
    },
    CodeTypeCategory(&'static str),
    CodeTypeEntry {
        category_id: &'static str,
        entry_key: &'static str,
    },
}

#[derive(Debug, Clone)]
struct MenuItem {
    text: String,
    kind: MenuItemKind,
}

#[derive(Debug)]
struct SetupState {
    stack: Vec<MenuFrame>,
    editor: Option<EditorState>,
    confirm_exit: bool,
    confirm_choice: usize,
    status: String,
    dirty: bool,
}

#[derive(Debug, Clone)]
struct SetupHelpContent {
    summary: String,
    shortcuts: String,
}

impl Default for MenuFrame {
    fn default() -> Self {
        Self::new(MenuId::Root)
    }
}

impl Default for SetupState {
    fn default() -> Self {
        Self {
            stack: vec![MenuFrame::new(MenuId::Root)],
            editor: None,
            confirm_exit: false,
            confirm_choice: 0,
            status: String::new(),
            dirty: false,
        }
    }
}

fn handle_key_code(
    code: KeyCode,
    catalog: &Catalog,
    target: &Path,
    working: &mut AppConfig,
    state: &mut SetupState,
) -> Result<bool> {
    ensure_valid_navigation(catalog, working, state);

    if state.editor.is_some() {
        handle_editor_key(code, catalog, working, state);
        ensure_valid_navigation(catalog, working, state);
        return Ok(false);
    }

    if state.confirm_exit {
        return handle_exit_confirm(code, catalog, target, working, state);
    }

    match code {
        KeyCode::Char('q') => return request_exit(catalog, state),
        KeyCode::Char('s') => {
            save_config(target, working)?;
            state.dirty = false;
            state.status = catalog.tf(
                "setup.status.saved",
                &[("path", target.display().to_string())],
            );
        }
        KeyCode::Esc => {
            if state.stack.len() > 1 {
                state.stack.pop();
                state.status = catalog.t("setup.status.returned");
            } else {
                return request_exit(catalog, state);
            }
        }
        KeyCode::Up => move_selection(catalog, working, state, -1),
        KeyCode::Down => move_selection(catalog, working, state, 1),
        KeyCode::Enter => handle_enter(catalog, working, state),
        KeyCode::Char(' ') => handle_space(catalog, working, state),
        _ => {}
    }

    ensure_valid_navigation(catalog, working, state);
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

fn move_selection(catalog: &Catalog, config: &AppConfig, state: &mut SetupState, delta: isize) {
    let items = current_items(catalog, config, state);
    if items.is_empty() {
        return;
    }
    if let Some(frame) = state.stack.last_mut() {
        let len = items.len() as isize;
        let current = frame.selected as isize;
        let next = (current + delta).rem_euclid(len) as usize;
        frame.selected = next;
    }
}

fn handle_enter(catalog: &Catalog, config: &mut AppConfig, state: &mut SetupState) {
    let Some(item) = selected_item(catalog, config, state) else {
        return;
    };

    match item.kind {
        MenuItemKind::Submenu(menu) => {
            state.stack.push(MenuFrame::new(menu));
            state.status = catalog.t("setup.status.enter_submenu");
        }
        MenuItemKind::ToggleSubmenu {
            menu,
            toggle: _,
            enabled,
        } => {
            if enabled {
                state.stack.push(MenuFrame::new(menu));
                state.status = catalog.t("setup.status.enter_submenu");
            } else {
                state.status = catalog.t("setup.status.enable_switch_first");
            }
        }
        MenuItemKind::Toggle(target) => {
            toggle_target(config, target);
            config.annotate.normalize();
            state.dirty = true;
            state.status = catalog.t("setup.status.field_updated");
        }
        MenuItemKind::Text(target) => {
            if text_item_blocked(config, &target) {
                state.status = catalog.t("setup.status.enable_custom_template_first");
            } else {
                state.editor = Some(EditorState {
                    title: editor_title(catalog, &target),
                    value: editor_initial_value(config, &target),
                    target,
                });
                state.status = catalog.t("setup.status.editing_field");
            }
        }
        MenuItemKind::Add(target) => {
            state.editor = Some(EditorState {
                title: editor_title(catalog, &target),
                value: String::new(),
                target,
            });
            state.status = catalog.t("setup.status.editing_field");
        }
        MenuItemKind::Action(action) => {
            run_action(config, state, action);
            config.annotate.normalize();
            state.dirty = true;
            state.status = catalog.t("setup.status.field_updated");
        }
        MenuItemKind::ChoiceOption(option) => {
            apply_choice(config, option);
            config.annotate.normalize();
            if state.stack.len() > 1 {
                state.stack.pop();
            }
            state.dirty = true;
            state.status = catalog.t("setup.status.field_updated");
        }
        MenuItemKind::SelectSubmenu { option, menu } => {
            apply_choice(config, option);
            config.annotate.normalize();
            state.stack.push(MenuFrame::new(menu));
            state.dirty = true;
            state.status = catalog.t("setup.status.field_updated");
        }
        MenuItemKind::CodeTypeCategory(category_id) => {
            state
                .stack
                .push(MenuFrame::new(MenuId::CodeFileTypesEntries(
                    category_id.to_string(),
                )));
            state.status = catalog.t("setup.status.enter_submenu");
        }
        MenuItemKind::CodeTypeEntry {
            category_id,
            entry_key,
        } => {
            toggle_code_type_entry(config, category_id, entry_key);
            state.dirty = true;
            state.status = catalog.t("setup.status.field_updated");
        }
    }
}

fn handle_space(catalog: &Catalog, config: &mut AppConfig, state: &mut SetupState) {
    let Some(item) = selected_item(catalog, config, state) else {
        return;
    };

    match item.kind {
        MenuItemKind::ToggleSubmenu {
            menu: _,
            toggle,
            enabled: _,
        } => {
            toggle_target(config, toggle);
            config.annotate.normalize();
            state.dirty = true;
            state.status = catalog.t("setup.status.field_updated");
        }
        MenuItemKind::Toggle(target) => {
            toggle_target(config, target);
            config.annotate.normalize();
            state.dirty = true;
            state.status = catalog.t("setup.status.field_updated");
        }
        MenuItemKind::ChoiceOption(option) => {
            apply_choice(config, option);
            config.annotate.normalize();
            state.dirty = true;
            state.status = catalog.t("setup.status.field_updated");
        }
        MenuItemKind::SelectSubmenu { option, menu: _ } => {
            apply_choice(config, option);
            config.annotate.normalize();
            state.dirty = true;
            state.status = catalog.t("setup.status.field_updated");
        }
        MenuItemKind::CodeTypeCategory(category_id) => {
            toggle_code_type_category(config, category_id);
            state.dirty = true;
            state.status = catalog.t("setup.status.field_updated");
        }
        MenuItemKind::CodeTypeEntry {
            category_id,
            entry_key,
        } => {
            toggle_code_type_entry(config, category_id, entry_key);
            state.dirty = true;
            state.status = catalog.t("setup.status.field_updated");
        }
        _ => {}
    }
}

fn handle_editor_key(
    code: KeyCode,
    catalog: &Catalog,
    config: &mut AppConfig,
    state: &mut SetupState,
) {
    let Some(editor) = state.editor.as_mut() else {
        return;
    };

    match code {
        KeyCode::Enter => {
            let editor = state.editor.take().expect("editor must exist");
            if apply_editor(config, state, editor) {
                config.annotate.normalize();
                state.dirty = true;
                state.status = catalog.t("setup.status.field_updated");
            } else {
                state.status = catalog.t("setup.status.edit_canceled");
            }
        }
        KeyCode::Esc => {
            state.editor = None;
            state.status = catalog.t("setup.status.edit_canceled");
        }
        KeyCode::Backspace => {
            editor.value.pop();
        }
        KeyCode::Char(ch) => {
            editor.value.push(ch);
        }
        _ => {}
    }
}

fn draw(
    frame: &mut ratatui::Frame,
    catalog: &Catalog,
    target: &Path,
    config: &AppConfig,
    state: &SetupState,
) {
    let chunks = setup_layout(frame.area());
    let help_content = active_help(catalog, config, state);

    let header = Paragraph::new(vec![
        Line::from(catalog.tf(
            "setup.help.target",
            &[("path", target.display().to_string())],
        )),
        Line::from(catalog.tf(
            "setup.help.breadcrumb",
            &[("path", breadcrumb(catalog, config, state))],
        )),
    ])
    .block(
        Block::default()
            .title(catalog.t("setup.block.setup"))
            .borders(Borders::ALL),
    )
    .wrap(Wrap { trim: false });
    frame.render_widget(header, chunks[0]);

    let items = current_items(catalog, config, state);
    let list_items = items
        .iter()
        .map(|item| ListItem::new(Line::from(item.text.clone())))
        .collect::<Vec<_>>();
    let mut list_state = ListState::default();
    list_state.select(Some(current_selection(state)));
    let list = List::new(list_items)
        .block(
            Block::default()
                .title(menu_title(catalog, config, current_menu(state)))
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, chunks[1], &mut list_state);

    let help = Paragraph::new(vec![
        Line::from(help_content.summary),
        Line::from(help_content.shortcuts),
    ])
    .block(
        Block::default()
            .title(catalog.t("setup.block.help"))
            .borders(Borders::ALL),
    )
    .wrap(Wrap { trim: false });
    frame.render_widget(help, chunks[2]);

    let status = Paragraph::new(if state.status.is_empty() {
        catalog.t("setup.status.ready")
    } else {
        state.status.clone()
    })
    .block(
        Block::default()
            .title(catalog.t("setup.block.status"))
            .borders(Borders::ALL),
    )
    .wrap(Wrap { trim: false });
    frame.render_widget(status, chunks[3]);

    if state.confirm_exit {
        draw_exit_confirm(frame, catalog, state);
    } else if let Some(editor) = &state.editor {
        draw_editor(frame, catalog, editor);
    }
}

fn setup_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(4),
            Constraint::Length(3),
        ])
        .split(area)
        .to_vec()
}

fn draw_exit_confirm(frame: &mut ratatui::Frame, catalog: &Catalog, state: &SetupState) {
    let area = centered_rect(60, 45, frame.area());
    frame.render_widget(Clear, area);

    let options = EXIT_OPTIONS
        .iter()
        .map(|key| ListItem::new(Line::from(catalog.t(key))))
        .collect::<Vec<_>>();
    let mut list_state = ListState::default();
    list_state.select(Some(state.confirm_choice));
    let list = List::new(options)
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
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_editor(frame: &mut ratatui::Frame, catalog: &Catalog, editor: &EditorState) {
    let area = centered_rect(72, 34, frame.area());
    frame.render_widget(Clear, area);
    let body = Paragraph::new(vec![
        Line::from(editor.value.clone()),
        Line::from(""),
        Line::from(catalog.t("setup.help.editor_actions")),
    ])
    .block(
        Block::default()
            .title(editor.title.clone())
            .borders(Borders::ALL),
    )
    .wrap(Wrap { trim: false });
    frame.render_widget(body, area);
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

fn ensure_valid_navigation(catalog: &Catalog, config: &AppConfig, state: &mut SetupState) {
    while !state.stack.is_empty() {
        let current_id = state.stack.last().map(|frame| frame.id.clone());
        if state.stack.len() > 1
            && current_id
                .as_ref()
                .is_some_and(|menu| !menu_accessible(config, menu))
        {
            state.stack.pop();
            continue;
        }
        let len = current_items(catalog, config, state).len();
        if len == 0 && state.stack.len() > 1 {
            state.stack.pop();
            continue;
        }
        if let Some(frame) = state.stack.last_mut() {
            frame.selected = frame.selected.min(len.saturating_sub(1));
        }
        break;
    }
    if state.stack.is_empty() {
        state.stack.push(MenuFrame::new(MenuId::Root));
    }
}

fn current_menu(state: &SetupState) -> &MenuId {
    &state
        .stack
        .last()
        .expect("setup navigation stack must not be empty")
        .id
}

fn menu_accessible(config: &AppConfig, menu: &MenuId) -> bool {
    match menu {
        MenuId::Template(kind) => template_ref(config, *kind).enabled,
        MenuId::OldCode
        | MenuId::ChoiceOldCodeMode
        | MenuId::ChoiceOldCodeLineLayout
        | MenuId::OldCodeLineLayoutPerLine
        | MenuId::OldCodeLineLayoutHeaderBody => old_code_processing_enabled(config),
        _ => true,
    }
}

fn current_selection(state: &SetupState) -> usize {
    state.stack.last().map(|frame| frame.selected).unwrap_or(0)
}

fn selected_item(catalog: &Catalog, config: &AppConfig, state: &SetupState) -> Option<MenuItem> {
    let items = current_items(catalog, config, state);
    items.get(current_selection(state)).cloned()
}

fn current_items(catalog: &Catalog, config: &AppConfig, state: &SetupState) -> Vec<MenuItem> {
    match current_menu(state) {
        MenuId::Root => vec![
            submenu_item(catalog.t("setup.menu.general"), MenuId::General),
            submenu_item(catalog.t("setup.menu.identity"), MenuId::Identity),
            submenu_item(catalog.t("setup.menu.annotate"), MenuId::Annotate),
        ],
        MenuId::General => vec![
            submenu_with_summary(
                catalog.t("setup.menu.ui"),
                language_value(catalog, config.ui.lang.as_str()),
                MenuId::ChoiceUiLang,
            ),
            submenu_with_summary(
                catalog.t("setup.menu.features"),
                feature_summary(catalog, config),
                MenuId::Features,
            ),
            submenu_with_summary(
                catalog.t("setup.menu.push"),
                config.push.placeholder.clone().unwrap_or_default(),
                MenuId::Push,
            ),
        ],
        MenuId::Features => vec![
            toggle_item(
                catalog.t("setup.field.features.push"),
                config.features.push,
                ToggleTarget::Feature(FeatureFlag::Push),
            ),
            toggle_item(
                catalog.t("setup.field.features.annotate"),
                config.features.annotate,
                ToggleTarget::Feature(FeatureFlag::Annotate),
            ),
            toggle_item(
                catalog.t("setup.field.features.reset"),
                config.features.reset,
                ToggleTarget::Feature(FeatureFlag::Reset),
            ),
            toggle_item(
                catalog.t("setup.field.features.checkout_remote"),
                config.features.checkout_remote,
                ToggleTarget::Feature(FeatureFlag::CheckoutRemote),
            ),
            toggle_item(
                catalog.t("setup.field.features.completion"),
                config.features.completion,
                ToggleTarget::Feature(FeatureFlag::Completion),
            ),
        ],
        MenuId::Push => vec![text_item(
            catalog.t("setup.field.push.placeholder"),
            config.push.placeholder.clone().unwrap_or_default(),
            EditorTarget::PushPlaceholder,
        )],
        MenuId::Identity => vec![
            text_item(
                catalog.t("setup.field.identity.author_tag"),
                config.identity.author_tag.clone().unwrap_or_default(),
                EditorTarget::IdentityAuthorTag,
            ),
            text_item(
                catalog.t("setup.field.identity.name"),
                config.identity.name.clone().unwrap_or_default(),
                EditorTarget::IdentityName,
            ),
            text_item(
                catalog.t("setup.field.identity.email"),
                config.identity.email.clone().unwrap_or_default(),
                EditorTarget::IdentityEmail,
            ),
        ],
        MenuId::Annotate => vec![
            submenu_with_summary(
                catalog.t("setup.menu.annotate_form"),
                form_summary(catalog, config),
                MenuId::AnnotateForm,
            ),
            submenu_with_summary(
                catalog.t("setup.menu.code_file_types"),
                code_file_type_summary(catalog, config),
                MenuId::CodeFileTypesCategories,
            ),
            submenu_with_summary(
                catalog.t("setup.menu.render"),
                render_summary(catalog, config),
                MenuId::Render,
            ),
            toggle_submenu_item(
                catalog.t("setup.menu.template_add"),
                config.annotate.block_templates.add.enabled,
                MenuId::Template(TemplateKind::Add),
                ToggleTarget::TemplateEnabled(TemplateKind::Add),
            ),
            toggle_submenu_item(
                catalog.t("setup.menu.template_modify"),
                config.annotate.block_templates.modify.enabled,
                MenuId::Template(TemplateKind::Modify),
                ToggleTarget::TemplateEnabled(TemplateKind::Modify),
            ),
            toggle_submenu_item(
                catalog.t("setup.menu.template_delete"),
                config.annotate.block_templates.del.enabled,
                MenuId::Template(TemplateKind::Delete),
                ToggleTarget::TemplateEnabled(TemplateKind::Delete),
            ),
            toggle_submenu_item(
                catalog.t("setup.menu.old_code"),
                old_code_processing_enabled(config),
                MenuId::OldCode,
                ToggleTarget::OldCodeEnabled,
            ),
        ],
        MenuId::AnnotateForm => vec![
            submenu_with_summary(
                catalog.t("setup.menu.form_fields"),
                catalog.tf(
                    "setup.value.form_field_count",
                    &[("count", config.annotate.form.fields.len().to_string())],
                ),
                MenuId::FormFields,
            ),
            submenu_with_summary(
                catalog.t("setup.menu.option_sets"),
                catalog.tf(
                    "setup.value.option_set_count",
                    &[("count", config.annotate.form.option_sets.len().to_string())],
                ),
                MenuId::OptionSets,
            ),
        ],
        MenuId::FormFields => {
            let mut items = config
                .annotate
                .form
                .fields
                .iter()
                .map(|field| {
                    submenu_with_summary(
                        field_display_label(field),
                        field_summary(catalog, field),
                        MenuId::FormField(field.id.clone()),
                    )
                })
                .collect::<Vec<_>>();
            items.push(add_item(
                catalog.t("setup.action.add_field"),
                EditorTarget::AddField,
            ));
            items
        }
        MenuId::FormField(field_id) => {
            let Some(field) = find_field(config, field_id.as_str()) else {
                return Vec::new();
            };
            let mut items = vec![
                text_item(
                    catalog.t("setup.field.annotate.field_id"),
                    field.id.clone(),
                    EditorTarget::FieldId(field.id.clone()),
                ),
                text_item(
                    catalog.t("setup.field.annotate.field_label"),
                    field.label.clone(),
                    EditorTarget::FieldLabel(field.id.clone()),
                ),
                submenu_with_summary(
                    catalog.t("setup.field.annotate.field_kind"),
                    field_kind_text(catalog, &field.kind),
                    MenuId::ChoiceFieldKind(field.id.clone()),
                ),
                toggle_item(
                    catalog.t("setup.field.annotate.field_required"),
                    field.required,
                    ToggleTarget::FieldRequired(field.id.clone()),
                ),
            ];
            if field.kind == AnnotateFormFieldKind::SingleSelect {
                items.push(text_item(
                    catalog.t("setup.field.annotate.field_option_set"),
                    field.option_set.clone().unwrap_or_default(),
                    EditorTarget::FieldOptionSet(field.id.clone()),
                ));
            }
            items.push(action_item(
                catalog.t("setup.action.delete_field"),
                Action::DeleteField(field.id.clone()),
            ));
            items
        }
        MenuId::OptionSets => {
            let mut items = config
                .annotate
                .form
                .option_sets
                .iter()
                .map(|(name, set)| {
                    submenu_with_summary(
                        name.clone(),
                        catalog.tf(
                            "setup.value.option_value_count",
                            &[("count", set.values.len().to_string())],
                        ),
                        MenuId::OptionSet(name.clone()),
                    )
                })
                .collect::<Vec<_>>();
            items.push(add_item(
                catalog.t("setup.action.add_option_set"),
                EditorTarget::AddOptionSet,
            ));
            items
        }
        MenuId::OptionSet(name) => {
            let Some(option_set) = config.annotate.form.option_sets.get(name) else {
                return Vec::new();
            };
            let mut items = option_set
                .values
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    text_item(
                        catalog.tf(
                            "setup.field.annotate.option_value",
                            &[("index", (index + 1).to_string())],
                        ),
                        value.clone(),
                        EditorTarget::EditOptionValue {
                            set_name: name.clone(),
                            index,
                        },
                    )
                })
                .collect::<Vec<_>>();
            items.push(add_item(
                catalog.t("setup.action.add_option_value"),
                EditorTarget::AddOptionValue(name.clone()),
            ));
            items
        }
        MenuId::CodeFileTypesCategories => categories()
            .iter()
            .map(|category| {
                let marker = tri_state_marker(category_state(
                    category,
                    &selection_from_file_rules(&config.annotate.file_rules).selected_keys,
                ));
                MenuItem {
                    text: format!(
                        "{marker} {} ({}/{})",
                        localized_label(catalog, category.label_key, category.default_label),
                        category_selected_count(
                            category,
                            &selection_from_file_rules(&config.annotate.file_rules).selected_keys
                        ),
                        category.entries.len()
                    ),
                    kind: MenuItemKind::CodeTypeCategory(category.id),
                }
            })
            .collect(),
        MenuId::CodeFileTypesEntries(category_id) => {
            let Some(category) = categories().iter().find(|item| item.id == category_id) else {
                return Vec::new();
            };
            let selection = selection_from_file_rules(&config.annotate.file_rules);
            category
                .entries
                .iter()
                .map(|entry| {
                    let selected = selection.selected_keys.contains(entry.key);
                    MenuItem {
                        text: format!(
                            "{} {} ({})",
                            if selected { "[x]" } else { "[ ]" },
                            localized_label(catalog, entry.label_key, entry.default_label),
                            entry.pattern
                        ),
                        kind: MenuItemKind::CodeTypeEntry {
                            category_id: category.id,
                            entry_key: entry.key,
                        },
                    }
                })
                .collect()
        }
        MenuId::Render => vec![
            toggle_item(
                catalog.t("setup.field.annotate.include_untracked"),
                config.annotate.staged.include_untracked,
                ToggleTarget::IncludeUntracked,
            ),
            toggle_item(
                catalog.t("setup.field.annotate.align_with_code_indent"),
                config.annotate.render.align_with_code_indent,
                ToggleTarget::AlignWithCodeIndent,
            ),
            toggle_item(
                catalog.t("setup.field.annotate.wrap_blank_lines"),
                config.annotate.render.wrap_blank_lines,
                ToggleTarget::WrapBlankLines,
            ),
            text_item(
                catalog.t("setup.field.annotate.date_format"),
                config.annotate.date.format.clone(),
                EditorTarget::DateFormat,
            ),
        ],
        MenuId::Template(kind) => {
            let template = template_ref(config, *kind);
            vec![
                text_item(
                    catalog.t("setup.field.annotate.template_start"),
                    template.start.clone(),
                    EditorTarget::TemplateStart(*kind),
                ),
                text_item(
                    catalog.t("setup.field.annotate.template_end"),
                    template.end.clone(),
                    EditorTarget::TemplateEnd(*kind),
                ),
            ]
        }
        MenuId::OldCode => vec![
            submenu_with_summary(
                catalog.t("setup.field.annotate.old_code_mode"),
                old_code_mode_text(catalog, config.annotate.old_code.mode.as_ref()),
                MenuId::ChoiceOldCodeMode,
            ),
            submenu_with_summary(
                catalog.t("setup.field.annotate.old_code_line_layout"),
                old_code_line_layout_text(
                    catalog,
                    config.annotate.old_code.line_comment.layout.clone(),
                ),
                MenuId::ChoiceOldCodeLineLayout,
            ),
        ],
        MenuId::OldCodeLineLayoutPerLine => vec![
            text_item(
                catalog.t("setup.field.annotate.old_code_line_body_prefix"),
                config.annotate.old_code.line_comment.body_prefix.clone(),
                EditorTarget::OldCodeLineBodyPrefix,
            ),
            text_item(
                catalog.t("setup.field.annotate.old_code_line_body_suffix"),
                config.annotate.old_code.line_comment.body_suffix.clone(),
                EditorTarget::OldCodeLineBodySuffix,
            ),
        ],
        MenuId::OldCodeLineLayoutHeaderBody => vec![
            text_item(
                catalog.t("setup.field.annotate.old_code_line_header"),
                config.annotate.old_code.line_comment.header.clone(),
                EditorTarget::OldCodeLineHeader,
            ),
            text_item(
                catalog.t("setup.field.annotate.old_code_line_body_prefix"),
                config.annotate.old_code.line_comment.body_prefix.clone(),
                EditorTarget::OldCodeLineBodyPrefix,
            ),
            text_item(
                catalog.t("setup.field.annotate.old_code_line_body_suffix"),
                config.annotate.old_code.line_comment.body_suffix.clone(),
                EditorTarget::OldCodeLineBodySuffix,
            ),
        ],
        MenuId::ChoiceUiLang => vec![
            choice_item(
                language_value(catalog, "zh-CN"),
                config.ui.lang == "zh-CN",
                ChoiceOption::UiLang("zh-CN"),
            ),
            choice_item(
                language_value(catalog, "en-US"),
                config.ui.lang == "en-US",
                ChoiceOption::UiLang("en-US"),
            ),
        ],
        MenuId::ChoiceOldCodeMode => vec![
            choice_item(
                old_code_mode_text(catalog, None),
                config.annotate.old_code.mode.is_none(),
                ChoiceOption::OldCodeMode(None),
            ),
            choice_item(
                old_code_mode_text(catalog, Some(&AnnotateOldCodeMode::LineComment)),
                config.annotate.old_code.mode == Some(AnnotateOldCodeMode::LineComment),
                ChoiceOption::OldCodeMode(Some(AnnotateOldCodeMode::LineComment)),
            ),
            choice_item(
                old_code_mode_text(catalog, Some(&AnnotateOldCodeMode::BlockComment)),
                config.annotate.old_code.mode == Some(AnnotateOldCodeMode::BlockComment),
                ChoiceOption::OldCodeMode(Some(AnnotateOldCodeMode::BlockComment)),
            ),
        ],
        MenuId::ChoiceOldCodeLineLayout => vec![
            select_submenu_item(
                old_code_line_layout_text(catalog, AnnotateOldCodeLineLayout::PerLine),
                config.annotate.old_code.line_comment.layout == AnnotateOldCodeLineLayout::PerLine,
                ChoiceOption::OldCodeLineLayout(AnnotateOldCodeLineLayout::PerLine),
                MenuId::OldCodeLineLayoutPerLine,
            ),
            select_submenu_item(
                old_code_line_layout_text(catalog, AnnotateOldCodeLineLayout::HeaderBody),
                config.annotate.old_code.line_comment.layout
                    == AnnotateOldCodeLineLayout::HeaderBody,
                ChoiceOption::OldCodeLineLayout(AnnotateOldCodeLineLayout::HeaderBody),
                MenuId::OldCodeLineLayoutHeaderBody,
            ),
        ],
        MenuId::ChoiceFieldKind(field_id) => {
            let Some(field) = find_field(config, field_id.as_str()) else {
                return Vec::new();
            };
            vec![
                choice_item(
                    field_kind_text(catalog, &AnnotateFormFieldKind::Text),
                    field.kind == AnnotateFormFieldKind::Text,
                    ChoiceOption::FieldKind {
                        field_id: field.id.clone(),
                        kind: AnnotateFormFieldKind::Text,
                    },
                ),
                choice_item(
                    field_kind_text(catalog, &AnnotateFormFieldKind::SingleSelect),
                    field.kind == AnnotateFormFieldKind::SingleSelect,
                    ChoiceOption::FieldKind {
                        field_id: field.id.clone(),
                        kind: AnnotateFormFieldKind::SingleSelect,
                    },
                ),
            ]
        }
    }
}

fn submenu_item(label: impl Into<String>, menu: MenuId) -> MenuItem {
    MenuItem {
        text: format!("--> {}", label.into()),
        kind: MenuItemKind::Submenu(menu),
    }
}

fn submenu_with_summary(
    label: impl Into<String>,
    summary: impl Into<String>,
    menu: MenuId,
) -> MenuItem {
    MenuItem {
        text: format!("--> {}: {}", label.into(), summary.into()),
        kind: MenuItemKind::Submenu(menu),
    }
}

fn toggle_submenu_item(
    label: impl Into<String>,
    enabled: bool,
    menu: MenuId,
    toggle: ToggleTarget,
) -> MenuItem {
    MenuItem {
        text: format!(
            "--> {} {}",
            label.into(),
            if enabled { "[x]" } else { "[ ]" }
        ),
        kind: MenuItemKind::ToggleSubmenu {
            menu,
            toggle,
            enabled,
        },
    }
}

fn text_item(label: impl Into<String>, value: impl Into<String>, target: EditorTarget) -> MenuItem {
    MenuItem {
        text: format!("--> {}: {}", label.into(), value.into()),
        kind: MenuItemKind::Text(target),
    }
}

fn toggle_item(label: impl Into<String>, value: bool, target: ToggleTarget) -> MenuItem {
    MenuItem {
        text: format!("{} {}", if value { "[x]" } else { "[ ]" }, label.into()),
        kind: MenuItemKind::Toggle(target),
    }
}

fn add_item(label: impl Into<String>, target: EditorTarget) -> MenuItem {
    MenuItem {
        text: format!("[+] {}", label.into()),
        kind: MenuItemKind::Add(target),
    }
}

fn action_item(label: impl Into<String>, action: Action) -> MenuItem {
    MenuItem {
        text: format!("[!] {}", label.into()),
        kind: MenuItemKind::Action(action),
    }
}

fn choice_item(label: impl Into<String>, selected: bool, option: ChoiceOption) -> MenuItem {
    MenuItem {
        text: format!("{} {}", if selected { "(*)" } else { "( )" }, label.into()),
        kind: MenuItemKind::ChoiceOption(option),
    }
}

fn select_submenu_item(
    label: impl Into<String>,
    selected: bool,
    option: ChoiceOption,
    menu: MenuId,
) -> MenuItem {
    MenuItem {
        text: format!(
            "{} {} -->",
            if selected { "[x]" } else { "[ ]" },
            label.into()
        ),
        kind: MenuItemKind::SelectSubmenu { option, menu },
    }
}

fn toggle_target(config: &mut AppConfig, target: ToggleTarget) {
    fn toggle_enabled_preserving_value(enabled: &mut bool) {
        *enabled = !*enabled;
    }

    match target {
        ToggleTarget::Feature(flag) => match flag {
            FeatureFlag::Push => config.features.push = !config.features.push,
            FeatureFlag::Annotate => config.features.annotate = !config.features.annotate,
            FeatureFlag::Reset => config.features.reset = !config.features.reset,
            FeatureFlag::CheckoutRemote => {
                config.features.checkout_remote = !config.features.checkout_remote
            }
            FeatureFlag::Completion => config.features.completion = !config.features.completion,
        },
        ToggleTarget::IncludeUntracked => {
            config.annotate.staged.include_untracked = !config.annotate.staged.include_untracked;
        }
        ToggleTarget::AlignWithCodeIndent => {
            config.annotate.render.align_with_code_indent =
                !config.annotate.render.align_with_code_indent;
        }
        ToggleTarget::WrapBlankLines => {
            config.annotate.render.wrap_blank_lines = !config.annotate.render.wrap_blank_lines;
        }
        ToggleTarget::TemplateEnabled(kind) => {
            let template = template_mut(config, kind);
            toggle_enabled_preserving_value(&mut template.enabled);
        }
        ToggleTarget::OldCodeEnabled => {
            toggle_enabled_preserving_value(&mut config.annotate.old_code.enabled);
        }
        ToggleTarget::FieldRequired(field_id) => {
            if let Some(field) = find_field_mut(config, field_id.as_str()) {
                field.required = !field.required;
            }
        }
    }
}

fn run_action(config: &mut AppConfig, state: &mut SetupState, action: Action) {
    match action {
        Action::DeleteField(field_id) => {
            config
                .annotate
                .form
                .fields
                .retain(|field| field.id != field_id);
            if matches!(current_menu(state), MenuId::FormField(_)) && state.stack.len() > 1 {
                state.stack.pop();
            }
        }
    }
}

fn apply_choice(config: &mut AppConfig, option: ChoiceOption) {
    match option {
        ChoiceOption::UiLang(lang) => {
            config.ui.lang = lang.to_string();
        }
        ChoiceOption::OldCodeMode(mode) => {
            config.annotate.old_code.mode = mode;
        }
        ChoiceOption::OldCodeLineLayout(layout) => {
            config.annotate.old_code.line_comment.layout = layout;
        }
        ChoiceOption::FieldKind { field_id, kind } => {
            if let Some(field) = find_field_mut(config, field_id.as_str()) {
                field.kind = kind.clone();
                if kind == AnnotateFormFieldKind::Text {
                    field.option_set = None;
                } else if field.option_set.is_none() {
                    field.option_set =
                        Some(AnnotateConfig::reference_kind_option_set_name().to_string());
                }
            }
        }
    }
}

fn apply_editor(config: &mut AppConfig, state: &mut SetupState, editor: EditorState) -> bool {
    let value = editor.value.trim().to_string();
    match editor.target {
        EditorTarget::PushPlaceholder => {
            config.push.placeholder = empty_to_none(value.as_str());
            true
        }
        EditorTarget::IdentityAuthorTag => {
            config.identity.author_tag = empty_to_none(value.as_str());
            true
        }
        EditorTarget::IdentityName => {
            config.identity.name = empty_to_none(value.as_str());
            true
        }
        EditorTarget::IdentityEmail => {
            config.identity.email = empty_to_none(value.as_str());
            true
        }
        EditorTarget::DateFormat => {
            config.annotate.date.format = value;
            true
        }
        EditorTarget::TemplateStart(kind) => {
            template_mut(config, kind).start = editor.value;
            true
        }
        EditorTarget::TemplateEnd(kind) => {
            template_mut(config, kind).end = editor.value;
            true
        }
        EditorTarget::OldCodeLineHeader => {
            config.annotate.old_code.line_comment.header = editor.value;
            true
        }
        EditorTarget::OldCodeLineBodyPrefix => {
            config.annotate.old_code.line_comment.body_prefix = editor.value;
            true
        }
        EditorTarget::OldCodeLineBodySuffix => {
            config.annotate.old_code.line_comment.body_suffix = editor.value;
            true
        }
        EditorTarget::FieldId(old_id) => {
            if value.is_empty() {
                return false;
            }
            if old_id != value && find_field(config, value.as_str()).is_some() {
                state.status = "Field id already exists".to_string();
                return false;
            }
            if let Some(field) = find_field_mut(config, old_id.as_str()) {
                field.id = value.clone();
                rewrite_menu_ids(state, old_id.as_str(), value.as_str());
                true
            } else {
                false
            }
        }
        EditorTarget::FieldLabel(field_id) => {
            if let Some(field) = find_field_mut(config, field_id.as_str()) {
                field.label = editor.value;
                true
            } else {
                false
            }
        }
        EditorTarget::FieldOptionSet(field_id) => {
            if let Some(field) = find_field_mut(config, field_id.as_str()) {
                field.option_set = empty_to_none(value.as_str());
                if let Some(option_set) = field.option_set.clone() {
                    config
                        .annotate
                        .form
                        .option_sets
                        .entry(option_set)
                        .or_default();
                }
                true
            } else {
                false
            }
        }
        EditorTarget::AddField => {
            if value.is_empty() || find_field(config, value.as_str()).is_some() {
                return false;
            }
            config
                .annotate
                .form
                .fields
                .push(default_field_definition(value.as_str()));
            state
                .stack
                .push(MenuFrame::new(MenuId::FormField(value.to_string())));
            true
        }
        EditorTarget::AddOptionSet => {
            if value.is_empty()
                || config
                    .annotate
                    .form
                    .option_sets
                    .contains_key(value.as_str())
            {
                return false;
            }
            config
                .annotate
                .form
                .option_sets
                .insert(value.clone(), Default::default());
            state
                .stack
                .push(MenuFrame::new(MenuId::OptionSet(value.to_string())));
            true
        }
        EditorTarget::AddOptionValue(set_name) => {
            if value.is_empty() {
                return false;
            }
            if let Some(option_set) = config.annotate.form.option_sets.get_mut(set_name.as_str()) {
                option_set.values.push(value);
                true
            } else {
                false
            }
        }
        EditorTarget::EditOptionValue { set_name, index } => {
            if let Some(option_set) = config.annotate.form.option_sets.get_mut(set_name.as_str()) {
                if let Some(slot) = option_set.values.get_mut(index) {
                    *slot = value;
                    return true;
                }
            }
            false
        }
    }
}

fn rewrite_menu_ids(state: &mut SetupState, old_id: &str, new_id: &str) {
    for frame in &mut state.stack {
        frame.id = match &frame.id {
            MenuId::FormField(field_id) if field_id == old_id => {
                MenuId::FormField(new_id.to_string())
            }
            MenuId::ChoiceFieldKind(field_id) if field_id == old_id => {
                MenuId::ChoiceFieldKind(new_id.to_string())
            }
            other => other.clone(),
        };
    }
}

fn editor_initial_value(config: &AppConfig, target: &EditorTarget) -> String {
    match target {
        EditorTarget::PushPlaceholder => config.push.placeholder.clone().unwrap_or_default(),
        EditorTarget::IdentityAuthorTag => config.identity.author_tag.clone().unwrap_or_default(),
        EditorTarget::IdentityName => config.identity.name.clone().unwrap_or_default(),
        EditorTarget::IdentityEmail => config.identity.email.clone().unwrap_or_default(),
        EditorTarget::DateFormat => config.annotate.date.format.clone(),
        EditorTarget::TemplateStart(kind) => template_ref(config, *kind).start.clone(),
        EditorTarget::TemplateEnd(kind) => template_ref(config, *kind).end.clone(),
        EditorTarget::OldCodeLineHeader => config.annotate.old_code.line_comment.header.clone(),
        EditorTarget::OldCodeLineBodyPrefix => {
            config.annotate.old_code.line_comment.body_prefix.clone()
        }
        EditorTarget::OldCodeLineBodySuffix => {
            config.annotate.old_code.line_comment.body_suffix.clone()
        }
        EditorTarget::FieldId(field_id) => find_field(config, field_id.as_str())
            .map(|field| field.id.clone())
            .unwrap_or_default(),
        EditorTarget::FieldLabel(field_id) => find_field(config, field_id.as_str())
            .map(|field| field.label.clone())
            .unwrap_or_default(),
        EditorTarget::FieldOptionSet(field_id) => find_field(config, field_id.as_str())
            .and_then(|field| field.option_set.clone())
            .unwrap_or_default(),
        EditorTarget::AddField | EditorTarget::AddOptionSet | EditorTarget::AddOptionValue(_) => {
            String::new()
        }
        EditorTarget::EditOptionValue { set_name, index } => config
            .annotate
            .form
            .option_sets
            .get(set_name.as_str())
            .and_then(|set| set.values.get(*index))
            .cloned()
            .unwrap_or_default(),
    }
}

fn editor_title(catalog: &Catalog, target: &EditorTarget) -> String {
    match target {
        EditorTarget::PushPlaceholder => catalog.t("setup.field.push.placeholder"),
        EditorTarget::IdentityAuthorTag => catalog.t("setup.field.identity.author_tag"),
        EditorTarget::IdentityName => catalog.t("setup.field.identity.name"),
        EditorTarget::IdentityEmail => catalog.t("setup.field.identity.email"),
        EditorTarget::DateFormat => catalog.t("setup.field.annotate.date_format"),
        EditorTarget::TemplateStart(kind) => {
            format!(
                "{} / {}",
                catalog.t(kind.title_key()),
                catalog.t("setup.field.annotate.template_start")
            )
        }
        EditorTarget::TemplateEnd(kind) => {
            format!(
                "{} / {}",
                catalog.t(kind.title_key()),
                catalog.t("setup.field.annotate.template_end")
            )
        }
        EditorTarget::OldCodeLineHeader => catalog.t("setup.field.annotate.old_code_line_header"),
        EditorTarget::OldCodeLineBodyPrefix => {
            catalog.t("setup.field.annotate.old_code_line_body_prefix")
        }
        EditorTarget::OldCodeLineBodySuffix => {
            catalog.t("setup.field.annotate.old_code_line_body_suffix")
        }
        EditorTarget::FieldId(_) => catalog.t("setup.field.annotate.field_id"),
        EditorTarget::FieldLabel(_) => catalog.t("setup.field.annotate.field_label"),
        EditorTarget::FieldOptionSet(_) => catalog.t("setup.field.annotate.field_option_set"),
        EditorTarget::AddField => catalog.t("setup.action.add_field"),
        EditorTarget::AddOptionSet => catalog.t("setup.action.add_option_set"),
        EditorTarget::AddOptionValue(_) => catalog.t("setup.action.add_option_value"),
        EditorTarget::EditOptionValue { .. } => catalog.t("setup.field.annotate.option_value"),
    }
}

fn text_item_blocked(config: &AppConfig, target: &EditorTarget) -> bool {
    match target {
        EditorTarget::TemplateStart(kind) | EditorTarget::TemplateEnd(kind) => {
            !template_ref(config, *kind).enabled
        }
        _ => false,
    }
}

fn template_ref(config: &AppConfig, kind: TemplateKind) -> &crate::config::BlockTemplate {
    match kind {
        TemplateKind::Add => &config.annotate.block_templates.add,
        TemplateKind::Modify => &config.annotate.block_templates.modify,
        TemplateKind::Delete => &config.annotate.block_templates.del,
    }
}

fn template_mut(config: &mut AppConfig, kind: TemplateKind) -> &mut crate::config::BlockTemplate {
    match kind {
        TemplateKind::Add => &mut config.annotate.block_templates.add,
        TemplateKind::Modify => &mut config.annotate.block_templates.modify,
        TemplateKind::Delete => &mut config.annotate.block_templates.del,
    }
}

fn empty_to_none(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn find_field<'a>(config: &'a AppConfig, field_id: &str) -> Option<&'a AnnotateFormFieldConfig> {
    config
        .annotate
        .form
        .fields
        .iter()
        .find(|field| field.id == field_id)
}

fn find_field_mut<'a>(
    config: &'a mut AppConfig,
    field_id: &str,
) -> Option<&'a mut AnnotateFormFieldConfig> {
    config
        .annotate
        .form
        .fields
        .iter_mut()
        .find(|field| field.id == field_id)
}

fn field_display_label(field: &AnnotateFormFieldConfig) -> String {
    if field.label.trim().is_empty() {
        field.id.clone()
    } else {
        field.label.clone()
    }
}

fn field_summary(catalog: &Catalog, field: &AnnotateFormFieldConfig) -> String {
    let required = if field.required {
        catalog.t("setup.value.required")
    } else {
        catalog.t("setup.value.optional")
    };
    if field.kind == AnnotateFormFieldKind::SingleSelect {
        let option_set = field.option_set.clone().unwrap_or_default();
        catalog.tf(
            "setup.value.field_summary_single_select",
            &[("required", required), ("option_set", option_set)],
        )
    } else {
        catalog.tf("setup.value.field_summary_text", &[("required", required)])
    }
}

fn field_kind_text(catalog: &Catalog, kind: &AnnotateFormFieldKind) -> String {
    match kind {
        AnnotateFormFieldKind::Text => catalog.t("setup.value.field_kind.text"),
        AnnotateFormFieldKind::SingleSelect => catalog.t("setup.value.field_kind.single_select"),
    }
}

fn feature_summary(catalog: &Catalog, config: &AppConfig) -> String {
    let enabled = [
        config.features.push,
        config.features.annotate,
        config.features.reset,
        config.features.checkout_remote,
        config.features.completion,
    ]
    .into_iter()
    .filter(|value| *value)
    .count();
    catalog.tf(
        "setup.value.feature_summary",
        &[("enabled", enabled.to_string()), ("total", "5".to_string())],
    )
}

fn form_summary(catalog: &Catalog, config: &AppConfig) -> String {
    catalog.tf(
        "setup.value.form_summary",
        &[
            ("fields", config.annotate.form.fields.len().to_string()),
            ("sets", config.annotate.form.option_sets.len().to_string()),
        ],
    )
}

fn render_summary(catalog: &Catalog, config: &AppConfig) -> String {
    catalog.tf(
        "setup.value.render_summary",
        &[
            (
                "align",
                bool_text(catalog, config.annotate.render.align_with_code_indent),
            ),
            (
                "wrap",
                bool_text(catalog, config.annotate.render.wrap_blank_lines),
            ),
        ],
    )
}

fn code_file_type_summary(catalog: &Catalog, config: &AppConfig) -> String {
    let selection = selection_from_file_rules(&config.annotate.file_rules);
    catalog.tf(
        "setup.value.code_file_types_summary",
        &[
            ("selected", selection.selected_keys.len().to_string()),
            ("total", total_builtin_entry_count().to_string()),
        ],
    )
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
        Some(AnnotateOldCodeMode::None) => catalog.t("setup.value.disabled"),
        Some(AnnotateOldCodeMode::LineComment) => {
            catalog.t("setup.value.annotate.old_code_mode.line_comment")
        }
        Some(AnnotateOldCodeMode::BlockComment) => {
            catalog.t("setup.value.annotate.old_code_mode.block_comment")
        }
    }
}

fn old_code_processing_enabled(config: &AppConfig) -> bool {
    config.annotate.old_code.enabled
        && config.annotate.old_code.mode != Some(AnnotateOldCodeMode::None)
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

fn toggle_code_type_category(config: &mut AppConfig, category_id: &str) {
    let selection = selection_from_file_rules(&config.annotate.file_rules);
    let mut selected_keys = selection.selected_keys;
    if let Some(category) = categories().iter().find(|item| item.id == category_id) {
        let next_selected = !matches!(category_state(category, &selected_keys), TriState::All);
        set_category_selected(category, &mut selected_keys, next_selected);
        config.annotate.file_rules =
            file_rules_from_selection(&selected_keys, &selection.passthrough_rules);
    }
}

fn toggle_code_type_entry(config: &mut AppConfig, category_id: &str, entry_key: &str) {
    let selection = selection_from_file_rules(&config.annotate.file_rules);
    let mut selected_keys = selection.selected_keys;
    if let Some(category) = categories().iter().find(|item| item.id == category_id) {
        if category.entries.iter().any(|entry| entry.key == entry_key) {
            if selected_keys.contains(entry_key) {
                selected_keys.remove(entry_key);
            } else {
                selected_keys.insert(entry_key.to_string());
            }
            config.annotate.file_rules =
                file_rules_from_selection(&selected_keys, &selection.passthrough_rules);
        }
    }
}

fn menu_title(catalog: &Catalog, config: &AppConfig, menu: &MenuId) -> String {
    match menu {
        MenuId::Root => catalog.t("setup.menu.root"),
        MenuId::General => catalog.t("setup.menu.general"),
        MenuId::Features => catalog.t("setup.menu.features"),
        MenuId::Push => catalog.t("setup.menu.push"),
        MenuId::Identity => catalog.t("setup.menu.identity"),
        MenuId::Annotate => catalog.t("setup.menu.annotate"),
        MenuId::AnnotateForm => catalog.t("setup.menu.annotate_form"),
        MenuId::FormFields => catalog.t("setup.menu.form_fields"),
        MenuId::FormField(field_id) => find_field(config, field_id.as_str())
            .map(field_display_label)
            .unwrap_or_else(|| field_id.clone()),
        MenuId::OptionSets => catalog.t("setup.menu.option_sets"),
        MenuId::OptionSet(name) => {
            catalog.tf("setup.menu.option_set_detail", &[("name", name.clone())])
        }
        MenuId::CodeFileTypesCategories => catalog.t("setup.menu.code_file_types"),
        MenuId::CodeFileTypesEntries(category_id) => {
            let name = categories()
                .iter()
                .find(|item| item.id == category_id)
                .map(|category| {
                    localized_label(catalog, category.label_key, category.default_label)
                })
                .unwrap_or_else(|| category_id.clone());
            catalog.tf("setup.menu.code_file_types_detail", &[("name", name)])
        }
        MenuId::Render => catalog.t("setup.menu.render"),
        MenuId::Template(kind) => catalog.t(kind.title_key()),
        MenuId::OldCode => catalog.t("setup.menu.old_code"),
        MenuId::OldCodeLineLayoutPerLine => {
            format!(
                "{} / {}",
                catalog.t("setup.field.annotate.old_code_line_layout"),
                old_code_line_layout_text(catalog, AnnotateOldCodeLineLayout::PerLine)
            )
        }
        MenuId::OldCodeLineLayoutHeaderBody => {
            format!(
                "{} / {}",
                catalog.t("setup.field.annotate.old_code_line_layout"),
                old_code_line_layout_text(catalog, AnnotateOldCodeLineLayout::HeaderBody)
            )
        }
        MenuId::ChoiceUiLang => catalog.t("setup.field.ui.lang"),
        MenuId::ChoiceOldCodeMode => catalog.t("setup.field.annotate.old_code_mode"),
        MenuId::ChoiceOldCodeLineLayout => catalog.t("setup.field.annotate.old_code_line_layout"),
        MenuId::ChoiceFieldKind(field_id) => {
            let field_name = find_field(config, field_id.as_str())
                .map(field_display_label)
                .unwrap_or_else(|| field_id.clone());
            format!(
                "{} / {}",
                field_name,
                catalog.t("setup.field.annotate.field_kind")
            )
        }
    }
}

fn breadcrumb(catalog: &Catalog, config: &AppConfig, state: &SetupState) -> String {
    state
        .stack
        .iter()
        .map(|frame| menu_title(catalog, config, &frame.id))
        .collect::<Vec<_>>()
        .join(" > ")
}

fn active_help(catalog: &Catalog, config: &AppConfig, state: &SetupState) -> SetupHelpContent {
    if state.editor.is_some() {
        SetupHelpContent {
            summary: catalog.t("setup.help.summary.editor"),
            shortcuts: format_shortcuts(
                catalog,
                &[
                    "setup.help.shortcut.enter_save",
                    "setup.help.shortcut.esc_cancel",
                    "setup.help.shortcut.backspace_delete",
                ],
            ),
        }
    } else if state.confirm_exit {
        SetupHelpContent {
            summary: catalog.t("setup.help.summary.confirm_exit"),
            shortcuts: format_shortcuts(
                catalog,
                &[
                    "setup.help.shortcut.up_down_select",
                    "setup.help.shortcut.enter_confirm",
                    "setup.help.shortcut.esc_cancel",
                ],
            ),
        }
    } else {
        let summary = selected_item(catalog, config, state)
            .map(|item| help_summary_for_item(catalog, config, &item))
            .unwrap_or_else(|| catalog.t("setup.help.summary.fallback_default"));
        let shortcuts = selected_item(catalog, config, state)
            .map(|item| shortcuts_for_item(catalog, &item))
            .unwrap_or_else(|| {
                format_shortcuts(
                    catalog,
                    &[
                        "setup.help.shortcut.s_save",
                        "setup.help.shortcut.esc_or_q_exit",
                        "setup.help.shortcut.up_down_select",
                    ],
                )
            });
        SetupHelpContent { summary, shortcuts }
    }
}

fn help_summary_for_item(catalog: &Catalog, config: &AppConfig, item: &MenuItem) -> String {
    let old_code_state = bool_text(catalog, old_code_processing_enabled(config));
    let key = match &item.kind {
        MenuItemKind::Submenu(menu) | MenuItemKind::ToggleSubmenu { menu, .. } => match menu {
            MenuId::General => Some("setup.help.summary.menu.general"),
            MenuId::Identity => Some("setup.help.summary.menu.identity"),
            MenuId::Annotate => Some("setup.help.summary.menu.annotate"),
            MenuId::Features => Some("setup.help.summary.menu.features"),
            MenuId::Push => Some("setup.help.summary.menu.push"),
            MenuId::AnnotateForm => Some("setup.help.summary.menu.annotate_form"),
            MenuId::CodeFileTypesCategories => Some("setup.help.summary.menu.code_file_types"),
            MenuId::Render => Some("setup.help.summary.menu.render"),
            MenuId::Template(TemplateKind::Add) => Some("setup.help.summary.menu.template_add"),
            MenuId::Template(TemplateKind::Modify) => {
                Some("setup.help.summary.menu.template_modify")
            }
            MenuId::Template(TemplateKind::Delete) => {
                Some("setup.help.summary.menu.template_delete")
            }
            MenuId::OldCode => Some("setup.help.summary.menu.old_code"),
            _ => None,
        },
        MenuItemKind::Text(_) => Some("setup.help.summary.item.text"),
        MenuItemKind::Toggle(_) => Some("setup.help.summary.item.toggle"),
        MenuItemKind::Add(_) => Some("setup.help.summary.item.add"),
        MenuItemKind::Action(_) => Some("setup.help.summary.item.action"),
        MenuItemKind::ChoiceOption(_) => Some("setup.help.summary.item.choice"),
        MenuItemKind::SelectSubmenu { .. } => Some("setup.help.summary.item.select_submenu"),
        MenuItemKind::CodeTypeCategory(_) => Some("setup.help.summary.item.code_type_category"),
        MenuItemKind::CodeTypeEntry { .. } => Some("setup.help.summary.item.code_type_entry"),
    };

    if let Some(key) = key {
        if key == "setup.help.summary.menu.template_modify"
            || key == "setup.help.summary.menu.template_delete"
            || key == "setup.help.summary.menu.old_code"
        {
            return catalog.tf(key, &[("old_code_state", old_code_state)]);
        }
        return catalog.t(key);
    }

    let label = item_help_label(catalog, config, item);
    catalog.tf("setup.help.summary.fallback_item", &[("item", label)])
}

fn item_help_label(catalog: &Catalog, config: &AppConfig, item: &MenuItem) -> String {
    match &item.kind {
        MenuItemKind::Submenu(menu) | MenuItemKind::ToggleSubmenu { menu, .. } => {
            menu_title(catalog, config, menu)
        }
        _ => item.text.clone(),
    }
}

fn shortcuts_for_item(catalog: &Catalog, item: &MenuItem) -> String {
    let mut keys = vec![
        "setup.help.shortcut.s_save",
        "setup.help.shortcut.esc_or_q_back_or_exit",
        "setup.help.shortcut.up_down_select",
    ];

    match &item.kind {
        MenuItemKind::Submenu(_) | MenuItemKind::ToggleSubmenu { enabled: true, .. } => {
            keys.push("setup.help.shortcut.enter_open");
        }
        MenuItemKind::ToggleSubmenu { enabled: false, .. } => {
            keys.push("setup.help.shortcut.space_toggle");
        }
        MenuItemKind::Text(_) => {
            keys.push("setup.help.shortcut.enter_edit");
        }
        MenuItemKind::Toggle(_)
        | MenuItemKind::ChoiceOption(_)
        | MenuItemKind::CodeTypeCategory(_)
        | MenuItemKind::CodeTypeEntry { .. } => {
            keys.push("setup.help.shortcut.space_toggle");
        }
        MenuItemKind::Add(_) => {
            keys.push("setup.help.shortcut.enter_add");
        }
        MenuItemKind::Action(_) => {
            keys.push("setup.help.shortcut.enter_apply");
        }
        MenuItemKind::SelectSubmenu { .. } => {
            keys.push("setup.help.shortcut.enter_select_open");
            keys.push("setup.help.shortcut.space_select");
        }
    }

    format_shortcuts(catalog, &keys)
}

fn format_shortcuts(catalog: &Catalog, keys: &[&str]) -> String {
    keys.iter()
        .map(|key| catalog.t(key))
        .collect::<Vec<_>>()
        .join("   ")
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
    use super::{
        active_help, breadcrumb, current_items, handle_key_code, help_summary_for_item,
        setup_layout, toggle_target, EditorState, EditorTarget, MenuId, MenuItem, MenuItemKind,
        SetupState, TemplateKind, ToggleTarget,
    };
    use crate::i18n;
    use crossterm::event::KeyCode;
    use ratatui::layout::Rect;
    use std::path::Path;
    use tempfile::TempDir;

    fn test_catalog() -> crate::i18n::Catalog {
        i18n::load_catalog("en-US", Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    #[test]
    fn root_menu_is_single_column_tree() {
        let catalog = test_catalog();
        let config = crate::config::AppConfig::default();
        let state = SetupState::default();
        let items = current_items(&catalog, &config, &state);
        assert_eq!(items.len(), 3);
        assert!(items[0]
            .text
            .contains(catalog.t("setup.menu.general").as_str()));
        assert!(items[1]
            .text
            .contains(catalog.t("setup.menu.identity").as_str()));
        assert!(items[2]
            .text
            .contains(catalog.t("setup.menu.annotate").as_str()));
    }

    #[test]
    fn enter_submenu_updates_breadcrumb() {
        let catalog = test_catalog();
        let mut config = crate::config::AppConfig::default();
        let mut state = SetupState::default();
        let target = Path::new("/tmp/xgit-setup-test.toml");

        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        assert_eq!(super::current_menu(&state), &MenuId::General);
        let path = breadcrumb(&catalog, &config, &state);
        assert!(path.contains(catalog.t("setup.menu.root").as_str()));
        assert!(path.contains(catalog.t("setup.menu.general").as_str()));
    }

    #[test]
    fn root_menu_help_uses_contextual_summary_and_shortcuts() {
        let catalog = test_catalog();
        let config = crate::config::AppConfig::default();
        let state = SetupState::default();

        let help = active_help(&catalog, &config, &state);
        assert!(help.summary.contains("General settings"));
        assert!(help.shortcuts.contains("[s] Save"));
        assert!(help.shortcuts.contains("[Enter] Open"));
    }

    #[test]
    fn template_and_old_code_help_summaries_include_dependency_matrix() {
        let catalog = test_catalog();
        let mut config = crate::config::AppConfig::default();
        config.annotate.old_code.enabled = false;

        let modify_item = MenuItem {
            text: catalog.t("setup.menu.template_modify"),
            kind: MenuItemKind::Submenu(MenuId::Template(TemplateKind::Modify)),
        };
        let delete_item = MenuItem {
            text: catalog.t("setup.menu.template_delete"),
            kind: MenuItemKind::Submenu(MenuId::Template(TemplateKind::Delete)),
        };
        let old_code_item = MenuItem {
            text: catalog.t("setup.menu.old_code"),
            kind: MenuItemKind::Submenu(MenuId::OldCode),
        };

        let modify_help = help_summary_for_item(&catalog, &config, &modify_item);
        assert!(modify_help.contains("works even when"));
        assert!(modify_help.contains("disabled"));
        assert!(modify_help.contains("old fields"));

        let delete_help = help_summary_for_item(&catalog, &config, &delete_item);
        assert!(delete_help.contains("depends on old-code processing"));
        assert!(delete_help.contains("both delete template and old-code processing"));
        assert!(delete_help.contains("disabled"));

        let old_code_help = help_summary_for_item(&catalog, &config, &old_code_item);
        assert!(old_code_help.contains("only controls old-code display"));
        assert!(old_code_help.contains("does not auto-enable"));
        assert!(old_code_help.contains("disabled"));
    }

    #[test]
    fn editor_and_confirm_state_override_menu_help() {
        let catalog = test_catalog();
        let config = crate::config::AppConfig::default();
        let mut state = SetupState {
            editor: Some(EditorState {
                title: String::from("title"),
                value: String::from("value"),
                target: EditorTarget::IdentityName,
            }),
            ..SetupState::default()
        };
        let editor_help = active_help(&catalog, &config, &state);
        assert!(editor_help.summary.contains("Editing mode"));
        assert!(editor_help.shortcuts.contains("[Enter] Save"));

        state.editor = None;
        state.confirm_exit = true;
        let confirm_help = active_help(&catalog, &config, &state);
        assert!(confirm_help.summary.contains("Exit confirmation"));
        assert!(confirm_help.shortcuts.contains("[Enter] Confirm"));
    }

    #[test]
    fn setup_layout_keeps_two_lines_for_help_content() {
        let chunks = setup_layout(Rect::new(0, 0, 120, 40));
        assert_eq!(chunks[2].height, 4);
    }

    #[test]
    fn esc_returns_from_submenu_instead_of_exiting() {
        let catalog = test_catalog();
        let mut config = crate::config::AppConfig::default();
        let mut state = SetupState::default();
        let target = Path::new("/tmp/xgit-setup-test.toml");

        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        let exit =
            handle_key_code(KeyCode::Esc, &catalog, target, &mut config, &mut state).unwrap();
        assert!(!exit);
        assert_eq!(super::current_menu(&state), &MenuId::Root);
    }

    #[test]
    fn dirty_root_exit_opens_confirm_dialog() {
        let catalog = test_catalog();
        let mut config = crate::config::AppConfig::default();
        let mut state = SetupState::default();
        let target = Path::new("/tmp/xgit-setup-test.toml");

        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Esc, &catalog, target, &mut config, &mut state).unwrap();

        let exit =
            handle_key_code(KeyCode::Esc, &catalog, target, &mut config, &mut state).unwrap();
        assert!(!exit);
        assert!(state.confirm_exit);
    }

    #[test]
    fn code_file_type_navigation_uses_category_and_entry_levels() {
        let catalog = test_catalog();
        let mut config = crate::config::AppConfig::default();
        let mut state = SetupState::default();
        let target = Path::new("/tmp/xgit-setup-test.toml");

        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();

        assert_eq!(
            super::current_menu(&state),
            &MenuId::CodeFileTypesCategories
        );
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        match super::current_menu(&state) {
            MenuId::CodeFileTypesEntries(_) => {}
            other => panic!("expected code file type entry layer, got {other:?}"),
        }
        handle_key_code(
            KeyCode::Char(' '),
            &catalog,
            target,
            &mut config,
            &mut state,
        )
        .unwrap();
        assert!(state.dirty);
        handle_key_code(KeyCode::Esc, &catalog, target, &mut config, &mut state).unwrap();
        assert_eq!(
            super::current_menu(&state),
            &MenuId::CodeFileTypesCategories
        );
    }

    #[test]
    fn annotate_template_requires_space_toggle_before_enter() {
        let catalog = test_catalog();
        let mut config = crate::config::AppConfig::default();
        let mut state = SetupState::default();
        let target = Path::new("/tmp/xgit-setup-test.toml");

        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        assert_eq!(super::current_menu(&state), &MenuId::Annotate);

        for _ in 0..3 {
            handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        }

        assert!(!config.annotate.block_templates.add.enabled);
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        assert_eq!(super::current_menu(&state), &MenuId::Annotate);

        handle_key_code(
            KeyCode::Char(' '),
            &catalog,
            target,
            &mut config,
            &mut state,
        )
        .unwrap();
        assert!(config.annotate.block_templates.add.enabled);
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        assert_eq!(
            super::current_menu(&state),
            &MenuId::Template(super::TemplateKind::Add)
        );
    }

    #[test]
    fn template_toggle_only_changes_enabled_flag() {
        let mut config = crate::config::AppConfig::default();
        config.annotate.block_templates.add.enabled = true;
        config.annotate.block_templates.add.start = "// custom add".to_string();
        config.annotate.block_templates.add.end = "// custom end".to_string();

        toggle_target(
            &mut config,
            ToggleTarget::TemplateEnabled(TemplateKind::Add),
        );
        assert!(!config.annotate.block_templates.add.enabled);
        assert_eq!(config.annotate.block_templates.add.start, "// custom add");
        assert_eq!(config.annotate.block_templates.add.end, "// custom end");

        toggle_target(
            &mut config,
            ToggleTarget::TemplateEnabled(TemplateKind::Add),
        );
        assert!(config.annotate.block_templates.add.enabled);
        assert_eq!(config.annotate.block_templates.add.start, "// custom add");
        assert_eq!(config.annotate.block_templates.add.end, "// custom end");
    }

    #[test]
    fn old_code_toggle_only_changes_enabled_flag() {
        let mut config = crate::config::AppConfig::default();
        config.annotate.old_code.enabled = true;
        config.annotate.old_code.mode = Some(crate::config::AnnotateOldCodeMode::BlockComment);

        toggle_target(&mut config, ToggleTarget::OldCodeEnabled);
        assert!(!config.annotate.old_code.enabled);
        assert_eq!(
            config.annotate.old_code.mode,
            Some(crate::config::AnnotateOldCodeMode::BlockComment)
        );

        toggle_target(&mut config, ToggleTarget::OldCodeEnabled);
        assert!(config.annotate.old_code.enabled);
        assert_eq!(
            config.annotate.old_code.mode,
            Some(crate::config::AnnotateOldCodeMode::BlockComment)
        );
    }

    #[test]
    fn modify_template_value_persists_across_disable_enable_cycle() {
        let catalog = test_catalog();
        let mut config = crate::config::AppConfig::default();
        let mut state = SetupState::default();
        let target = Path::new("/tmp/xgit-setup-test.toml");

        // Root -> Annotate
        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        assert_eq!(super::current_menu(&state), &MenuId::Annotate);

        // Move to "template modify" toggle and enable it.
        for _ in 0..4 {
            handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        }
        handle_key_code(
            KeyCode::Char(' '),
            &catalog,
            target,
            &mut config,
            &mut state,
        )
        .unwrap();
        assert!(config.annotate.block_templates.modify.enabled);

        // Enter template submenu and edit start template.
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        assert_eq!(
            super::current_menu(&state),
            &MenuId::Template(TemplateKind::Modify)
        );
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(
            KeyCode::Char('X'),
            &catalog,
            target,
            &mut config,
            &mut state,
        )
        .unwrap();
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        let expected = config.annotate.block_templates.modify.start.clone();
        assert!(expected.ends_with('X'));

        // Back to annotate and toggle disable -> enable.
        handle_key_code(KeyCode::Esc, &catalog, target, &mut config, &mut state).unwrap();
        assert_eq!(super::current_menu(&state), &MenuId::Annotate);
        handle_key_code(
            KeyCode::Char(' '),
            &catalog,
            target,
            &mut config,
            &mut state,
        )
        .unwrap();
        assert!(!config.annotate.block_templates.modify.enabled);
        handle_key_code(
            KeyCode::Char(' '),
            &catalog,
            target,
            &mut config,
            &mut state,
        )
        .unwrap();
        assert!(config.annotate.block_templates.modify.enabled);

        // Re-open and verify template value was preserved.
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        assert_eq!(
            super::current_menu(&state),
            &MenuId::Template(TemplateKind::Modify)
        );
        assert_eq!(config.annotate.block_templates.modify.start, expected);
    }

    #[test]
    fn old_code_mode_only_has_three_choices() {
        let catalog = test_catalog();
        let mut config = crate::config::AppConfig::default();
        let mut state = SetupState::default();
        let target = Path::new("/tmp/xgit-setup-test.toml");

        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();

        for _ in 0..6 {
            handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        }
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        assert_eq!(super::current_menu(&state), &MenuId::OldCode);

        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        assert_eq!(super::current_menu(&state), &MenuId::ChoiceOldCodeMode);
        let items = current_items(&catalog, &config, &state);
        assert_eq!(items.len(), 3);
        assert!(items[0].text.contains("legacy"));
        assert!(items[1].text.contains("line comment"));
        assert!(items[2].text.contains("block comment"));
    }

    #[test]
    fn old_code_layout_supports_space_select_and_enter_detail() {
        let catalog = test_catalog();
        let mut config = crate::config::AppConfig::default();
        let mut state = SetupState::default();
        let target = Path::new("/tmp/xgit-setup-test.toml");

        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();

        for _ in 0..6 {
            handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        }
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        assert_eq!(super::current_menu(&state), &MenuId::OldCode);

        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        assert_eq!(
            super::current_menu(&state),
            &MenuId::ChoiceOldCodeLineLayout
        );

        let items = current_items(&catalog, &config, &state);
        assert!(items[0].text.contains("[x]"));
        assert!(items[0].text.ends_with("-->"));

        handle_key_code(KeyCode::Down, &catalog, target, &mut config, &mut state).unwrap();
        handle_key_code(
            KeyCode::Char(' '),
            &catalog,
            target,
            &mut config,
            &mut state,
        )
        .unwrap();
        assert_eq!(
            config.annotate.old_code.line_comment.layout,
            crate::config::AnnotateOldCodeLineLayout::HeaderBody
        );
        assert_eq!(
            super::current_menu(&state),
            &MenuId::ChoiceOldCodeLineLayout
        );

        handle_key_code(KeyCode::Enter, &catalog, target, &mut config, &mut state).unwrap();
        assert_eq!(
            super::current_menu(&state),
            &MenuId::OldCodeLineLayoutHeaderBody
        );
        let detail_items = current_items(&catalog, &config, &state);
        assert_eq!(detail_items.len(), 3);
    }

    #[test]
    fn save_from_confirm_writes_config_file() {
        let catalog = test_catalog();
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("config.toml");
        let mut config = crate::config::AppConfig::default();
        let mut state = SetupState::default();

        handle_key_code(
            KeyCode::Enter,
            &catalog,
            target.as_path(),
            &mut config,
            &mut state,
        )
        .unwrap();
        handle_key_code(
            KeyCode::Enter,
            &catalog,
            target.as_path(),
            &mut config,
            &mut state,
        )
        .unwrap();
        handle_key_code(
            KeyCode::Enter,
            &catalog,
            target.as_path(),
            &mut config,
            &mut state,
        )
        .unwrap();
        handle_key_code(
            KeyCode::Esc,
            &catalog,
            target.as_path(),
            &mut config,
            &mut state,
        )
        .unwrap();
        handle_key_code(
            KeyCode::Esc,
            &catalog,
            target.as_path(),
            &mut config,
            &mut state,
        )
        .unwrap();

        let exit = handle_key_code(
            KeyCode::Enter,
            &catalog,
            target.as_path(),
            &mut config,
            &mut state,
        )
        .unwrap();
        assert!(exit);
        let raw = std::fs::read_to_string(target).unwrap();
        assert!(raw.contains("[ui]"));
        assert!(raw.contains("lang = "));
    }
}
