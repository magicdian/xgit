# Rust CLI Runtime

> Grounded from the current `xgit/` Rust codebase and converted Trellis artifacts.

## Snapshot

- Primary language: Rust
- Binary entrypoint: `xgit/src/main.rs`
- Config and data assets: `xgit/config/default.toml`, `xgit/resources/i18n/*.toml`
- Runtime ownership: CLI dispatch, Git command orchestration, annotate rendering, layered config, locale loading, and setup TUI all live inside the Rust crate
- Test coverage shape: unit tests in `main.rs`, `config.rs`, `annotate.rs`, `code_file_types.rs`, and `setup_ui.rs`; CLI integration tests in `xgit/tests/`

## Primary Source Paths

- `xgit/Cargo.toml`
- `xgit/config/default.toml`
- `xgit/src/main.rs`
- `xgit/src/config.rs`
- `xgit/src/annotate.rs`
- `xgit/src/code_file_types.rs`
- `xgit/src/gitutils.rs`
- `xgit/src/i18n.rs`
- `xgit/src/remote.rs`
- `xgit/src/setup_ui.rs`
- `xgit/src/version.rs`
- `xgit/resources/i18n/en-US.toml`
- `xgit/resources/i18n/zh-CN.toml`
- `xgit/tests/remote_branch_ops_cli.rs`
- `xgit/tests/version_cli.rs`

## Runtime Shape

1. `main()` loads layered runtime config with `config::load_runtime_config`, resolves the locale catalog, builds the `clap` command tree, and dispatches subcommands.
2. `main.rs` keeps the top-level command handlers for `push`, `setup`, `annotate`, `reset`, `checkout-remote`, and `completion`.
3. `remote.rs` owns upstream mapping, remote detection, Gerrit detection, and remote-branch candidate lookup used by push/reset/checkout-remote.
4. `annotate.rs` owns staged/latest-commit collection, pending-block restoration, runtime context prompting, renderer selection, block rendering, and output writes.
5. `setup_ui.rs` owns the full-screen terminal editor for `AppConfig` and persists changes through `config::save_config`.

## Module Ownership

### CLI bootstrap and command contracts

- `xgit/src/main.rs`
- `xgit/src/version.rs`
- `xgit/src/gitutils.rs`

Key patterns:

- Help text is built at runtime so disabled features are visible in `--help`, not only at execution time.
- Completion generation and install flows are also defined in `main.rs`, including managed shell profile block replacement.
- Version output is sourced from `env!("CARGO_PKG_VERSION")` through `version::app_version()`.

### Layered config and compatibility normalization

- `xgit/config/default.toml`
- `xgit/src/config.rs`

Key patterns:

- Effective config is merged in this order: built-in defaults, global config, project config, environment overrides, then command-line behavior on top.
- Project config is tied to the Git workspace root through `resolve_git_root()`.
- `config.rs` also carries compatibility normalization for older annotate template and old-code settings, so schema changes belong there first.

### Localization and user-facing strings

- `xgit/src/i18n.rs`
- `xgit/resources/i18n/en-US.toml`
- `xgit/resources/i18n/zh-CN.toml`

Key patterns:

- `Catalog::t()` and `Catalog::tf()` are the shared access points for user-facing text.
- Locale loading first checks runtime file locations, then falls back to embedded resources.
- Locale normalization is intentionally limited to `zh-CN` and `en-US`.

### Remote-aware branch operations

- `xgit/src/remote.rs`
- `xgit/src/main.rs`
- `xgit/tests/remote_branch_ops_cli.rs`

Key patterns:

- `BranchUpstreamMapping` is the shared truth for push/reset branch mapping.
- Push target selection and reset target selection are split into helper functions in `main.rs`, but both depend on the same upstream mapping.
- Remote lookup falls through branch config, upstream tracking, explicit config/env hints, then remote-list heuristics.

### Annotate pipeline

- `xgit/src/annotate.rs`
- `xgit/src/code_file_types.rs`
- `xgit/src/config.rs`

Key patterns:

- Annotate is the largest workflow in the repo and already contains both rendering logic and normalization/rebuild logic for pending blocks.
- Built-in file-type categories are defined in `code_file_types.rs` and converted to persisted `file_rules`.
- Renderer selection, old-code handling, template expansion, and runtime-form prompting all terminate in `annotate.rs`.
- Segment discovery is text-based: `diff_patch_between_contents()` shells out to `git diff --no-index --unified=0` on temporary baseline/current files, then `parse_hunk_segments()` turns the patch into `Add`/`Modify`/`Delete` segments.
- A single large `Modify` segment is expected when the new file no longer shares enough unchanged lines with the baseline, including cases caused by comment-wrapping old code, line-ending churn, broad formatter rewrites, or move/copy-heavy edits without stable anchors.
- When users report that annotate rewrote an entire file for a seemingly small change, debug the incoming diff and segment shape first; the renderer usually reflects the diff it receives rather than inventing additional scope.

### Terminal setup editor

- `xgit/src/setup_ui.rs`
- `xgit/src/config.rs`
- `xgit/resources/i18n/*.toml`

Key patterns:

- The setup UI is a state-machine-driven TUI with menu stack frames, text editors, choice menus, and exit confirmation state.
- It edits `AppConfig` directly and saves the serialized config to either the global or project config path.
- Menu/help labels are localized through the shared catalog rather than hardcoded display strings.

## Validation Seams

- `xgit/src/main.rs` tests cover help rendering, completion generation/install helpers, and push/reset branch-target helpers.
- `xgit/src/config.rs` tests cover merge behavior, compatibility normalization, and config persistence.
- `xgit/src/annotate.rs` tests cover annotate normalization, template handling, old-code rendering, and latest-commit/staged edge cases.
- `xgit/src/code_file_types.rs` tests cover builtin selection/default rules conversion.
- `xgit/src/setup_ui.rs` tests cover menu navigation, toggles, field editing, and choice handling.
- `xgit/tests/remote_branch_ops_cli.rs` exercises real Git repositories for `reset` and `checkout-remote`.
- `xgit/tests/version_cli.rs` locks the `--version` output to Cargo package version.

## Change Triggers

- If you change config schema or defaults, update `xgit/config/default.toml`, `xgit/src/config.rs`, the setup editor in `xgit/src/setup_ui.rs`, and any affected locale strings.
- If you change push/reset/checkout-remote behavior, update `xgit/src/main.rs`, `xgit/src/remote.rs`, and `xgit/tests/remote_branch_ops_cli.rs`.
- If you change annotate rendering or candidate classification, update `xgit/src/annotate.rs`, plus `xgit/src/code_file_types.rs` or `xgit/src/config.rs` when the change touches render configuration or file-rule selection.
- If you add or rename a user-visible command/flag, update `build_runtime_command()` and the locale keys it reads before changing downstream behavior.
