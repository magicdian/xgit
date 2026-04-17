# Cross-Layer Thinking Guide

> Grounded repository-specific boundary checks for ongoing Trellis-first development.

## Repository-Specific Boundaries

### Config schema, defaults, and persistence

- `xgit/config/default.toml`
- `xgit/src/config.rs`
- `xgit/src/setup_ui.rs`

When a change crosses this boundary:

- Keep the built-in default config, merge logic, setup editor, and serialization behavior aligned.
- If a field becomes user-editable, make sure setup can read, mutate, and save it without dropping compatibility fields.

### CLI contract, feature flags, and command routing

- `xgit/src/main.rs`
- `xgit/src/version.rs`
- `xgit/src/i18n.rs`
- `xgit/resources/i18n/en-US.toml`
- `xgit/resources/i18n/zh-CN.toml`

When a change crosses this boundary:

- Update `build_runtime_command()` and the localized help/description strings together.
- Preserve the rule that disabled features stay visible in help output.
- If version behavior changes, check `xgit/tests/version_cli.rs`.

### Remote branch operations

- `xgit/src/main.rs`
- `xgit/src/remote.rs`
- `xgit/tests/remote_branch_ops_cli.rs`

When a change crosses this boundary:

- Treat upstream mapping as a shared contract across push, reset, and checkout-remote.
- If remote selection rules or target-branch rules change, update both the command handler and the remote helper layer.
- Keep the real-Git CLI tests aligned with the command semantics.

### Annotate rendering and setup-controlled config

- `xgit/src/annotate.rs`
- `xgit/src/config.rs`
- `xgit/src/code_file_types.rs`
- `xgit/src/setup_ui.rs`
- `xgit/resources/i18n/*.toml`

When a change crosses this boundary:

- If a render behavior is configurable, it usually spans config schema, setup editing, and annotate runtime logic.
- File-rule changes must keep builtin code-type catalogs, persisted rules, and renderer selection in sync.
- Prompt text and validation errors should remain catalog-backed.

### Trellis maintenance layer

- `.trellis/spec/`
- `.trellis/tasks/`
- `.trellis/tasks/archive/`
- `.trellis/legacy/specs/`
- `.trellis/workflow.md`

When a change crosses this boundary:

- Update the grounded Trellis docs when runtime ownership or interaction surfaces materially change.
- Keep new work in `.trellis/tasks/`; treat archived imports and legacy specs as provenance, not the active editing layer.

## Before Implementing Cross-Layer Changes

- Map the full path from input source to side effect: CLI flag or config key, runtime module, user-visible output, tests, and Trellis docs.
- Choose the single source of truth for the behavior before duplicating validation or string formatting in multiple places.
- Verify whether the change also alters localized text, setup editing, or archived task guidance.

## Repo-Specific Checklist

- Config/default change: update `xgit/config/default.toml`, `xgit/src/config.rs`, setup editing in `xgit/src/setup_ui.rs`, and any affected locale text.
- Remote behavior change: update `xgit/src/remote.rs`, callers in `xgit/src/main.rs`, and `xgit/tests/remote_branch_ops_cli.rs`.
- Annotate behavior change: update `xgit/src/annotate.rs`, any relevant config or file-type helpers, and the prompt/help strings that explain the behavior.
- User-visible command/help change: update `build_runtime_command()` plus locale resources before relying on downstream error messages to explain the new behavior.
