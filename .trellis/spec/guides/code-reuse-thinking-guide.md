# Code Reuse Thinking Guide

> Grounded repository-specific reuse hotspots for ongoing Trellis-first development.

## Reuse Hotspots

### Config loading and persistence

- `xgit/src/config.rs`

Reuse before adding new helpers:

- `load_runtime_config()`
- `merge_layers()`
- `save_config()`
- `resolve_git_root()`

### Remote and upstream mapping

- `xgit/src/remote.rs`

Reuse before adding new branch/remote logic:

- `get_branch_upstream_mapping()`
- `detect_remote_for_branch()`
- `detect_preferred_remote()`
- `list_remote_branch_candidates()`

### CLI contract assembly

- `xgit/src/main.rs`

Reuse before adding new command wiring:

- `build_runtime_command()`
- `resolve_push_target_branch()`
- `resolve_reset_target_ref()`
- completion install helpers and managed profile block helpers

### Annotate config-to-render mapping

- `xgit/src/annotate.rs`
- `xgit/src/code_file_types.rs`

Reuse before adding new annotate behavior:

- `collect_runtime_context()`
- `select_renderer()`
- `active_block_template()`
- `builtin_default_file_rules()`
- `selection_from_file_rules()`
- `file_rules_from_selection()`

### Localization

- `xgit/src/i18n.rs`
- `xgit/resources/i18n/en-US.toml`
- `xgit/resources/i18n/zh-CN.toml`

Reuse before adding new user-facing text:

- `Catalog::t()`
- `Catalog::tf()`
- locale normalization and catalog loading helpers

## Search-First Checklist

- Search `load_runtime_config` before adding another config bootstrap path.
- Search `get_branch_upstream_mapping` before inventing a second upstream parser.
- Search `build_runtime_command` before hardcoding new help text or disabled-feature suffixes.
- Search `selection_from_file_rules` and `file_rules_from_selection` before creating a second file-rule translation path.
- Search `Catalog::t` and `Catalog::tf` before introducing hardcoded CLI or TUI display strings.

## Review Triggers

- If a new command or flag touches both help text and runtime behavior, keep `main.rs` and locale resources aligned in the same change.
- If a new config field is editable in setup, verify the field is represented once in `AppConfig` and reused by both config persistence and UI state.
- If a new remote or annotate helper looks similar to an existing one, prefer extending the existing helper and its tests rather than splitting the logic across modules.
