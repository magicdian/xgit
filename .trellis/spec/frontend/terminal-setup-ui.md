# Terminal Setup UI

> Grounded from the current `xgit/` terminal interaction surfaces.

## Snapshot

- Primary interaction model: terminal CLI with one full-screen TUI surface
- Full-screen surface: `xgit setup`
- Prompt-based surfaces: `xgit annotate` runtime form prompts and `xgit completion --install` confirmation flow
- Localized text source: `xgit/resources/i18n/en-US.toml` and `xgit/resources/i18n/zh-CN.toml`

## Primary Source Paths

- `xgit/src/main.rs`
- `xgit/src/setup_ui.rs`
- `xgit/src/annotate.rs`
- `xgit/src/config.rs`
- `xgit/src/i18n.rs`
- `xgit/resources/i18n/en-US.toml`
- `xgit/resources/i18n/zh-CN.toml`

## Interaction Surfaces

### Runtime CLI help and command descriptions

- `build_runtime_command()` in `xgit/src/main.rs`

Grounded behavior:

- Command help is assembled at runtime from the effective feature flags.
- Disabled features remain visible in help output and gain a localized disabled marker instead of disappearing from the command tree.
- `setup` remains available even when other features are disabled.

### Full-screen setup editor

- `run_setup_ui()` in `xgit/src/setup_ui.rs`

Grounded behavior:

- The UI runs in raw mode on an alternate screen through `crossterm` and `ratatui`.
- Navigation is state-machine driven through `MenuId`, `MenuFrame`, `EditorState`, `ToggleTarget`, `ChoiceOption`, and `SetupState`.
- The editor directly mutates `AppConfig`, tracks dirty state, and persists through `config::save_config()`.
- Help content is contextual and generated from the current selection or editor state instead of being a static footer.

### Annotate runtime prompts

- `collect_runtime_context()` and prompt helpers in `xgit/src/annotate.rs`

Grounded behavior:

- Annotate only prompts when required values are not already supplied via CLI or reusable pending-block context.
- Prompted fields come from structured config in `annotate.form.fields`, not a hardcoded fixed form.
- Reusable context confirmation uses line-based stdin/stdout prompts, not the full-screen TUI.

### Completion install preview and confirmation

- `execute_completion_install()` in `xgit/src/main.rs`

Grounded behavior:

- The install flow prints a preview of the detected shell, temp script path, target script path, and managed profile lines before writing anything.
- Confirmation is a simple `[y/N]` prompt on stdin.
- Profile updates use managed begin/end markers so reinstall replaces the existing block instead of appending duplicates.

## UI State Ownership

### Setup-only state

- `xgit/src/setup_ui.rs`

Ownership notes:

- Add new setup screens by extending the menu/choice/editor state machine, not by adding ad hoc global flags.
- If a config value is editable in setup, its persistence still belongs to `config.rs`; the UI should only manipulate the in-memory `AppConfig`.
- Contextual help and menu text should remain catalog-driven.

### Prompt-based command state

- `xgit/src/annotate.rs`
- `xgit/src/main.rs`

Ownership notes:

- Annotate prompt wording and completion-install prompt wording both belong to the shared localization layer.
- Prompt-based command flows should stay line-oriented and deterministic; they are not secondary TUI modes.

## Validation Focus

- `xgit/src/setup_ui.rs` tests cover navigation, toggles, text editing, choice handling, and help-related state transitions.
- `xgit/src/main.rs` tests cover help rendering, completion generation, completion install helper behavior, and feature-disabled command descriptions.
- When UI behavior crosses into config or command semantics, re-check `xgit/src/config.rs`, `xgit/src/annotate.rs`, and `xgit/src/main.rs` together rather than validating the surface in isolation.

## Change Triggers

- If you add a new setup menu node or editor flow, update `xgit/src/setup_ui.rs`, the relevant locale keys, and the setup UI tests in the same change.
- If you add a new runtime prompt field for annotate, update `xgit/src/config.rs`, `xgit/src/annotate.rs`, and any locale strings that describe the prompt.
- If you change completion install behavior, keep the preview text, confirmation prompt, and managed profile block logic aligned inside `xgit/src/main.rs`.
