# Development Workflow

> Grounded Trellis workflow bootstrap for converted repository maintenance.

This converted repository keeps historical source specs under `.trellis/legacy/specs/` and imported historical work under `.trellis/tasks/archive/`. It also already carries a usable local Trellis maintenance layer under `.trellis/` plus project-local agent support under `.agents/` and `.codex/`. Start new work from the grounded Trellis docs below instead of treating preserved source artifacts as the active workflow layer.

## Quick Start

1. Read `.trellis/spec/backend/index.md` and `.trellis/spec/guides/index.md` before runtime or conversion-contract changes.
2. Read `.trellis/spec/frontend/index.md` before terminal UI or other interaction-surface changes.
3. Treat `.trellis/tasks/archive/` as imported history; create new work under `.trellis/tasks/`.
4. Treat `.transpec/` and `.codex_bak/` as conversion workspace or backup directories, not as the active maintenance layer.
5. If your team intentionally regenerates imported guides or historical artifacts, verify the grounded Trellis docs and task context files still match the repository state.

## Bootstrap Stages

1. **Current converted baseline**
   - Expect grounded docs, imported history, local `.trellis/scripts/`, and project-local support files to already be present in the current workspace state.
   - Ordinary Trellis-first work in this repo should not require an immediate `trellis update` or `trellis init`.
2. **Optional runtime refresh**
   - Run `trellis update` and `trellis init` only when you intentionally want to refresh or standardize upstream Trellis tooling for this repository.
   - After refreshing, re-verify `.trellis/workflow.md`, the spec indexes, and task context files against the actual repo state.
3. **IDE-specific setup**
   - `.codex/` may already exist as active project-local support; `.codex_bak/` is backup or provenance output, not an active workflow directory.

## Grounded Docs

- Backend/runtime guides live under `.trellis/spec/backend/`.
- Frontend or interaction-surface guides live under `.trellis/spec/frontend/`.
- Cross-layer and repository-state guides live under `.trellis/spec/guides/`.

## Repository Notes

- Primary language detected from repository scan: Rust
- The imported archive is historical context, not an active task queue.
- Local Trellis task tooling is already present under `.trellis/scripts/`.
- Project-local support files under `.agents/` and `.codex/` are part of the active development setup for this repo.
- `.transpec/` and `.codex_bak/` are conversion workspace or backup directories and are not required for routine Trellis-first maintenance.
- Treat the grounded Trellis docs as the default source of truth for ongoing development; consult preserved source artifacts when you need provenance or to refine regenerated guidance.
- Treat `trellis update` and `trellis init` as optional refresh commands for this repo, not as a prerequisite for day-to-day work on the current converted baseline.
